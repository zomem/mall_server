use actix_web::{Responder, Result, error, get, web};
use mysql_quick::myfind;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::types::Role;
use crate::db::{my_run_vec, mysql_conn};
use crate::middleware::AuthRole;
use crate::routes::Res;
use crate::utils::files::get_file_url;

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct SaleUserItem {
    id: u32,
    /// 销售(客户)uid
    uid: u64,
    /// 销售(客户)头像
    avatar_url: String,
    /// 销售(客户)名称
    name: String,
    created_at: String,
    status: i8,
}
/// 【分销】总销售的销售
#[utoipa::path(
    responses((status = 200, description = "【返回：SaleUserItem[]】", body = Vec<SaleUserItem>)),
    params(("page", description="页码"),("limit", description="每页数量"))
)]
#[get("/sales/list/sale/{page}/{limit}")]
pub async fn sales_list_sale(
    user: AuthRole,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let uid = user.id;
    if !user.role.contains(&(Role::MainSale as u16)) {
        return Err(error::ErrorUnauthorized("你不是总销售"));
    }
    let page = query.0.parse::<u32>().unwrap();
    let limit = query.1.parse::<u32>().unwrap();
    let mut conn = mysql_conn()?;

    #[derive(Serialize, Deserialize, Debug)]
    struct MainSaleGet {
        id: u32,
        sale_uid: u64,
        sale_avatar_url: Option<String>,
        sale_name: String,
        created_at: String,
        status: i8,
    }
    let list: Vec<MainSaleGet> = my_run_vec(
        &mut conn,
        myfind!("sal_main_sale", {
            j1: ["sale_uid", "inner", "usr_silent.id as u2"],
            p0: ["is_del", "=", 0],
            p1: ["main_sale_uid", "=", uid],
            r: "p0 && p1",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id,sale_uid,u2.avatar_url as sale_avatar_url,u2.nickname as sale_name,status,created_at",
        }),
    )?;

    let list: Vec<SaleUserItem> = list
        .into_iter()
        .map(|x| {
            return SaleUserItem {
                id: x.id,
                created_at: x.created_at,
                status: x.status,
                uid: x.sale_uid,
                avatar_url: get_file_url(x.sale_avatar_url).unwrap_or("".to_string()),
                name: x.sale_name,
            };
        })
        .collect();
    Ok(web::Json(Res::success(list)))
}

/// 【分销】销售的客户
#[utoipa::path(
    responses((status = 200, description = "【返回：SaleUserItem[]】", body = Vec<SaleUserItem>)),
    params(("page", description="页码"),("limit", description="每页数量"))
)]
#[get("/sales/list/user/{page}/{limit}")]
pub async fn sales_list_user(
    user: AuthRole,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let uid = user.id;
    if !user.role.contains(&(Role::Sale as u16)) {
        return Err(error::ErrorUnauthorized("你不是销售"));
    }
    let page = query.0.parse::<u32>().unwrap();
    let limit = query.1.parse::<u32>().unwrap();
    let mut conn = mysql_conn()?;

    #[derive(Serialize, Deserialize, Debug)]
    struct SaleGet {
        id: u32,
        uid: u64,
        avatar_url: Option<String>,
        name: String,
        created_at: String,
        status: i8,
    }
    let list: Vec<SaleGet> = my_run_vec(
        &mut conn,
        myfind!("sal_sale_user", {
            j1: ["uid", "inner", "usr_silent.id as u2"],
            p0: ["is_del", "=", 0],
            p1: ["sale_uid", "=", uid],
            r: "p0 && p1",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id,uid,u2.avatar_url as avatar_url,u2.nickname as name,status,created_at",
        }),
    )?;

    let list: Vec<SaleUserItem> = list
        .into_iter()
        .map(|x| {
            return SaleUserItem {
                id: x.id,
                created_at: x.created_at,
                status: x.status,
                uid: x.uid,
                avatar_url: get_file_url(x.avatar_url).unwrap_or("".to_string()),
                name: x.name,
            };
        })
        .collect();
    Ok(web::Json(Res::success(list)))
}
