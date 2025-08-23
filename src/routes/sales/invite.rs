use actix_web::{Responder, Result, error, get, post, put, web};
use mysql_quick::TxOpts;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::LocalKeySeed;
use crate::common::types::Role;
use crate::db::mysql_conn;
use crate::middleware::{AuthRole, AuthUser};
use crate::routes::Res;
use crate::routes::utils_set::sales_set::{
    main_sale_invite_sale, sale_and_main_del, sale_invite_user, user_and_sale_del,
};
use crate::utils::crypto::{aes_256_decrypt, aes_256_encrypt};
use crate::utils::time::{gen_now_expire_time, is_expired};

/// 【分销】邀请销售码
#[utoipa::path(
    responses((status = 200, description = "【返回：String】", body = String)),
)]
#[get("/sales/invite/sale/code")]
pub async fn sales_invite_sale_code(user: AuthRole) -> Result<impl Responder> {
    let uid = user.id;
    if !user.role.contains(&(Role::MainSale as u16)) {
        return Err(error::ErrorUnauthorized("你不是总销售"));
    }
    let code = aes_256_encrypt(
        &format!("{},{}", uid.to_string(), gen_now_expire_time()),
        LocalKeySeed::InviteSaleCode,
    )?;
    Ok(web::Json(Res::success(code)))
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct Invite {
    /// 码
    code: String,
}
/// 【分销】绑定为销售
#[utoipa::path(
    request_body = Invite,
    responses((status = 200, description = "【请求：Invite】【返回：String】", body = String)),
)]
#[post("/sales/invite/sale/bind")]
pub async fn sales_invite_sale_bind(
    user: AuthUser,
    params: web::Json<Invite>,
) -> Result<impl Responder> {
    let uid = user.id;
    let code = params.code.clone();
    let data_str = aes_256_decrypt(&code, LocalKeySeed::InviteSaleCode)?;
    let data = data_str.split(",").collect::<Vec<&str>>();
    if data.len() != 2 {
        return Err(error::ErrorBadRequest("无效的邀请码"));
    }
    let expire_time = data[1].parse::<u64>().unwrap();
    if is_expired(expire_time) {
        return Err(error::ErrorBadRequest("邀请码已过期"));
    }
    let m_uid = data[0]
        .parse::<u64>()
        .map_err(|e| error::ErrorBadRequest(e))?;

    let mut conn = mysql_conn()?;
    // ---- 事务开始 ----
    let mut tran = conn.start_transaction(TxOpts::default()).unwrap();
    match main_sale_invite_sale(&mut tran, m_uid, uid) {
        Ok(_) => {
            tran.commit().unwrap();
            Ok(web::Json(Res::success("操作成功")))
        }
        Err(e) => {
            tran.rollback().unwrap();
            Err(e)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct SaleDelUid {
    uid: u64,
}
/// 【分销】删除销售
#[utoipa::path(
    request_body = SaleDelUid,
    responses((status = 200, description = "【请求：SaleDelUid】【返回：String】", body = String)),
)]
#[put("/sales/invite/sale/del")]
pub async fn sales_invite_sale_del(
    user: AuthRole,
    params: web::Json<SaleDelUid>,
) -> Result<impl Responder> {
    let uid = user.id;
    let sub_uid = params.uid.clone();
    if !user.role.contains(&(Role::MainSale as u16)) {
        return Err(error::ErrorUnauthorized("你不是总销售"));
    }

    let mut conn = mysql_conn()?;
    // ---- 事务开始 ----
    let mut tran = conn.start_transaction(TxOpts::default()).unwrap();
    match sale_and_main_del(&mut tran, uid, sub_uid) {
        Ok(_) => {
            tran.commit().unwrap();
            Ok(web::Json(Res::success("操作成功")))
        }
        Err(e) => {
            tran.rollback().unwrap();
            Err(e)
        }
    }
}

/// 【分销】邀请客户码
#[utoipa::path(
    responses((status = 200, description = "【返回：String】", body = String)),
)]
#[get("/sales/invite/user/code")]
pub async fn sales_invite_user_code(user: AuthRole) -> Result<impl Responder> {
    let uid = user.id;
    if !user.role.contains(&(Role::Sale as u16)) {
        return Err(error::ErrorUnauthorized("你不是销售"));
    }
    let code = aes_256_encrypt(
        &format!("{},{}", uid.to_string(), gen_now_expire_time()),
        LocalKeySeed::InviteUserCode,
    )?;
    Ok(web::Json(Res::success(code)))
}

/// 【分销】绑定为客户
#[utoipa::path(
    request_body = Invite,
    responses((status = 200, description = "【请求：Invite】【返回：String】", body = String)),
)]
#[post("/sales/invite/user/bind")]
pub async fn sales_invite_user_bind(
    user: AuthUser,
    params: web::Json<Invite>,
) -> Result<impl Responder> {
    let uid = user.id;
    let code = params.code.clone();
    let data_str = aes_256_decrypt(&code, LocalKeySeed::InviteUserCode)?;
    let data = data_str.split(",").collect::<Vec<&str>>();
    if data.len() != 2 {
        return Err(error::ErrorBadRequest("无效的邀请码"));
    }
    let expire_time = data[1].parse::<u64>().unwrap();
    if is_expired(expire_time) {
        return Err(error::ErrorBadRequest("邀请码已过期"));
    }
    let s_uid = data[0]
        .parse::<u64>()
        .map_err(|e| error::ErrorBadRequest(e))?;

    let mut conn = mysql_conn()?;
    // ---- 事务开始 ----
    let mut tran = conn.start_transaction(TxOpts::default()).unwrap();
    match sale_invite_user(&mut tran, s_uid, uid) {
        Ok(_) => {
            tran.commit().unwrap();
            Ok(web::Json(Res::success("操作成功")))
        }
        Err(e) => {
            tran.rollback().unwrap();
            Err(e)
        }
    }
}

/// 【分销】删除客户
#[utoipa::path(
    request_body = SaleDelUid,
    responses((status = 200, description = "【请求：SaleDelUid】【返回：String】", body = String)),
)]
#[put("/sales/invite/user/del")]
pub async fn sales_invite_user_del(
    user: AuthRole,
    params: web::Json<SaleDelUid>,
) -> Result<impl Responder> {
    let uid = user.id;
    let sub_uid = params.uid.clone();
    if !user.role.contains(&(Role::Sale as u16)) {
        return Err(error::ErrorUnauthorized("你不是销售"));
    }

    let mut conn = mysql_conn()?;
    // ---- 事务开始 ----
    let mut tran = conn.start_transaction(TxOpts::default()).unwrap();
    match user_and_sale_del(&mut tran, uid, sub_uid) {
        Ok(_) => {
            tran.commit().unwrap();
            Ok(web::Json(Res::success("操作成功")))
        }
        Err(e) => {
            tran.rollback().unwrap();
            Err(e)
        }
    }
}

// TODO 接口权限添加
