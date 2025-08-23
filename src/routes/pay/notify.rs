use actix_web::{Responder, Result, error, post, web};
use mysql_quick::{TxOpts, myget};
use serde::{Deserialize, Serialize};
use wx_pay::TradeState;
use wx_pay::decode::{WxPayNotify, decode_wx_pay};

use crate::common::WECHAT_PAY_APIV3;
use crate::common::types::{OrderPayStatus, PayType, TranType};
use crate::db::{my_run_tran_vec, mysql_conn};
use crate::routes::Res;
use crate::routes::utils_set::mall_set::{upd_order_status, upd_product_unit_sell_total};
use crate::routes::utils_set::sales_set::do_order_sale_split;
use crate::routes::utils_set::tran_set::add_tran_record;
use crate::routes::utils_set::write_off_item::add_write_off;

/// 微信支付 回调
#[post("/pay/notify")]
pub async fn pay_notify(params: web::Json<WxPayNotify>) -> Result<impl Responder> {
    let params = params.0;
    if params.event_type != "TRANSACTION.SUCCESS".to_string() {
        // 没返回成功
        return Err(error::ErrorInternalServerError("失败"));
    }
    let data = decode_wx_pay(WECHAT_PAY_APIV3, params).unwrap();
    if data.trade_state != TradeState::SUCCESS {
        // 没返回成功
        return Err(error::ErrorInternalServerError("失败"));
    }
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
        total_amount: String,
        reduce_amount: String,
        reduce_des: Option<String>,
        pay_amount: String,
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
        pay_type: Option<String>,
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
    // 如果是充值记录，要另外写
    // 新增订单的交易记录
    let order_json = serde_json::to_value(&order).unwrap();
    match add_tran_record(
        &mut tran,
        TranType::Purchase,
        PayType::WxPay,
        order[0].uid,
        -order[0].pay_amount.parse::<f64>().unwrap(),
        Some(&order_json),
    ) {
        Ok(()) => (),
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    // 进行销售分成处理
    match do_order_sale_split(&mut tran, &order_sn, order[0].uid, PayType::WxPay) {
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
