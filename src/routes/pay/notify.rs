use actix_web::{Responder, Result, error, post, web};
use mysql_quick::{TxOpts, myfind, myget, myupdate};
use serde::{Deserialize, Serialize};
use serde_aux::prelude::deserialize_number_from_string;
use wx_pay::decode::{WxPayResource, WxRefundResource, WxTransferResource};
use wx_pay::{RefundStatus, TradeState, TransferBillStatus};

use crate::common::types::{
    DeliveryType, OrderItemStatus, OrderPayStatus, PayType, TranType, WithdrawalReqStatus,
    WriteOffStatus,
};
use crate::control::wx_info::get_decode_wx_notify;
use crate::db::{my_run_tran_drop, my_run_tran_vec, mysql_conn};
use crate::routes::Res;
use crate::routes::utils_set::hash_set::{
    hash_user_withdrawal_money, hash_user_withdrawal_money_verify,
};
use crate::routes::utils_set::mall_set::{
    upd_order_item_status, upd_order_item_write_off_status, upd_order_status,
    upd_product_unit_sell_total,
};
use crate::routes::utils_set::sales_set::do_order_sale_split;
use crate::routes::utils_set::tran_set::add_tran_record;
use crate::routes::utils_set::write_off_item::add_write_off;

/// 微信支付 回调
#[post("/pay/notify")]
pub async fn pay_notify(body: web::Bytes, req: actix_web::HttpRequest) -> Result<impl Responder> {
    let data: WxPayResource = get_decode_wx_notify(body, req)?;

    if data.trade_state != TradeState::SUCCESS {
        // 不是成功，则不修改订单状态
        return Ok(web::Json(Res::success("")));
    }

    // ----- 业务逻辑 -----
    let mut conn = mysql_conn()?;
    // #[derive(Deserialize)]
    // struct Uid {
    //     uid: u64,
    // }
    // let res_uid: Vec<Uid> = my_run_vec(
    //     &mut conn,
    //     myget!("usr_silent", {"openid": &data.payer.openid}, "uid"),
    // )?;
    // if res_uid.len() == 0 {
    //     return Err(error::ErrorBadRequest("用户不存在"));
    // }
    // let uid = res_uid[0].uid;

    let order_sn = data.out_trade_no;
    // ---- 事务开始 ----
    let mut tran = conn
        .start_transaction(TxOpts::default())
        .map_err(|_| error::ErrorInternalServerError("事务错误"))?;
    #[derive(Deserialize, Serialize)]
    struct OrderGet {
        order_sn: String,
        uid: u64,
        total_quantity: u32,
        #[serde(deserialize_with = "deserialize_number_from_string")]
        total_amount: f64,
        #[serde(deserialize_with = "deserialize_number_from_string")]
        reduce_amount: f64,
        #[serde(deserialize_with = "deserialize_number_from_string")]
        pay_amount: f64,
        reduce_des: Option<String>,
        notes: Option<String>,
        appointment_time: Option<String>,
        province: Option<String>,
        city: Option<String>,
        area: Option<String>,
        addr_detail: Option<String>,
        contact_user: Option<String>,
        contact_phone: Option<String>,
        info: Option<String>,
        transaction_id: Option<String>,
        pay_type: PayType,
        delivery_type: DeliveryType,
    }
    let order: Vec<OrderGet> =
        match my_run_tran_vec(&mut tran, myget!("ord_order", {"order_sn": &order_sn})) {
            Ok(d) => d,
            Err(e) => {
                tran.rollback().unwrap();
                return Err(e);
            }
        };
    if order.is_empty() {
        tran.rollback().unwrap();
        return Err(error::ErrorNotFound("订单不存在"));
    }
    let uid = order[0].uid;
    // 如果是充值记录，要另外写
    // 新增订单的交易记录
    let order_json = serde_json::to_value(&order).unwrap();
    match add_tran_record(
        &mut tran,
        TranType::Purchase,
        PayType::WxPay,
        uid,
        -order[0].pay_amount,
        Some(&order_json),
    ) {
        Ok(()) => (),
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    // 进行销售分成处理
    match do_order_sale_split(&mut tran, &order_sn, uid, PayType::WxPay) {
        Ok(()) => (),
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    // 修改订单状态 为 已支付
    match upd_order_status(
        &mut tran,
        &order_sn,
        OrderPayStatus::Paid,
        Some(data.transaction_id.clone()),
        None,
    ) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    // 修改商品和产品的销售量计数
    match upd_product_unit_sell_total(&mut tran, &order_sn) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    // 如果有核销类型的产品，那新增待核销记录
    match add_write_off(&mut tran, &order_sn) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };

    tran.commit().unwrap();
    // ---- 事务结束 ----

    Ok(web::Json(Res::success("")))
}

/// 微信转账回调通知
#[post("/pay/transfer/notify")]
pub async fn pay_transfer_notify(
    body: web::Bytes,
    req: actix_web::HttpRequest,
) -> Result<impl Responder> {
    let data: WxTransferResource = get_decode_wx_notify(body, req)?;

    if data.state != TransferBillStatus::SUCCESS {
        // 不是成功，则不修改状态
        return Ok(web::Json(Res::success("")));
    }

    // ----- 业务逻辑 -----
    let mut conn = mysql_conn()?;

    // 从回调数据中获取批次号
    let out_batch_no = data.out_bill_no; // 转账时这里存储的是批次号

    // 开启事务
    let mut tran = conn
        .start_transaction(TxOpts::default())
        .map_err(|_| error::ErrorInternalServerError("事务错误"))?;

    // 根据批次号查找提现申请记录
    #[derive(Deserialize, Serialize)]
    struct WithdrawalRequest {
        id: u64,
        uid: u64,
        #[serde(deserialize_with = "deserialize_number_from_string")]
        req_amount: f64,
        status: u8,
        transfer_hash: String,
    }

    let withdrawal_requests: Vec<WithdrawalRequest> = match my_run_tran_vec(
        &mut tran,
        myget!("usr_withdrawal_request", {"out_bill_no": &out_batch_no}),
    ) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };

    if withdrawal_requests.is_empty() {
        tran.rollback().unwrap();
        return Err(error::ErrorNotFound("找不到对应的提现申请记录"));
    }

    let withdrawal_req = &withdrawal_requests[0];

    // 如果状态不是转账中(5)，
    if withdrawal_req.status != WithdrawalReqStatus::Ing as u8 {
        tran.rollback().unwrap();
        return Err(error::ErrorBadRequest("提现申请状态不正确"));
    }

    let is_pass = hash_user_withdrawal_money_verify(
        &withdrawal_req.transfer_hash,
        withdrawal_req.uid,
        withdrawal_req.req_amount,
        &out_batch_no,
        withdrawal_req.status,
    )?;
    if !is_pass {
        return Err(error::ErrorBadRequest("数据一致性错误，请联系客服"));
    }

    let hash = hash_user_withdrawal_money(
        withdrawal_req.uid,
        withdrawal_req.req_amount,
        &out_batch_no,
        WithdrawalReqStatus::Success as u8,
    )?;
    // 更新提现申请状态为转账成功(3)
    match my_run_tran_drop(
        &mut tran,
        myupdate!("usr_withdrawal_request", withdrawal_req.id, {
            "status": WithdrawalReqStatus::Success as u8,
            "transfer_hash": hash,
        }),
    ) {
        Ok(_) => (),
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };

    // 提交事务
    tran.commit().unwrap();

    Ok(web::Json(Res::success("转账回调处理成功")))
}

