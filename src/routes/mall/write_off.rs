use actix_web::{Responder, Result, error, get, post, web};
use mysql_quick::{TxOpts, myfind};
use serde::{Deserialize, Serialize};
use serde_aux::prelude::{deserialize_number_from_string, deserialize_option_number_from_string};
use utoipa::ToSchema;

use crate::common::LocalKeySeed;
use crate::common::types::WriteOffStatus;
use crate::db::{my_run_vec, mysql_conn};
use crate::middleware::{AuthRole, AuthUser};
use crate::routes::Res;
use crate::routes::utils_set::write_off_item::do_write_off;
use crate::utils::crypto::aes_256_encrypt;
use crate::utils::files::get_file_url;
use crate::utils::qrcode::generate_qrcode;
use crate::utils::time::{NowTimeType, gen_now_expire_time, get_now_time};

#[derive(Serialize, Clone, Debug, ToSchema)]
pub struct WriteOffInfo {
    /// id
    id: u32,
    /// 子订单唯一id
    order_item_id: String,
    /// 商品名
    unit_name: String,
    /// 商品封面
    unit_cover: Option<String>,
    /// 商品标价
    price: f64,
    /// 用户购买的商品数量
    buy_quantity: u32,
    /// 店铺唯一编码
    store_code: u32,
    /// 店铺名
    store_name: String,
    /// 店铺地址
    store_address: String,
    /// 店铺纬度
    store_lat: Option<f64>,
    /// 店铺经度
    store_lng: Option<f64>,
    /// 店铺封面
    store_cover: Option<String>,
    /// 核销二维码图片，base64编码
    verification_qrcode: String,
}
/// 【核销】核销信息
#[utoipa::path(
    responses((status = 200, description = "【返回：WriteOffInfo】", body = WriteOffInfo)),
    params(("order_item_id", description="订单号"))
)]
#[get("/mall/write_off/info/{order_item_id}")]
pub async fn mall_write_off_info(
    user: AuthUser,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let uid = user.id;
    let order_item_id = path.into_inner();
    if order_item_id.is_empty() {
        return Err(error::ErrorBadRequest("订单号不能为空"));
    }

    let mut conn = mysql_conn()?;

    #[derive(Deserialize, Clone, Debug, ToSchema)]
    struct WriteOffInfoGet {
        id: u32,
        order_item_id: String,
        unit_name: String,
        unit_cover: Option<String>,
        #[serde(deserialize_with = "deserialize_number_from_string")]
        price: f64,
        buy_quantity: u32,
        store_code: u32,
        store_name: String,
        store_province: Option<String>,
        store_city: Option<String>,
        store_area: Option<String>,
        store_addr_detail: Option<String>,
        #[serde(deserialize_with = "deserialize_option_number_from_string")]
        store_lat: Option<f64>,
        #[serde(deserialize_with = "deserialize_option_number_from_string")]
        store_lng: Option<f64>,
        store_cover: Option<String>,
        write_off_status: i8,
        expired_time: Option<String>,
    }
    let sql = myfind!("ord_write_off_item", {
        j0: ["order_item_id", "inner", "ord_order_item.order_item_id"],
        j1: ["store_code", "inner", "com_store.code"],
        p0: ["is_del", "=", 0],
        p1: ["order_item_id", "=", order_item_id],
        r: "p0 && p1",
        select: "id, order_item_id, ord_order_item.unit_name, ord_order_item.unit_cover,
            ord_order_item.price, ord_order_item.buy_quantity, store_code,
            com_store.name as store_name, com_store.province as store_province,
            com_store.city as store_city, com_store.area as store_area,
            com_store.addr_detail as store_addr_detail, com_store.lng as store_lng,
            com_store.lat as store_lat, com_store.cover_img as store_cover,
            write_off_status, expired_time",
    });
    let list: Vec<WriteOffInfoGet> = my_run_vec(&mut conn, sql)?;
    if list.is_empty() {
        return Err(error::ErrorBadRequest("未找到相关订单"));
    }
    if list[0].write_off_status == WriteOffStatus::Cancel as i8 {
        return Err(error::ErrorBadRequest("订单已取消"));
    }
    if list[0].write_off_status == WriteOffStatus::SuccessWriteOff as i8 {
        return Err(error::ErrorBadRequest("订单已核销"));
    }
    if list[0].write_off_status == WriteOffStatus::Invalidated as i8 {
        return Err(error::ErrorBadRequest("订单已作废"));
    }
    if let Some(ex) = list[0].expired_time.clone() {
        if ex < get_now_time(NowTimeType::DateTime) {
            return Err(error::ErrorBadRequest("订单已过期"));
        }
    }

    let list = list
        .iter()
        .map(|x| WriteOffInfo {
            id: x.id,
            order_item_id: x.order_item_id.clone(),
            unit_name: x.unit_name.clone(),
            unit_cover: get_file_url(x.unit_cover.clone()),
            price: x.price,
            buy_quantity: x.buy_quantity,
            store_code: x.store_code,
            store_name: x.store_name.clone(),
            store_address: format!(
                "{}{}{}{}",
                x.store_province.clone().unwrap_or_default(),
                x.store_city.clone().unwrap_or_default(),
                x.store_area.clone().unwrap_or_default(),
                x.store_addr_detail.clone().unwrap_or_default()
            ),
            store_lat: x.store_lat,
            store_lng: x.store_lng,
            store_cover: get_file_url(x.store_cover.clone()),
            verification_qrcode: generate_qrcode(
                &(aes_256_encrypt(
                    &format!("{},{},{}", uid, x.order_item_id, gen_now_expire_time()),
                    LocalKeySeed::WriteOffCode,
                )
                .unwrap()),
            )
            .unwrap(),
        })
        .collect::<Vec<_>>();

    Ok(web::Json(Res::success(list[0].clone())))
}

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct DoWriteOff {
    /// 二维码，扫码结果
    pub qr_code_info: String,
}
/// 【核销】核销员核销
#[utoipa::path(
    request_body = DoWriteOff,
    responses((status = 200, description = "【请求：DoWriteOff】【返回：核销成功】", body = String))
)]
#[post("/mall/write_off/do")]
pub async fn mall_write_off_do(
    role: AuthRole,
    params: web::Json<DoWriteOff>,
) -> Result<impl Responder> {
    let r_uid = role.id;
    let mut conn = mysql_conn()?;

    // ---- 事务开始 ----
    let mut tran = conn
        .start_transaction(TxOpts::default())
        .map_err(|e| error::ErrorInternalServerError(e))?;
    match do_write_off(&mut tran, &params.qr_code_info, r_uid) {
        Ok(_) => (),
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    tran.commit().unwrap();
    // ---- 事务结束 ----

    Ok(web::Json(Res::success("核销成功")))
}
