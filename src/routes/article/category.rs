use actix_web::{Responder, Result, get, web};
use mysql_quick::myfind;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::types::NormalStatus;
use crate::db::{my_run_vec, mysql_conn};
use crate::routes::Res;
use crate::utils::filter::deserialize_path_to_url;

#[derive(Serialize, Clone, Debug, ToSchema)]
pub struct ArticleCat {
    id: u32,
    name: String,
    icon: String,
}
/// 【文章】文章分类
#[utoipa::path(
    responses((status = 200, description = "【返回：ArticleCat[]】", body = Vec<ArticleCat>)),
)]
#[get("/article/category/list")]
pub async fn article_category_list() -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    #[derive(Serialize, Deserialize, Debug)]
    struct ArticleCatGet {
        id: u32,
        name: String,
        status: i8,
        sort: i32,
        created_at: String,
        #[serde(deserialize_with = "deserialize_path_to_url")]
        icon: String,
    }
    let list: Vec<ArticleCatGet> = my_run_vec(
        &mut conn,
        myfind!("art_article_cat", {
            p0: ["is_del", "=", 0],
            p1: ["status", "=", NormalStatus::Online as i8],
            r: "p0 && p1",
            order_by: "-sort,-created_at",
            select: "id,name,sort,icon,created_at,status",
        }),
    )?;

    let list: Vec<ArticleCat> = list
        .into_iter()
        .map(|x| {
            return ArticleCat {
                id: x.id,
                name: x.name,
                icon: x.icon,
            };
        })
        .collect();
    Ok(web::Json(Res::success(list)))
}
