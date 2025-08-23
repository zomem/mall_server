use actix_web::{Error, error};
use mysql_quick::{Queryable, myget, myupdate};
use serde::Deserialize;

use crate::common::JWT_NORMAL_EXPIRES_SEC;
use crate::db::my_run_drop;
use crate::middleware::AuthUser;
use crate::routes::utils_set::pocket_set::init_user_pocket_money;
use crate::utils::files::get_file_url_sec;
use crate::utils::jwt::get_token;
use crate::utils::utils::log_err;
use crate::{
    db::{my_run_vec, mysql_conn},
    routes::UserInfo,
    utils::random::rand_string,
};

/// 通过 unionid 或 openid 登录
pub fn user_set_with_union_open(
    unionid: Option<String>,
    openid: String,
) -> Result<UserInfo, Error> {
    let mut conn = mysql_conn()?;

    let rand_user_name = rand_string(24);

    // unionid 存在，就用它登录
    match unionid {
        Some(uni_v) => {
            let check_user: Option<u64> = conn
                .query_first(
                    "select id from usr_silent where unionid = \"".to_string()
                        + uni_v.as_str()
                        + "\"",
                )
                .unwrap();

            if check_user.is_none() {
                // 没用户，新增
                let stmt = "insert ignore into usr_silent
                (nickname,username,openid,unionid)
                values (?,?,?,?)";
                conn.exec_drop(stmt, ("".to_string(), rand_user_name, &openid, uni_v))
                    .map_err(|e| error::ErrorInternalServerError(log_err(&e, "user_set")))?;
            }
        }
        None => {
            let check_user: Option<u64> = conn
                .query_first(
                    "select id from usr_silent where openid = \"".to_string() + &openid + "\"",
                )
                .unwrap();

            if check_user.is_none() {
                let stmt = "insert ignore into usr_silent
                (nickname,username,openid)
                values (?,?,?)";
                conn.exec_drop(stmt, ("".to_string(), rand_user_name, &openid))
                    .map_err(|e| error::ErrorInternalServerError(log_err(&e, "user_set")))?;
            }
        }
    }

    // 获取最新用户信息
    #[derive(Deserialize)]
    struct UserInfoGet {
        id: u64,
        username: String,
        nickname: String,
        avatar_url: Option<String>,
        gender: u8,
        role: String,
    }
    let user_get: Vec<UserInfoGet> = my_run_vec(
        &mut conn,
        myget!(
            "usr_silent",
            {"openid": &openid},
            "id,username,nickname,avatar_url,gender,role"
        ),
    )?;
    let token = get_token(AuthUser { id: user_get[0].id }, JWT_NORMAL_EXPIRES_SEC)?;
    let user = UserInfo {
        id: user_get[0].id,
        username: user_get[0].username.clone(),
        nickname: user_get[0].nickname.clone(),
        avatar_url: if let Some(a) = user_get[0].avatar_url.clone() {
            if a == String::from("") {
                None
            } else {
                let temp_url =
                    get_file_url_sec(Some(&a), JWT_NORMAL_EXPIRES_SEC).unwrap_or("".to_string());
                Some(temp_url)
            }
        } else {
            None
        },
        gender: user_get[0].gender,
        role: if user_get[0].role.is_empty() {
            vec![]
        } else {
            user_get[0]
                .role
                .split(",")
                .map(|x| x.parse().unwrap())
                .collect::<Vec<u16>>()
        },
        phone: None,
        token,
    };
    init_user_pocket_money(&mut conn, user.id)?;
    Ok(user)
}

/// 更新用户手机号
pub fn user_upd_phone(uid: u64, phone: &str) -> Result<UserInfo, Error> {
    let mut conn = mysql_conn()?;

    my_run_drop(
        &mut conn,
        myupdate!("usr_silent", {"id": uid}, {"phone": phone}),
    )?;

    // 获取最新用户信息
    #[derive(Deserialize)]
    struct UserInfoGet {
        id: u64,
        username: String,
        nickname: String,
        avatar_url: Option<String>,
        gender: u8,
        role: String,
        phone: Option<String>,
    }
    let user_get: Vec<UserInfoGet> = my_run_vec(
        &mut conn,
        myget!(
            "usr_silent",
            {"id": uid},
            "id,username,nickname,avatar_url,gender,role,phone"
        ),
    )?;
    let token = get_token(AuthUser { id: user_get[0].id }, JWT_NORMAL_EXPIRES_SEC)?;
    let user = UserInfo {
        id: user_get[0].id,
        username: user_get[0].username.clone(),
        nickname: user_get[0].nickname.clone(),
        avatar_url: if let Some(a) = user_get[0].avatar_url.clone() {
            if a == String::from("") {
                None
            } else {
                let temp_url =
                    get_file_url_sec(Some(&a), JWT_NORMAL_EXPIRES_SEC).unwrap_or("".to_string());
                Some(temp_url)
            }
        } else {
            None
        },
        gender: user_get[0].gender,
        role: if user_get[0].role.is_empty() {
            vec![]
        } else {
            user_get[0]
                .role
                .split(",")
                .map(|x| x.parse().unwrap())
                .collect::<Vec<u16>>()
        },
        phone: user_get[0].phone.clone(),
        token,
    };
    Ok(user)
}
