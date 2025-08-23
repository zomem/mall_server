use actix_web::{Responder, Result, error, get, web};
use mysql_quick::{myfind, myupdate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::types::NormalStatus;
use crate::db::{my_run_drop, my_run_vec, mysql_conn};
use crate::middleware::AuthOptionUser;
use crate::routes::Res;
use crate::utils::files::get_file_url;
use crate::utils::html::to_html_image_urls;

#[derive(Serialize, Clone, Debug, ToSchema)]
pub struct Article {
    id: u32,
    /// 标题
    title: String,
    /// 封面图
    cover_img: String,
    /// 浏览量
    views: u32,
    /// 点赞量
    praise: u32,
    /// 创建时间
    created_at: String,
    /// 分类名称
    cat_name: String,
    /// 分类ID
    article_cat_id: u32,
    /// 是否点赞了
    is_praised: bool,
}
/// 【文章】文章列表
#[utoipa::path(
    responses((status = 200, description = "【返回：Article[]】", body = Vec<Article>)),
    params(("page", description="页码"),("category", description="分类ID。传0表示全部分类"))
)]
#[get("/article/content/list/{category}/{page}")]
pub async fn article_content_list(
    user: AuthOptionUser,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let category = query.0.parse::<u32>().unwrap();
    let page = query.1.parse::<u32>().unwrap();
    let mut conn = mysql_conn()?;

    #[derive(Serialize, Deserialize, Debug)]
    struct ArticleGet {
        id: u32,
        title: String,
        cover_img: Option<String>,
        views: u32,
        praise: u32,
        article_cat_id: u32,
        created_at: String,
        cat_name: String,
    }
    let r = if category == 0 {
        "p0 && p1"
    } else {
        "p0 && p1 && p2"
    };

    let list: Vec<ArticleGet> = my_run_vec(
        &mut conn,
        myfind!("art_article", {
            j0: ["article_cat_id", "inner", "art_article_cat.id"],
            p0: ["is_del", "=", 0],
            p1: ["status", "=", NormalStatus::Online as i8],
            p2: ["article_cat_id", "=", category],
            r: r,
            page: page,
            limit: 15,
            order_by: "-sort,-created_at",
            select: "id,title,cover_img,created_at,praise,views,art_article_cat.id as article_cat_id,art_article_cat.name as cat_name",
        }),
    )?;

    // 如果有用户信息，则查寻判断有没有点赞
    let mut praise_list = Vec::new();
    if let Some(uid) = user.id {
        let ids = list
            .iter()
            .map(|x| x.id.to_string())
            .collect::<Vec<String>>()
            .join(",");
        #[derive(Serialize, Deserialize, Debug)]
        struct ArticlePraiseGet {
            id: u32,
            article_id: u32,
        }
        let praise_temp: Vec<ArticlePraiseGet> = my_run_vec(
            &mut conn,
            myfind!("art_article_praise", {
                p0: ["uid", "=", uid],
                p1: ["article_id", "in", ids],
                p2: ["is_praise", "=", 1],
                r: "p0 && p1 && p2",
            }),
        )?;
        praise_list = praise_temp.into_iter().map(|x| x.article_id).collect();
    }

    let list: Vec<Article> = list
        .into_iter()
        .map(|x| {
            return Article {
                id: x.id,
                cover_img: get_file_url(x.cover_img).unwrap_or("".to_string()),
                title: x.title,
                created_at: x.created_at,
                views: x.views,
                praise: x.praise,
                cat_name: x.cat_name,
                article_cat_id: x.article_cat_id,
                is_praised: praise_list.contains(&x.id),
            };
        })
        .collect();
    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Clone, Debug, ToSchema)]
pub struct ArticleDetail {
    id: u32,
    /// 标题
    title: String,
    /// 封面图
    cover_img: String,
    /// 文章内容
    html: String,
    /// 浏览量
    views: u32,
    /// 点赞量
    praise: u32,
    /// 创建时间
    created_at: String,
    /// 分类名称
    cat_name: String,
    /// 分类ID
    article_cat_id: u32,
    /// 是否点赞了
    is_praised: bool,
}
/// 【文章】文章详情
#[utoipa::path(
    responses((status = 200, description = "【返回：ArticleDetail】", body = ArticleDetail)),
    params(("id", description="文章id"))
)]
#[get("/article/content/detail/{id}")]
pub async fn article_content_detail(
    user: AuthOptionUser,
    id: web::Path<String>,
) -> Result<impl Responder> {
    let id = id.parse::<u32>().unwrap();
    let mut conn = mysql_conn()?;

    #[derive(Serialize, Deserialize, Debug)]
    struct ArticleDetailGet {
        id: u32,
        title: String,
        cover_img: Option<String>,
        html: String,
        views: u32,
        praise: u32,
        article_cat_id: u32,
        created_at: String,
        cat_name: String,
    }

    let list: Vec<ArticleDetailGet> = my_run_vec(
        &mut conn,
        myfind!("art_article", {
            j0: ["article_cat_id", "inner", "art_article_cat.id"],
            p0: ["is_del", "=", 0],
            p1: ["status", "=", NormalStatus::Online as i8],
            p2: ["id", "=", id],
            r: "p0 && p1 && p2",
            select: "id,title,cover_img,html,created_at,praise,views,art_article_cat.id as article_cat_id,art_article_cat.name as cat_name",
        }),
    )?;
    if list.is_empty() {
        return Err(error::ErrorNotFound(format!("文章 {}，未找到", id)));
    }

    // 如果有用户信息，则查寻判断有没有点赞
    let mut praise_list = Vec::new();
    if let Some(uid) = user.id {
        let ids = list
            .iter()
            .map(|x| x.id.to_string())
            .collect::<Vec<String>>()
            .join(",");
        #[derive(Serialize, Deserialize, Debug)]
        struct ArticlePraiseGet {
            id: u32,
            article_id: u32,
        }
        let praise_temp: Vec<ArticlePraiseGet> = my_run_vec(
            &mut conn,
            myfind!("art_article_praise", {
                p0: ["uid", "=", uid],
                p1: ["article_id", "in", ids],
                p2: ["is_praise", "=", 1],
                r: "p0 && p1 && p2",
            }),
        )?;
        praise_list = praise_temp.into_iter().map(|x| x.article_id).collect();
    }
    // 浏览量添加 1
    my_run_drop(
        &mut conn,
        myupdate!("art_article", id, { "views": ["incr", 1] }),
    )?;

    let list: Vec<ArticleDetail> = list
        .into_iter()
        .map(|x| {
            return ArticleDetail {
                id: x.id,
                cover_img: get_file_url(x.cover_img).unwrap_or("".to_string()),
                title: x.title,
                created_at: x.created_at,
                html: to_html_image_urls(&x.html),
                views: x.views,
                praise: x.praise,
                cat_name: x.cat_name,
                article_cat_id: x.article_cat_id,
                is_praised: praise_list.contains(&x.id),
            };
        })
        .collect();
    Ok(web::Json(Res::success(list.first().unwrap().clone())))
}
