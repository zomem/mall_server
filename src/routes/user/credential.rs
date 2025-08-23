use actix_web::{Responder, Result, get, post, web};
use mysql_quick::{MysqlQuickCount, mycount, myfind, myset};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::types::{NormalStatus, OssBucket};
use crate::db::my_run_vec;
use crate::routes::Res;
use crate::{
    db::{my_run_drop, mysql_conn},
    middleware::AuthUser,
    utils::files::get_path_from_urls,
};

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct UserAddCredential {
    /// 认证标题
    title: String,
    /// 认证内容
    content: Option<String>,
    /// 认证资质图片
    imgs: Option<Vec<String>>,
    /// 认证的角色编号
    role: String,
}
/// 【用户】用户角色认证
#[utoipa::path(
    request_body = UserAddCredential,
    responses((status = 200, description = "【请求：UserAddCredential】【返回：String】", body = String))
)]
#[post("/user/credential/add")]
pub async fn user_credential_add(
    user: AuthUser,
    params: web::Json<UserAddCredential>,
) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;

    // 查看，当前用户，是不是已经有相同角色的认证了
    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("usr_credential", {
            p0: ["uid", "=", uid],
            p1: ["role", "=", &params.role],
            p2: ["is_del", "=", 0],
            r: "p0 && p1 && p2",
        }),
    )?;
    if count[0].mysql_quick_count > 0 {
        return Ok(web::Json(Res::fail("你已申请过了")));
    }

    #[derive(Serialize, Deserialize)]
    pub struct UserAddCredentialSet {
        title: String,
        content: Option<String>,
        imgs: Vec<String>,
    }
    my_run_drop(
        &mut conn,
        myset!("usr_credential", {
            "uid": uid,
            "title": &params.title,
            "content": &params.content,
            "imgs": if let Some(im) = &params.imgs {
                Some(get_path_from_urls(im, &OssBucket::EobFiles).join(","))
            } else {
                None
            },
            "role": &params.role,
            "status": NormalStatus::UnderReview as u8,
        }),
    )?;

    Ok(web::Json(Res::success("")))
}

#[derive(Serialize, Deserialize, Debug, ToSchema, Clone)]
pub struct CredentialRes {
    /// 自增id
    id: u32,
    /// 认证标题
    title: String,
    /// 认证内容
    content: Option<String>,
    /// 认证资质图片
    imgs: Option<Vec<String>>,
    /// 认证的角色编号
    role: String,
    /// 审核结果
    reason: Option<String>,
    /// 状态 2正常通过，1审核中，0审核未通过，3撤销
    status: u8,
    /// 创建时间
    created_at: String,
}
/// 【用户】用户认证详情
#[utoipa::path(
    responses((status = 200, description = "【返回：CredentialRes】", body = CredentialRes)),
    params(("role", description="角色编号"))
)]
#[get("/user/credential/detail/{role}")]
pub async fn user_credential_detail(
    user: AuthUser,
    query: web::Path<String>,
) -> Result<impl Responder> {
    let uid = user.id;
    let role = query.to_owned();
    let mut conn = mysql_conn()?;

    let list: Vec<CredentialRes> = my_run_vec(
        &mut conn,
        myfind!("usr_credential", {
            p0: ["uid", "=", uid],
            p1: ["role", "=", &role],
            p2: ["is_del", "=", 0],
            r: "p0 && p1 && p2",
        }),
    )?;
    if list.len() == 0 {
        return Ok(web::Json(Res::fail("还未申请过")));
    }

    Ok(web::Json(Res::success(list[0].clone())))
}
