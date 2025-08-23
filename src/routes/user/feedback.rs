use actix_web::{Responder, Result, post, web};
use mysql_quick::myset;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::types::OssBucket;
use crate::routes::Res;
use crate::{
    db::{my_run_drop, mysql_conn},
    middleware::AuthUser,
    utils::files::get_path_from_urls,
};

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct Feedback {
    /// 反馈的文本内容
    #[schema(example = "这是一条返回内容")]
    content: String,
    /// 反馈的图片列表
    #[schema(example = "['a.jpg','b.jpg']")]
    imgs: Option<Vec<String>>,
}
/// 【用户】用户反馈
#[utoipa::path(
    request_body = Feedback,
    responses((status = 200, description = "【请求：Feedback】【返回：String】", body = String))
)]
#[post("/user/feedback")]
pub async fn user_feedback(user: AuthUser, params: web::Json<Feedback>) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let uid = user.id;
    let mut img_list = vec![];
    if let Some(images) = &params.imgs {
        img_list = get_path_from_urls(images, &OssBucket::EobFiles);
    }
    my_run_drop(
        &mut conn,
        myset!("usr_feedback", {
            "uid": uid,
            "content": &params.content,
            "images": img_list.join(",")
        }),
    )?;

    Ok(web::Json(Res::<u8>::info(1, "反馈成功")))
}
