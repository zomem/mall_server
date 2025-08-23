use actix_web::{Responder, Result, get, post, put, web};
use mysql_quick::{MysqlQuickCount, mycount, myfind, myset, myupdate};
use serde::{Deserialize, Serialize};

use crate::PageData;
use crate::common::types::OssBucket;
use crate::routes::Res;
use crate::utils::files::{get_file_url, get_path_from_url};
use crate::utils::html::{to_html_image_paths, to_html_image_urls};
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::AuthMana,
};
#[derive(Serialize, Deserialize)]
pub struct ArticleInfo {
    id: u32,
    title: String,
    cover_img: String,
    html: String,
    status: i8,
    sort: i32,
    views: u32,
    praise: u32,
    created_at: String,
    cat_name: String,
    article_cat_id: u32,
}
#[get("/manage/article/article/list/{page}/{limit}")]
pub async fn manage_article_article_list(
    _mana: AuthMana,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (page, limit) = query.to_owned();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("art_article", {
            p0: ["is_del", "=", 0],
            r: "p0",
        }),
    )?;

    #[derive(Serialize, Deserialize, Debug)]
    struct ArticleGet {
        id: u32,
        title: String,
        cover_img: Option<String>,
        html: String,
        status: i8,
        sort: i32,
        views: u32,
        praise: u32,
        article_cat_id: u32,
        created_at: String,
        cat_name: String,
    }

    let list: Vec<ArticleGet> = my_run_vec(
        &mut conn,
        myfind!("art_article", {
            j0: ["article_cat_id", "inner", "art_article_cat.id"],
            p0: ["is_del", "=", 0],
            r: "p0",
            page: page,
            limit: limit,
            order_by: "-sort,-created_at",
            select: "id,title,cover_img,html,sort,praise,views,created_at,status,art_article_cat.id as article_cat_id,art_article_cat.name as cat_name",
        }),
    )?;

    let list: Vec<ArticleInfo> = list
        .into_iter()
        .map(|x| {
            return ArticleInfo {
                id: x.id,
                cover_img: get_file_url(x.cover_img).unwrap_or("".to_string()),
                title: x.title,
                created_at: x.created_at,
                status: x.status,
                html: to_html_image_urls(&x.html),
                sort: x.sort,
                views: x.views,
                praise: x.praise,
                cat_name: x.cat_name,
                article_cat_id: x.article_cat_id,
            };
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize)]
pub struct ArticleAdd {
    id: u32,
    title: String,
    cover_img: String,
    html: String,
    article_cat_id: u32,
    sort: Option<i32>,
}
#[post("/manage/article/article/add")]
pub async fn manage_article_article_add(
    mana: AuthMana,
    params: web::Json<ArticleAdd>,
) -> Result<impl Responder> {
    let uid = mana.id;
    let mut conn = mysql_conn()?;
    let product_name = params.title.trim();
    if product_name.is_empty() {
        return Ok(web::Json(Res::fail("名称不能为空")));
    }
    let sort = if let Some(s) = params.sort { s } else { 0 };
    let sql;
    if params.id > 0 {
        // 有产品编号，则更新
        sql = myupdate!("art_article", {"id": params.id}, {
            "title": &params.title,
            "cover_img": get_path_from_url(&params.cover_img, &OssBucket::EobFiles),
            "html": to_html_image_paths(&params.html),
            "article_cat_id": &params.article_cat_id,
            "sort": sort,
            "uid": uid,
        })
    } else {
        // 新增
        sql = myset!("art_article", {
            "title": &params.title,
            "cover_img": get_path_from_url(&params.cover_img, &OssBucket::EobFiles),
            "html": to_html_image_paths(&params.html),
            "article_cat_id": &params.article_cat_id,
            "sort": sort,
            "uid": uid,
        })
    }
    my_run_drop(&mut conn, sql)?;

    Ok(web::Json(Res::success("")))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArticleStatus {
    id: u32,
    status: i8,
}
#[put("/manage/article/article/status")]
pub async fn manage_article_article_status(
    _mana: AuthMana,
    params: web::Json<ArticleStatus>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("art_article", {"id": params.id}, {
            "status": &params.status
        }),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArticleDel {
    id: u32,
}
#[put("/manage/article/article/del")]
pub async fn manage_article_article_del(
    _mana: AuthMana,
    params: web::Json<ArticleDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("art_article", {"id": params.id}, {"is_del": 1}),
    )?;
    Ok(web::Json(Res::success("成功")))
}