/// 微信退款回调通知
#[post("/pay/refund/notify")]
pub async fn pay_refund_notify(
    body: web::Bytes,
    req: actix_web::HttpRequest,
) -> Result<impl Responder> {
    let data: WxRefundResource = get_decode_wx_notify(body, req)?;

    if data.refund_status != RefundStatus::SUCCESS {
        // 不是成功，则不修改状态
        return Ok(web::Json(Res::success("")));
    }

    // ----- 业务逻辑 -----
    let mut conn = mysql_conn()?;
    // 从回调数据中获取退款单号
    let order_sn = data.out_trade_no; // 这里实际存储的是退款单号
    // 开启事务
    let mut tran = conn
        .start_transaction(TxOpts::default())
        .map_err(|_| error::ErrorInternalServerError("事务错误"))?;

    // 通过微信交易号查找对应的主订单
    #[derive(Deserialize, Serialize)]
    struct OrderRefundInfo {
        order_sn: String,
        status: i8,
    }
    // 查找退款中的主订单
    let orders: Vec<OrderRefundInfo> = match my_run_tran_vec(
        &mut tran,
        myfind!("ord_order", {
            p0: ["order_sn", "=", &order_sn],
            p1: ["status", "=", OrderPayStatus::Refunding as i8],
            p2: ["is_del", "=", 0],
            r: "p0 && p1 && p2",
            select: "order_sn, status",
        }),
    ) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };

    if orders.is_empty() {
        tran.rollback().unwrap();
        return Err(error::ErrorNotFound("找不到对应的退款中订单"));
    }

    // 1. 修改主订单状态为已退款
    match upd_order_status(&mut tran, &order_sn, OrderPayStatus::Refund, None, None) {
        Ok(_) => {}
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    }

    // 2. 查询该订单下的所有子订单项
    #[derive(Deserialize, Serialize)]
    struct OrderItemRefund {
        order_item_id: String,
    }

    let order_items: Vec<OrderItemRefund> = match my_run_tran_vec(
        &mut tran,
        myfind!("ord_order_item", {
            p0: ["order_sn", "=", &order_sn],
            p1: ["status", "=", OrderItemStatus::Refunding as i8],
            p2: ["is_del", "=", 0],
            r: "p0 && p1 && p2",
            select: "order_item_id",
        }),
    ) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };

    // 3. 修改核销状态（如果存在核销记录）
    for order_item in &order_items {
        match upd_order_item_write_off_status(
            &mut tran,
            &order_item.order_item_id,
            WriteOffStatus::Invalidated,
        ) {
            Ok(_) => {}
            Err(e) => {
                tran.rollback().unwrap();
                return Err(e);
            }
        };
        // 4. 修改所有子订单项状态为已退货
        match upd_order_item_status(
            &mut tran,
            &order_item.order_item_id,
            OrderItemStatus::Refund,
        ) {
            Ok(_) => {}
            Err(e) => {
                tran.rollback().unwrap();
                return Err(e);
            }
        }
    }

    // 提交事务
    tran.commit().unwrap();

    Ok(web::Json(Res::success("退款回调处理成功")))
}

#[cfg(test)]
mod tests {
    use crate::common::WECHAT_PAY_APIV3;
    use wx_pay::decode::*;
    #[test]
    fn test_decodewx() {
        let params = WxNotify {
            id: "cbd377099bf76e06e2".to_string(),
            create_time: "2025-08-28T16:33:33+08:00".to_string(),
            event_type: "REFUND.SUCCESS".to_string(),
            resource_type: "encrypt-resource".to_string(),
            resource: WxNotifyResource {
                algorithm: "AEAD_AES_256_GCM".to_string(),
                ciphertext: "N6Fo8RTo=".to_string(),
                associated_data: "refund".to_string(),
                original_type: "refund".to_string(),
                nonce: "jAQw6".to_string(),
            },
            summary: "退款成功".to_string(),
        };

        let data: WxRefundResource = decode_wx_notify(WECHAT_PAY_APIV3, params).unwrap();

        println!("数据： {:#?}", data);
    }
}
