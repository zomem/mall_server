use actix_web::{Responder, Result, get, post, put, web};
use mysql_quick::{MysqlQuickCount, mycount, myfind, myset, myupdate};
use serde::{Deserialize, Serialize};

use crate::PageData;
use crate::common::types::OssBucket;
use crate::routes::Res;
use crate::utils::files::get_path_from_url;
use crate::utils::filter::deserialize_path_to_url;
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::AuthMana,
};

#[derive(Serialize, Deserialize)]
pub struct ArticleCatAdd {
    id: u32,
    name: String,
    icon: Option<String>,
    sort: Option<i32>,
}
#[post("/manage/article/article_cat/add")]
pub async fn manage_article_article_cat_add(
    _mana: AuthMana,
    params: web::Json<ArticleCatAdd>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let product_name = params.name.trim();
    if product_name.is_empty() {
        return Ok(web::Json(Res::fail("名称不能为空")));
    }
    let temp_img = if let Some(c) = &params.icon {
        get_path_from_url(c, &OssBucket::EobFiles)
    } else {
        "null".to_string()
    };
    let sort = if let Some(s) = params.sort { s } else { 0 };
    let sql;
    if params.id > 0 {
        // 有产品编号，则更新
        sql = myupdate!("art_article_cat", {"id": params.id}, {
            "name": &params.name,
            "icon": &temp_img,
            "sort": sort,
        })
    } else {
        // 新增
        sql = myset!("art_article_cat", {
            "name": &params.name,
            "icon": &temp_img,
            "sort": sort,
        })
    }
    my_run_drop(&mut conn, sql)?;

    Ok(web::Json(Res::success("")))
}

#[derive(Serialize, Deserialize)]
pub struct ArticleCatInfo {
    id: u32,
    name: String,
    icon: String,
    status: i8,
    sort: i32,
    created_at: String,
}
#[get("/manage/article/article_cat/list/{page}/{limit}")]
pub async fn manage_article_article_cat_list(
    _mana: AuthMana,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (page, limit) = query.to_owned();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("art_article_cat", {
            p0: ["is_del", "=", 0],
            r: "p0",
        }),
    )?;

    #[derive(Serialize, Deserialize, Debug)]
    struct ArticleCatGet {
        id: u32,
        name: String,
        #[serde(deserialize_with = "deserialize_path_to_url")]
        icon: String,
        status: i8,
        sort: i32,
        created_at: String,
    }

    let list: Vec<ArticleCatGet> = my_run_vec(
        &mut conn,
        myfind!("art_article_cat", {
            p0: ["is_del", "=", 0],
            r: "p0",
            page: page,
            limit: limit,
            order_by: "-sort,-created_at",
            select: "id,name,sort,icon,created_at,status",
        }),
    )?;

    let list: Vec<ArticleCatInfo> = list
        .into_iter()
        .map(|x| {
            return ArticleCatInfo {
                id: x.id,
                icon: x.icon,
                created_at: x.created_at,
                status: x.status,
                sort: x.sort,
                name: x.name,
            };
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArticleCatStatus {
    id: u32,
    status: i8,
}
#[put("/manage/article/article_cat/status")]
pub async fn manage_article_article_cat_status(
    _mana: AuthMana,
    params: web::Json<ArticleCatStatus>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("art_article_cat", {"id": params.id}, {
            "status": &params.status
        }),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArticleCatDel {
    id: u32,
}
#[put("/manage/article/article_cat/del")]
pub async fn manage_article_article_cat_del(
    _mana: AuthMana,
    params: web::Json<ArticleCatDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("art_article_cat", {"id": params.id}, {"is_del": 1}),
    )?;
    Ok(web::Json(Res::success("成功")))
}
