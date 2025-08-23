use actix_web::{Responder, Result, error, post, web};
use mysql_quick::{myfind, myset, myupdate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::types::NormalStatus;
use crate::db::{my_run_drop, my_run_vec, mysql_conn};
use crate::middleware::AuthUser;
use crate::routes::Res;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct ArticleId {
    id: u32,
}
/// 【文章】用户点赞
#[utoipa::path(
    request_body = ArticleId,
    responses((status = 200, description = "【请求：ArticleId】【返回：String】", body = String)),
)]
#[post("/article/stat/praise")]
pub async fn article_stat_praise(
    user: AuthUser,
    params: web::Json<ArticleId>,
) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;

    let list: Vec<serde_json::Value> = my_run_vec(
        &mut conn,
        myfind!("art_article", {
            p0: ["is_del", "=", 0],
            p1: ["status", "=", NormalStatus::Online as i8],
            p2: ["id", "=", params.id],
            r: "p0 && p1 && p2",
            select: "id",
        }),
    )?;
    if list.is_empty() {
        return Err(error::ErrorNotFound(format!("文章 {}，未找到", params.id)));
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct PraiseGet {
        id: u32,
        is_praise: i8,
    }
    let data: Vec<PraiseGet> = my_run_vec(
        &mut conn,
        myfind!("art_article_praise", {
            p0: ["uid", "=", uid],
            p1: ["article_id", "=", params.id],
            r: "p0 && p1",
        }),
    )?;
    if data.is_empty() {
        // 没有记录
        my_run_drop(
            &mut conn,
            myset!("art_article_praise", {
                "uid": uid,
                "article_id": params.id,
                "is_praise": 1
            }),
        )?;
        my_run_drop(
            &mut conn,
            myupdate!("art_article", params.id, {
                "praise": ["incr", 1],
            }),
        )?;
    } else {
        let is_praise = if data[0].is_praise == 1 { 0 } else { 1 };
        my_run_drop(
            &mut conn,
            myupdate!("art_article_praise", {"id": data[0].id}, {
                "is_praise": is_praise
            }),
        )?;
        my_run_drop(
            &mut conn,
            myupdate!("art_article", params.id, {
                "praise": ["incr", if is_praise == 1 { 1 } else { -1 }],
            }),
        )?;
    }

    Ok(web::Json(Res::success("操作成功")))
}
