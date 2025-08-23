use actix_web::{get, web, Responder, Result};
use mysql_quick::myfind;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    db::{my_run_vec, mysql_conn},
    middleware::AuthUser,
};

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct UserCouponRes {
    /// 自增id
    id: u64,
    /// 优化券id
    coupon_id: u32,
    /// 优惠券名称
    coupon_name: String,
    /// 优惠券优惠金额
    reduce_amount: Option<f64>,
    /// 优惠券折扣额度
    discount: Option<f64>,
    /// 过期时间
    expire_time: Option<String>,
    /// 优惠券条件标题
    coupon_condition_title: String,
    /// 用户的优惠券状态 1已过期，2未使用，3已使用
    status: u8,
}
/// 【用户】用户优惠券
#[utoipa::path(
    responses((status = 200, description = "【返回：UserCouponRes[]】", body = Vec<UserCouponRes>)),
    params(("status", description="0全部，1已过期，2未使用，3已使用"))
)]
#[get("/user/coupon/list/{status}")]
pub async fn user_coupon_list(user: AuthUser, query: web::Path<u8>) -> Result<impl Responder> {
    let uid = user.id;
    let status = query.to_owned();

    let mut conn = mysql_conn()?;
    #[derive(Deserialize)]
    pub struct CouponGet {
        id: u64,
        coupon_id: u32,
        coupon_name: String,
        reduce_amount: Option<String>,
        discount: Option<String>,
        expire_time: Option<String>,
        coupon_condition_title: String,
        status: u8,
    }
    let sql = myfind!("usr_coupon", {
        j0: ["coupon_id", "inner", "pmt_coupon.id"],
        j1: ["pmt_coupon.coupon_condition_id", "inner", "pmt_coupon_condition.id"],
        p0: ["is_del", "=", 0],
        p1: ["uid", "=", uid],
        p2: ["status", "=", status],
        r: if status == 0 { "p0 && p1" } else { "p0 && p1 && p2" },
       select: "id, coupon_id, pmt_coupon.coupon_name, pmt_coupon.reduce_amount, status, pmt_coupon.discount,pmt_coupon.expire_time,
            pmt_coupon_condition.title as coupon_condition_title",
    });
    let list: Vec<CouponGet> = my_run_vec(&mut conn, sql)?;

    let list: Vec<UserCouponRes> = list
        .into_iter()
        .map(|x| UserCouponRes {
            id: x.id,
            coupon_id: x.coupon_id,
            coupon_name: x.coupon_name,
            reduce_amount: if let Some(d) = x.reduce_amount {
                Some(d.parse::<f64>().unwrap())
            } else {
                None
            },
            discount: if let Some(d) = x.discount {
                Some(d.parse::<f64>().unwrap())
            } else {
                None
            },
            expire_time: x.expire_time.clone(),
            coupon_condition_title: x.coupon_condition_title,
            status: x.status,
        })
        .collect();

    Ok(web::Json(list))
}
