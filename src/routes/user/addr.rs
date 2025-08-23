use actix_web::{Responder, Result, get, post, put, web};
use mysql_quick::{MysqlQuickCount, mycount, myfind, myset, myupdate, myupdatemany};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::routes::Res;
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::AuthUser,
};

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct UserAddress {
    /// 地址信息id,有id则为更新，没有id或id=0，则为新增
    id: Option<u64>,
    /// 省
    province: String,
    /// 市
    city: String,
    /// 区
    area: String,
    /// 详细地址
    addr_detail: String,
    /// 联系人
    contact_user: String,
    /// 联系手机
    contact_phone: String,
    /// 是否为默认地址，1为默认地址，0为非默认地址。不传该字段，则也表示0
    is_default: Option<u8>,
}
/// 【用户】用户添加地址
#[utoipa::path(
    request_body = UserAddress,
    responses((status = 200, description = "【请求：UserAddAddr】【返回：String】", body = String))
)]
#[post("/user/addr/add")]
pub async fn user_addr_add(
    user: AuthUser,
    params: web::Json<UserAddress>,
) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;
    // 查寻用户，有没有 default 的地址
    #[derive(Serialize, Deserialize)]
    struct AddrGet {
        id: u64,
        is_default: u8,
    }
    let addr_info: Vec<AddrGet> = my_run_vec(
        &mut conn,
        myfind!("usr_address", {
            p0: ["uid", "=", uid],
            p1: ["is_del", "=", 0],
            p2: ["is_default", "=", 1],
            r: "p0 && p1 && p2",
            select: "id,is_default",
        }),
    )?;

    let mut sql_default = String::new();
    let mut default_value = 0;
    if let Some(p) = params.is_default {
        if p == 1 {
            if addr_info.len() > 0 {
                let addr_upd = addr_info
                    .iter()
                    .map(|x| AddrGet {
                        id: x.id,
                        is_default: 0,
                    })
                    .collect::<Vec<AddrGet>>();
                sql_default = myupdatemany!("usr_address", "id", addr_upd);
            }
            default_value = 1;
        } else {
            default_value = 0;
        }
    }

    let sql;
    if let Some(id) = params.id {
        if id == 0 {
            // 新增
            sql = myset!("usr_address", {
                "uid": uid,
                "province": &params.province,
                "city": &params.city,
                "area": &params.area,
                "addr_detail": &params.addr_detail,
                "contact_user": &params.contact_user,
                "contact_phone": &params.contact_phone,
                "is_default": default_value,
            });
        } else {
            // 更新
            sql = myupdate!("usr_address", id, {
                "province": &params.province,
                "city": &params.city,
                "area": &params.area,
                "addr_detail": &params.addr_detail,
                "contact_user": &params.contact_user,
                "contact_phone": &params.contact_phone,
                "is_default": default_value,
            });
        }
    } else {
        // 新增
        sql = myset!("usr_address", {
            "uid": uid,
            "province": &params.province,
            "city": &params.city,
            "area": &params.area,
            "addr_detail": &params.addr_detail,
            "contact_user": &params.contact_user,
            "contact_phone": &params.contact_phone,
            "is_default": default_value,
        });
    }

    if !sql_default.is_empty() {
        // 将其他 default 的改为 0
        my_run_drop(&mut conn, sql_default)?;
    }
    my_run_drop(&mut conn, sql)?;

    Ok(web::Json(Res::success("")))
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct UserAddressId {
    /// 地址信息id
    id: u64,
}
/// 【用户】用户删除地址
#[utoipa::path(
    request_body = UserAddressId,
    responses((status = 200, description = "【请求：UserAddressId】【返回：String】", body = String))
)]
#[put("/user/addr/del")]
pub async fn user_addr_del(
    user: AuthUser,
    params: web::Json<UserAddressId>,
) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;

    let c_info: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("usr_address", {
            p0: ["uid", "=", uid],
            p1: ["id", "=", params.id],
            r: "p0 && p1",
        }),
    )?;

    if c_info[0].mysql_quick_count == 0 {
        return Ok(web::Json(Res::fail("地址信息不存在")));
    }
    my_run_drop(
        &mut conn,
        myupdate!("usr_address", params.id, {
            "is_del": 1,
        }),
    )?;

    Ok(web::Json(Res::success("删除成功")))
}

/// 【用户】用户地址列表
#[utoipa::path(
    responses((status = 200, description = "【返回：UserAddress[]】", body = Vec<UserAddress>)),
)]
#[get("/user/addr/list")]
pub async fn user_addr_list(user: AuthUser) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;

    let addr_info: Vec<UserAddress> = my_run_vec(
        &mut conn,
        myfind!("usr_address", {
            p0: ["uid", "=", uid],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "id,province,city,area,addr_detail,contact_user,contact_phone,is_default",
        }),
    )?;

    Ok(web::Json(Res::success(addr_info)))
}

/// 【用户】用户地址详情
#[utoipa::path(
    responses((status = 200, description = "【返回：UserAddress】", body = UserAddress)),
)]
#[get("/user/addr/detail/{id}")]
pub async fn user_addr_detail(user: AuthUser, query: web::Path<String>) -> Result<impl Responder> {
    let uid = user.id;
    let id = query.to_owned().parse::<u64>().unwrap();
    let mut conn = mysql_conn()?;

    let addr_info: Vec<UserAddress> = my_run_vec(
        &mut conn,
        myfind!("usr_address", {
            p0: ["uid", "=", uid],
            p1: ["is_del", "=", 0],
            p2: ["id", "=", id],
            r: "p0 && p1 && p2",
            select: "id,province,city,area,addr_detail,contact_user,contact_phone,is_default",
        }),
    )?;

    if addr_info.len() == 0 {
        return Ok(web::Json(Res::fail("地址信息不存在")));
    }

    Ok(web::Json(Res::success(addr_info)))
}
