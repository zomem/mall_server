use actix_web::{Responder, Result, get, post, web};
use mysql_quick::{myfind, myset, myupdate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::routes::Res;
use crate::utils::files::get_file_url;
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::AuthUser,
};

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct AddCollect {
    /// 类别  1是店铺，2是品牌，3是产品，4是商品
    add_type: u8,
    /// 对应的id或编号
    unique_id: u32,
}
/// 【用户】用户添加收藏
#[utoipa::path(
    request_body = AddCollect,
    responses((status = 200, description = "【请求：AddCollect】【返回：String】返回 true 表示收藏，false 表示取消收藏", body = bool))
)]
#[post("/user/collect/add")]
pub async fn user_collect_add(
    user: AuthUser,
    params: web::Json<AddCollect>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let uid = user.id;

    #[derive(Deserialize, Debug)]
    struct CollectId {
        id: u8,
        status: i8,
    }

    let mut is_collect = true;
    let sql;
    if params.add_type == 3 {
        let info: Vec<CollectId> = my_run_vec(
            &mut conn,
            myfind!("usr_collect_product", {
                p0: ["uid", "=", uid],
                p1: ["product_sn", "=", params.unique_id],
                r: "p0 && p1",
                select: "id, status",
            }),
        )?;
        if info.len() > 0 {
            sql = myupdate!("usr_collect_product", info[0].id,{
                "uid": uid,
                "product_sn": params.unique_id,
                "status": if info[0].status == 1 { is_collect=false; 0 } else { 1 },
            });
        } else {
            sql = myset!("usr_collect_product", {
                "uid": uid,
                "product_sn": params.unique_id,
                "status": 1,
            });
        }
    } else {
        return Ok(web::Json(Res::fail("add_type 参数不正确")));
    }
    my_run_drop(&mut conn, sql)?;

    Ok(web::Json(Res::success(is_collect)))
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct CollectRes {
    /// 收藏的id
    id: u64,
    /// 对应的id或编号
    unique_id: u32,
    /// 收藏物品的 名字
    name: String,
    /// 收藏物品的 图片
    cover: Option<String>,
    /// 收藏的时间
    created_at: String,
    /// 收藏类别 1是店铺，2是品牌，3是产品，4是商品
    collect_type: u8,
}
/// 【用户】获取用户收藏
#[utoipa::path(
    responses((status = 200, description = "【返回：CollectRes[]】", body = Vec<CollectRes>)),
    params(("collect_type", description="1是店铺，2是品牌，3是产品，4是商品"),("page", description="页码"))
)]
#[get("/user/collect/list/{collect_type}/{page}")]
pub async fn user_collect_list(
    user: AuthUser,
    query: web::Path<(u8, u32)>,
) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;
    let collect_type = query.0.to_owned();
    let page = query.1.to_owned();

    let sql;
    match collect_type {
        3 => {
            sql = myfind!("usr_collect_product", {
                j0: ["product_sn", "inner", "spu_product.product_sn"],
                p0: ["uid", "=", uid],
                p1: ["status", "=", 1],
                p2: ["is_del", "=", 0],
                r: "p0 && p1 && p2",
                page: page,
                limit: 10,
                select: "id, uid, product_sn as unique_id, spu_product.product_name as name, spu_product.product_cover_img as cover, created_at",
            })
        }
        4 => {
            sql = myfind!("usr_collect_unit", {
                j0: ["unit_sn", "inner", "sku_unit.unit_sn"],
                p0: ["uid", "=", uid],
                p1: ["status", "=", 1],
                p2: ["is_del", "=", 0],
                r: "p0 && p1 && p2",
                page: page,
                limit: 10,
                select: "id, uid, unit_sn as unique_id, sku_unit.unit_name as name, sku_unit.unit_cover as cover, created_at",
            })
        }
        _ => {
            return Ok(web::Json(Res::fail("类型参数错误")));
        }
    }
    #[derive(Deserialize)]
    pub struct CollectGet {
        id: u64,
        unique_id: u32,
        name: String,
        cover: Option<String>,
        created_at: String,
    }
    let list: Vec<CollectGet> = my_run_vec(&mut conn, sql)?;
    let list: Vec<CollectRes> = list
        .into_iter()
        .map(|x| CollectRes {
            id: x.id,
            unique_id: x.unique_id,
            name: x.name,
            cover: get_file_url(x.cover),
            created_at: x.created_at,
            collect_type,
        })
        .collect();

    Ok(web::Json(Res::success(list)))
}
