//! 核销业务逻辑
//!

use actix_web::{Error, error};
use mysql_quick::{Transaction, myfind, myget, myset, myupdate};
use serde::{Deserialize, Serialize};

use crate::common::LocalKeySeed;
use crate::common::types::{DeliveryType, NormalStatus, OrderItemStatus, Role, WriteOffStatus};
use crate::db::{my_run_tran_drop, my_run_tran_vec};
use crate::utils::crypto::aes_256_decrypt;
use crate::utils::time::{NowTimeType, get_now_time, is_expired};

/// 新增待核销产品记录
pub fn add_write_off(tran: &mut Transaction, order_sn: &str) -> Result<(), Error> {
    // 查询当前订单，的物流类型。
    #[derive(Deserialize)]
    struct DeliveryTypeResponse {
        delivery_type: String,
    }
    let deliver: Vec<DeliveryTypeResponse> = my_run_tran_vec(
        tran,
        myget!("ord_order", {"order_sn": order_sn}, "delivery_type"),
    )?;
    if deliver.is_empty() {
        return Ok(()); // 没有，则不处理
    }
    if deliver[0].delivery_type != DeliveryType::StoreWriteOff.to_string() {
        return Ok(()); // 不是门店核销，则不处理
    }

    // 查询，当前订单下面的，所有商品
    #[derive(Deserialize, Serialize)]
    struct OrderItemGet {
        order_sn: String,
        order_item_id: String,
        uid: u64,
        store_code: Option<u64>,
    }
    let item_list: Vec<OrderItemGet> = my_run_tran_vec(
        tran,
        myfind!("ord_order_item", {
            j0: ["unit_sn", "inner", "sku_unit.unit_sn"],
            j1: ["sku_unit.product_sn", "inner", "spu_product.product_sn"],
            p0: ["order_sn", "=", order_sn],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "order_sn, order_item_id, uid, spu_product.store_code",
        }),
    )?;
    for item in item_list {
        if item.store_code.is_none() {
            // 没有门店信息，不处理
            return Err(error::ErrorInternalServerError(format!(
                "产品没有门店信息，下单失败。订单号：{}",
                item.order_sn
            )));
        }
        my_run_tran_drop(
            tran,
            myset!("ord_write_off_item", {
                "uid": item.uid,
                "store_code": item.store_code,
                "order_item_id": item.order_item_id,
            }),
        )?;
    }

    Ok(())
}

/// 核销产品记录
pub fn do_write_off(
    tran: &mut Transaction,
    verification_code: &str,
    wuid: u64,
) -> Result<(), Error> {
    // 通过 verification_code 解出 核销单的 order_item_id,
    // 100,6873808f42d37bd74023000,1752405934
    let data_str = aes_256_decrypt(verification_code, LocalKeySeed::WriteOffCode)?;
    let data = data_str.split(",").collect::<Vec<&str>>();
    if data.len() != 3 {
        return Err(error::ErrorBadRequest("无效的核销码"));
    }
    let user_id = data[0].parse::<u64>().unwrap();
    let order_item_id = data[1];
    let expire_time = data[2].parse::<u64>().unwrap();

    // 判断 码有没有过期，
    if is_expired(expire_time) {
        return Err(error::ErrorBadRequest("核销码已过期"));
    }

    // 查寻
    #[allow(unused)]
    #[derive(Deserialize)]
    struct WriteOffGet {
        uid: u64,
        order_item_id: String,
        store_code: u32,
        expired_time: Option<String>,
        write_off_status: u8,
        is_del: u8,
    }
    let w: Vec<WriteOffGet> = my_run_tran_vec(
        tran,
        myget!("ord_write_off_item", { "order_item_id": order_item_id }, "uid,order_item_id,store_code,write_off_status,expired_time,is_del"),
    )?;
    if w.is_empty() {
        return Err(error::ErrorBadRequest("未找到核销单"));
    }
    if w[0].write_off_status == WriteOffStatus::Cancel as u8 {
        return Err(error::ErrorBadRequest("核销单已取消"));
    }
    if w[0].write_off_status == WriteOffStatus::SuccessWriteOff as u8 {
        return Err(error::ErrorBadRequest("已核销"));
    }
    if w[0].write_off_status == WriteOffStatus::Invalidated as u8 {
        return Err(error::ErrorBadRequest("核销单已作废"));
    }
    if let Some(ex) = w[0].expired_time.clone() {
        if ex < get_now_time(NowTimeType::DateTime) {
            return Err(error::ErrorBadRequest("核销单已过期"));
        }
    }
    if w[0].is_del != 0 {
        return Err(error::ErrorBadRequest("未找到核销单"));
    }
    // 是不是订单里的用户uid
    if w[0].uid != user_id {
        return Err(error::ErrorBadRequest("未找到当前用户的核销单"));
    }

    #[allow(unused)]
    #[derive(Deserialize)]
    struct StoreGet {
        uid: u64,
        role: String,
        com_store_code: u32,
    }
    let list: Vec<StoreGet> = my_run_tran_vec(
        tran,
        myfind!("com_store_employee", {
            j0: ["com_store_code", "inner", "com_store.code"],
            j1: ["uid", "inner", "usr_silent.id"],
            p0: ["status", "=", NormalStatus::Online as i8],
            p1: ["is_del", "=", 0],
            p2: ["uid", "=", wuid],
            r: "p0 && p1 && p2",
            select: "uid, usr_silent.role, com_store_code",
        }),
    )?;

    // 当前用户 wuid 所在的店铺或公司，是不是当前一个 store_code
    let info_data = list.iter().find(|x| x.com_store_code == w[0].store_code);
    if let Some(info) = info_data {
        // 有没有权限 是不是核销员
        let roles = info.role.split(",").collect::<Vec<&str>>();
        let is_write_off = roles.contains(&(Role::WriteOff as u16).to_string().as_str());
        if !is_write_off {
            return Err(error::ErrorForbidden("你不是核销员，暂无核销权限"));
        }
    } else {
        // 判断，是不是员工
        return Err(error::ErrorForbidden("你不是员工，暂无核销权限"));
    }

    // 核销操作
    my_run_tran_drop(
        tran,
        myupdate!("ord_write_off_item", { "order_item_id": order_item_id }, {
            "write_off_uid": wuid,
            "write_off_time": get_now_time(NowTimeType::DateTime),
            "write_off_status": WriteOffStatus::SuccessWriteOff as u8,
        }),
    )?;
    // 同时还要修改，原子订单状态为已完成
    my_run_tran_drop(
        tran,
        myupdate!("ord_order_item", { "order_item_id": order_item_id }, {
            "status": OrderItemStatus::Complete as u8,
        }),
    )?;
    Ok(())
}
