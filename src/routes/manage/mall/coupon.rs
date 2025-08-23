use actix_web::{Responder, Result, get, post, put, web};
use mysql_quick::{MysqlQuickCount, mycount, myfind, myset, myupdate};
use serde::{Deserialize, Serialize};

use crate::PageData;
use crate::routes::Res;
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::AuthMana,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct CouponConditionAdd {
    id: Option<u32>,
    full_amount: Option<f64>,
    product_brand: Option<u32>,
    product_cat: Option<String>,
    product_sn: Option<u32>,
    store_code: Option<u32>,
    title: String,
    unit_sn: Option<u32>,
}
/// 优惠券条件新增
#[post("/manage/mall/coupon/condition/add")]
pub async fn manage_mall_coupon_condition_add(
    _mana: AuthMana,
    params: web::Json<CouponConditionAdd>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let title = params.title.trim();
    if title.is_empty() {
        return Ok(web::Json(Res::fail("标题不能为空")));
    }
    let sql;
    if let Some(id) = params.id {
        // 更新
        sql = myupdate!("pmt_coupon_condition", id, {
            "title": title,
            "full_amount": params.full_amount,
            "store_code": params.store_code,
            "brand_code": params.product_brand,
            "product_cat": &params.product_cat,
            "product_sn": params.product_sn,
            "unit_sn": params.unit_sn,
        });
    } else {
        // 新增
        sql = myset!("pmt_coupon_condition", {
            "title": title,
            "full_amount": params.full_amount,
            "store_code": params.store_code,
            "brand_code": params.product_brand,
            "product_cat": &params.product_cat,
            "product_sn": params.product_sn,
            "unit_sn": params.unit_sn,
        });
    }
    my_run_drop(&mut conn, sql)?;

    Ok(web::Json(Res::success("")))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CouponConditionRes {
    id: u32,
    title: String,
    full_amount: Option<f64>,
    product_brand: Option<u32>,
    product_cat: Option<Vec<u32>>,
    product_sn: Option<u32>,
    store_code: Option<u32>,
    unit_sn: Option<u32>,
    created_at: String,
}
/// 优惠券列表
#[get("/manage/mall/coupon/condition/list/{page}/{limit}")]
pub async fn manage_mall_coupon_condition_list(
    _mana: AuthMana,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (page, limit) = query.to_owned();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("pmt_coupon_condition", {
            p0: ["is_del", "=", 0],
            r: "p0",
        }),
    )?;

    #[derive(Debug, Deserialize, Serialize)]
    pub struct CouponConditionGet {
        id: u32,
        title: String,
        full_amount: Option<String>,
        brand_code: Option<u32>,
        product_cat: Option<String>,
        product_sn: Option<u32>,
        store_code: Option<u32>,
        unit_sn: Option<u32>,
        created_at: String,
    }
    let list: Vec<CouponConditionGet> = my_run_vec(
        &mut conn,
        myfind!("pmt_coupon_condition", {
            p0: ["is_del", "=", 0],
            r: "p0",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id,title,full_amount,store_code,brand_code,product_cat,product_sn,unit_sn,created_at",
        }),
    )?;
    let list: Vec<CouponConditionRes> = list
        .into_iter()
        .map(|x| CouponConditionRes {
            id: x.id,
            title: x.title,
            full_amount: if let Some(f) = x.full_amount {
                Some(f.parse::<f64>().unwrap())
            } else {
                None
            },
            product_brand: x.brand_code,
            product_cat: if let Some(p) = x.product_cat {
                let d = p.split(",").collect::<Vec<&str>>();
                let cat = d.into_iter().map(|x| x.parse::<u32>().unwrap()).collect();
                Some(cat)
            } else {
                None
            },
            product_sn: x.product_sn,
            store_code: x.store_code,
            unit_sn: x.unit_sn,
            created_at: x.created_at,
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize)]
pub struct CouponConditionSearchInfo {
    label: String,
    value: u32,
}
/// 条件搜索页面
#[get("/manage/mall/coupon/condition/search/{keyword}")]
pub async fn manage_mall_coupon_condition_search(
    _mana: AuthMana,
    query: web::Path<String>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let keyword = query.to_owned();
    #[derive(Deserialize)]
    struct ConditionGet {
        id: u32,
        title: String,
    }
    let list: Vec<ConditionGet> = my_run_vec(
        &mut conn,
        myfind!("pmt_coupon_condition", {
            p0: ["title", "like", format!("%{keyword}%")],
            p1: ["is_del", "=", 0],
            p2: ["id", "=", &keyword],
            r: "(p0 || p2) && p1",
        }),
    )?;
    let list: Vec<CouponConditionSearchInfo> = list
        .into_iter()
        .map(|x| CouponConditionSearchInfo {
            label: x.title,
            value: x.id,
        })
        .collect();
    Ok(web::Json(Res::success(list)))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CouponAdd {
    id: Option<u32>,
    coupon_name: String,
    coupon_condition_id: u32,
    reduce_amount: Option<f64>,
    discount: Option<f64>,
    coupon_num: u32,
    expire_time: Option<String>,
}
/// 优惠券条件新增
#[post("/manage/mall/coupon/add")]
pub async fn manage_mall_coupon_add(
    _mana: AuthMana,
    params: web::Json<CouponAdd>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    let sql;
    if let Some(id) = params.id {
        // 更新
        sql = myupdate!("pmt_coupon", id, {
            "coupon_name": &params.coupon_name,
            "coupon_condition_id": params.coupon_condition_id,
            "reduce_amount": params.reduce_amount,
            "discount": params.discount,
            "coupon_num": params.coupon_num,
            "expire_time": &params.expire_time,
        });
    } else {
        // 新增
        sql = myset!("pmt_coupon", {
            "coupon_name": &params.coupon_name,
            "coupon_condition_id": params.coupon_condition_id,
            "reduce_amount": params.reduce_amount,
            "discount": params.discount,
            "coupon_num": params.coupon_num,
            "expire_time": &params.expire_time,
        });
    }
    my_run_drop(&mut conn, sql)?;

    Ok(web::Json(Res::success("")))
}

#[derive(Debug, Deserialize, Serialize)]
struct CouponRes {
    id: u32,
    coupon_name: String,
    coupon_condition_id: u32,
    coupon_condition_name: String,
    reduce_amount: Option<f64>,
    discount: Option<f64>,
    coupon_num: u32,
    expire_time: Option<String>,
    status: i8,
    created_at: String,
}
/// 优惠券条件新增
#[get("/manage/mall/coupon/list/{page}/{limit}")]
pub async fn manage_mall_coupon_list(
    _mana: AuthMana,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (page, limit) = query.to_owned();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("pmt_coupon", {
            p0: ["is_del", "=", 0],
            r: "p0",
        }),
    )?;

    #[derive(Debug, Deserialize, Serialize)]
    pub struct CouponResGet {
        id: u32,
        coupon_name: String,
        coupon_condition_id: u32,
        coupon_condition_name: String,
        reduce_amount: Option<String>,
        discount: Option<String>,
        coupon_num: u32,
        expire_time: Option<String>,
        status: i8,
        created_at: String,
    }
    let list: Vec<CouponResGet> = my_run_vec(
        &mut conn,
        myfind!("pmt_coupon", {
            j0: ["coupon_condition_id", "inner", "pmt_coupon_condition.id"],
            p0: ["is_del", "=", 0],
            r: "p0",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id,coupon_name,coupon_condition_id,pmt_coupon_condition.title as coupon_condition_name, reduce_amount,discount,coupon_num,expire_time,status,created_at",
        }),
    )?;

    let list: Vec<CouponRes> = list
        .into_iter()
        .map(|x| CouponRes {
            id: x.id,
            coupon_name: x.coupon_name,
            coupon_condition_id: x.coupon_condition_id,
            coupon_condition_name: x.coupon_condition_name,
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
            expire_time: x.expire_time,
            status: x.status,
            created_at: x.created_at,
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CouponDel {
    id: u32,
}
/// 删除
#[put("/manage/mall/coupon/del")]
pub async fn manage_mall_coupon_del(
    _mana: AuthMana,
    params: web::Json<CouponDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("pmt_coupon", {"id": params.id}, {"is_del": 1}),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CouponStatus {
    id: u32,
    status: i8,
}
/// 修改状态
#[put("/manage/mall/coupon/status")]
pub async fn manage_mall_coupon_status(
    _mana: AuthMana,
    params: web::Json<CouponStatus>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("pmt_coupon", {"id": params.id}, {
            "status": &params.status
        }),
    )?;
    Ok(web::Json(Res::success("成功")))
}
