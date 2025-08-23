use actix_web::{Responder, Result, error, get, post, web};
use mysql_quick::{MY_EXCLUSIVE_LOCK, TxOpts, myfind, myget, myset, myupdate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::types::NormalStatus;
use crate::db::{my_run_tran_drop, my_run_tran_vec, my_run_vec, mysql_conn};
use crate::middleware::AuthUser;
use crate::routes::Res;
use crate::utils::time::{NowTimeType, get_now_time};
use crate::utils::utils::log_err;

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct CouponReceive {
    /// 优惠券id
    coupon_id: u32,
}
/// 【优惠券】领取优惠券
#[utoipa::path(
    request_body = CouponReceive,
    responses((status = 200, description = "【请求：CouponReceive】【返回：String】", body = String)),
)]
#[post("/mall/coupon/receive")]
pub async fn mall_coupon_receive(
    user: AuthUser,
    params: web::Json<CouponReceive>,
) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;
    // ---- 事务开始 ----
    let mut tran = conn.start_transaction(TxOpts::default()).unwrap();
    let sql_cou = myget!(
        "pmt_coupon",
        params.coupon_id,
        "coupon_num,expire_time,status,is_del"
    ) + MY_EXCLUSIVE_LOCK;
    #[derive(Deserialize, Debug)]
    struct CouponGet {
        coupon_num: u32,
        expire_time: Option<String>,
        status: i8,
        is_del: u8,
    }
    let coupon: Vec<CouponGet> = match my_run_tran_vec(&mut tran, sql_cou) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(error::ErrorInternalServerError(log_err(&e, &params)));
        }
    };
    if coupon.len() == 0 {
        tran.rollback().unwrap();
        return Ok(web::Json(Res::fail("优惠券不存在")));
    }
    if coupon[0].is_del == 1 {
        tran.rollback().unwrap();
        return Ok(web::Json(Res::fail("优惠券不存在")));
    }
    if let Some(time) = coupon[0].expire_time.clone() {
        if time <= get_now_time(NowTimeType::DateTime) {
            tran.rollback().unwrap();
            return Ok(web::Json(Res::fail("优惠券已过期")));
        }
    }
    if coupon[0].status != NormalStatus::Online as i8 {
        tran.rollback().unwrap();
        return Ok(web::Json(Res::fail("优惠券已下架")));
    }
    if coupon[0].coupon_num <= 0 {
        tran.rollback().unwrap();
        return Ok(web::Json(Res::fail("优惠券已被领完了")));
    }
    // 查找当前用户，有没有已经领取过同一优惠券
    let sql_usr = myfind!("usr_coupon", {
        p0: ["uid", "=", uid],
        p1: ["coupon_id", "=", params.coupon_id],
        r: "p0 && p1",
    }) + MY_EXCLUSIVE_LOCK;
    let usr_cou: Vec<serde_json::Value> = match my_run_tran_vec(&mut tran, sql_usr) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(error::ErrorInternalServerError(log_err(&e, &params)));
        }
    };
    if usr_cou.len() > 0 {
        // 已经领取过，则不能领了
        tran.rollback().unwrap();
        return Ok(web::Json(Res::fail("你已领取过了")));
    }
    // 用户没有领取过，则领取
    match my_run_tran_drop(
        &mut tran,
        myupdate!("pmt_coupon", params.coupon_id, { "coupon_num": ["incr", -1] }),
    ) {
        Ok(_) => (),
        Err(e) => {
            tran.rollback().unwrap();
            return Err(error::ErrorInternalServerError(log_err(&e, &params)));
        }
    }
    match my_run_tran_drop(
        &mut tran,
        myset!("usr_coupon", { "uid": uid, "coupon_id": params.coupon_id }),
    ) {
        Ok(_) => (),
        Err(e) => {
            tran.rollback().unwrap();
            return Err(error::ErrorInternalServerError(log_err(&e, &params)));
        }
    }
    tran.commit().unwrap();
    // ---- 事务结束 ----

    Ok(web::Json(Res::success("领取成功")))
}

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct CouponRes {
    /// 优化券id
    id: u64,
    /// 优惠券名称
    coupon_name: String,
    /// 优惠券优惠金额
    reduce_amount: Option<f64>,
    /// 优惠券折扣额度
    discount: Option<f64>,
    /// 剩余数量
    coupon_num: u32,
    /// 过期时间
    expire_time: Option<String>,
    /// 优惠券条件id
    coupon_condition_id: u32,
    /// 优惠券条件标题
    coupon_condition_title: String,
}
/// 【优惠券】优惠券列表
#[utoipa::path(
    responses((status = 200, description = "【返回：CouponRes[]】", body = Vec<CouponRes>))
)]
#[get("/mall/coupon/list")]
pub async fn mall_coupon_list() -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    #[derive(Deserialize)]
    pub struct CouponGet {
        id: u64,
        coupon_name: String,
        reduce_amount: Option<String>,
        discount: Option<String>,
        coupon_num: u32,
        expire_time: Option<String>,
        coupon_condition_id: u32,
        coupon_condition_title: String,
    }
    let now_date = get_now_time(NowTimeType::DateTime);
    let sql = myfind!("pmt_coupon", {
        j0: ["coupon_condition_id", "inner", "pmt_coupon_condition.id"],
        p0: ["is_del", "=", 0],
        p1: ["status", "=", NormalStatus::Online as u8],
        p2: ["expire_time", ">", &now_date], // 要找没过期的
        r: "p0 && p1 && p2",
        select: "id, coupon_name, reduce_amount, discount, coupon_num, expire_time, coupon_condition_id, pmt_coupon_condition.title as coupon_condition_title",
    });

    let list: Vec<CouponGet> = my_run_vec(&mut conn, sql)?;

    let list: Vec<CouponRes> = list
        .into_iter()
        .map(|x| {
            return CouponRes {
                id: x.id,
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
                coupon_num: x.coupon_num,
                expire_time: x.expire_time.clone(),
                coupon_condition_id: x.coupon_condition_id,
                coupon_condition_title: x.coupon_condition_title,
            };
        })
        .collect();

    Ok(web::Json(Res::success(list)))
}
