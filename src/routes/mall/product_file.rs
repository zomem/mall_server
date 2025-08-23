use actix_web::{Responder, Result, error, get, post, web};
use mysql_quick::myfind;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::types::NormalStatus;
use crate::control::email::SendEmail;
use crate::control::frequency::freq_user_day;
use crate::db::{my_run_vec, mysql_conn};
use crate::middleware::AuthUser;
use crate::routes::Res;
use crate::utils::files::get_file_url;
use crate::utils::utils::log_err;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct ProductFile {
    id: u32,
    product_sn: u32,
    title: Option<String>,
    file_url: Option<String>,
}
/// 【产品文件】文件列表
#[utoipa::path(
    responses((status = 200, description = "【返回：ProductFile[]】", body = Res<Vec<ProductFile>>))
)]
#[get("/mall/product_file/{product_sn}")]
pub async fn mall_product_file(_user: AuthUser, path: web::Path<String>) -> Result<impl Responder> {
    let prod_sn = path
        .to_owned()
        .parse::<u32>()
        .map_err(|_| error::ErrorNotFound("访问的内容不存在"))?;
    let mut conn = mysql_conn()?;

    let list: Vec<ProductFile> = my_run_vec(
        &mut conn,
        myfind!("spu_product_file", {
            p0: ["product_sn", "=", prod_sn],
            p1: ["status", "=", NormalStatus::Online as u8],
            p2: ["is_del", "=", 0],
            r: "p0 && p1 && p2",
            order_by: "-created_at",
            select: "id,product_sn,title,file_url,created_at",
        }),
    )?;
    let list = list
        .iter()
        .map(|x| ProductFile {
            id: x.id,
            product_sn: x.product_sn,
            title: x.title.clone(),
            file_url: get_file_url(x.file_url.clone()),
        })
        .collect::<Vec<_>>();

    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct EmailProductFile {
    /// 产品文件id
    id: u32,
    /// 接收邮箱
    email: String,
}
/// 【产品文件】发送邮箱
#[utoipa::path(
    request_body = EmailProductFile,
    responses((status = 200, description = "【请求：EmailProductFile】【返回：String】", body = String)),
)]
#[post("/mall/product_file/send_email")]
pub async fn mall_product_file_send_email(
    user: AuthUser,
    params: web::Json<EmailProductFile>,
) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;

    #[derive(Serialize, Deserialize)]
    struct ProductFileGet {
        title: Option<String>,
        file_url: Option<String>,
    }
    // 查寻当前文件url
    let file: Vec<ProductFileGet> = my_run_vec(
        &mut conn,
        myfind!("spu_product_file", {
            p0: ["id", "=", params.id],
            p1: ["status", "=", NormalStatus::Online as u8],
            p2: ["is_del", "=", 0],
            r: "p0 && p1 && p2",
            select: "title,file_url",
        }),
    )?;
    if file.is_empty() {
        return Err(error::ErrorNotFound("产品文件不存在"));
    }
    let file_url = if let Some(url) = get_file_url(file[0].file_url.clone()) {
        url
    } else {
        return Err(error::ErrorNotFound("产品文件不存在"));
    };

    // 判断频率限制
    freq_user_day(uid, "mall_product_file_send_email", 50)?;

    // 发送邮件
    let send_email = SendEmail::new();
    let name = file[0].title.clone().unwrap_or("产品文件".to_string());
    send_email
        .send(
            &name,
            &format!("{}下载地址：{}", name, &file_url),
            &params.email,
        )
        .map_err(|e| error::ErrorBadGateway(log_err(&e, "发送失败")))?;

    Ok(web::Json(Res::<u8>::info(1, "已发送至邮箱")))
}
