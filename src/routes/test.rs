use actix_web::{HttpRequest, Responder, Result, get, web};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    common::{JWT_MANAGE_EXPIRES_SEC, LocalKeySeed},
    middleware::AuthUser,
    utils::jwt::get_token,
};

// use crate::db::mysql_conn;
// use crate::middleware::{save_logs, AuthRole, AuthUser};

// use mysql_quick::{
//     my_run, my_run_drop, my_run_tran, my_run_tran_drop, my_run_vec, mycount, mydel, myfind, myget,
//     myset, mysetmany, myupdate, Queryable, TxOpts, MY_EXCLUSIVE_LOCK,
// };

// use tiberius::{Client, Config, AuthMethod, error::Error};
// use tokio_util::compat::TokioAsyncWriteCompatExt;
// use tokio::net::TcpStream;

#[derive(Serialize, Deserialize, Debug)]
pub struct ForTestItem {
    id: u32,
    title: String,
    content: String,
    price: f64,
    total: i32,
    uid: u16,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ForTestSonItem {
    id: u32,
    t: String,
    cc: String,
    uid: u16,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ForTestSonItemJoin {
    id: u32,
    t: String,
    cc: String,
    uid: u16,
    nickname: String,
    nickname2: String,
}

#[get("/test/mysql")]
pub async fn test_mysql() -> Result<impl Responder> {
    use crate::utils::crypto::{aes_256_decrypt, aes_256_encrypt};

    let plain = "admin55a中国Strin🙂g地".to_string();
    let ct = aes_256_encrypt(&plain, LocalKeySeed::Test)?;
    let ct2 = ct.clone();
    println!("加密结果：{:?}", ct2);

    let pt = aes_256_decrypt(&ct2, LocalKeySeed::Test);
    println!("解密结果：{:?}", pt);

    Ok(web::Json({}))
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct TestJwtToken {
    head: String,
    bearer_token: String,
}
/// 【测试】获取token
#[utoipa::path(
    responses((status = 200, description = "【返回：TestJwtToken】", body = TestJwtToken))
)]
#[get("/test/jwt/token/{uid}")]
pub async fn test_jwt_token(query: web::Path<String>, _req: HttpRequest) -> Result<impl Responder> {
    let uid = query.parse::<u64>().unwrap();
    let token = get_token(AuthUser { id: uid }, JWT_MANAGE_EXPIRES_SEC)?;

    // if let Some(client_ip) = get_client_ip(&req) {
    //     println!("客户端IP: {}", client_ip.ip());
    //     println!("是否通过代理: {}", client_ip.is_behind_proxy());
    //     // ... 其他逻辑
    // }

    Ok(web::Json(TestJwtToken {
        head: "Authorization".to_owned(),
        bearer_token: "Bearer ".to_owned() + &token,
    }))
}
