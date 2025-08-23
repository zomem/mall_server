use actix_web::{Responder, Result, get, web};
use mysql_quick::myfind;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::types::NormalStatus;
use crate::db::{my_run_vec, mysql_conn};
use crate::routes::Res;
use crate::utils::files::{get_file_url, get_file_urls};

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct StoreRes {
    /// 店铺自增id
    id: u64,
    /// 店铺编号
    code: u32,
    /// 店铺名
    name: String,
    /// 店铺封面图
    cover_img: Option<String>,
}
/// 【店铺】获取店铺列表
#[utoipa::path(
    responses((status = 200, description = "【返回：StoreRes[]】", body = Vec<StoreRes>)),
    params(("page", description="页码"))
)]
#[get("/mall/store/list/{page}")]
pub async fn mall_store_list(query: web::Path<String>) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let page = query.to_owned().parse::<u32>().unwrap();

    let list: Vec<StoreRes> = my_run_vec(
        &mut conn,
        myfind!("com_store", {
            p0: ["is_del", "=", 0],
            p1: ["status", "=", NormalStatus::Online as u8],
            r: "p0 && p1",
            page: page,
            limit: 8,
            order_by: "-created_at",
            select: "id,code,name,cover_img",
        }),
    )?;
    let list: Vec<StoreRes> = list
        .into_iter()
        .map(|x| StoreRes {
            id: x.id,
            code: x.code,
            name: x.name.clone(),
            cover_img: get_file_url(x.cover_img),
        })
        .collect();

    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct StoreDetailRes {
    /// 店铺自增id
    id: u64,
    /// 店铺编号
    code: u32,
    /// 店铺名
    name: String,
    /// 店铺描述
    des: Option<String>,
    /// 店铺封面图
    cover_img: Option<String>,
    /// 店铺图片列表
    imgs: Vec<String>,
    /// 店铺地址 省
    province: Option<String>,
    /// 店铺地址 市
    city: Option<String>,
    /// 店铺地址 区
    area: Option<String>,
    /// 店铺详细地址
    addr_detail: Option<String>,
    /// 店铺详 坐标 纬度
    lat: Option<f64>,
    /// 店铺详 坐标 经度
    lng: Option<f64>,
    /// 店铺创建时间
    created_at: String,
}
/// 【店铺】获取店铺详情
#[utoipa::path(
    responses((status = 200, description = "【返回：StoreDetailRes】", body = StoreDetailRes)),
    params(("store_code", description="店铺编号"))
)]
#[get("/mall/store/detail/{store_code}")]
pub async fn mall_store_detail(query: web::Path<String>) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let store_code = query.to_owned().parse::<u32>().unwrap();

    #[derive(Deserialize)]
    pub struct StoreDetailGet {
        id: u64,
        code: u32,
        name: String,
        des: Option<String>,
        cover_img: Option<String>,
        imgs: Option<String>,
        province: Option<String>,
        city: Option<String>,
        area: Option<String>,
        addr_detail: Option<String>,
        lat: Option<f64>,
        lng: Option<f64>,
        created_at: String,
    }
    let list: Vec<StoreDetailGet> = my_run_vec(
        &mut conn,
        myfind!("com_store", {
            p0: ["is_del", "=", 0],
            p1: ["status", "=", NormalStatus::Online as u8],
            p2: ["code", "=", store_code],
            r: "p0 && p1 && p2",
            select: "id,code,name,des,cover_img,imgs,province,city,area,addr_detail,lat,lng,created_at",
        }),
    )?;

    if list.len() == 0 {
        return Ok(web::Json(Res::fail("店铺不存在")));
    }

    let list: Vec<StoreDetailRes> = list
        .into_iter()
        .map(|x| StoreDetailRes {
            id: x.id,
            code: x.code,
            name: x.name.clone(),
            des: x.des.clone(),
            cover_img: get_file_url(x.cover_img),
            imgs: get_file_urls(x.imgs),
            province: x.province.clone(),
            city: x.city.clone(),
            area: x.area.clone(),
            addr_detail: x.addr_detail.clone(),
            lat: x.lat.clone(),
            lng: x.lng.clone(),
            created_at: x.created_at.clone(),
        })
        .collect();

    Ok(web::Json(Res::success(list)))
}
