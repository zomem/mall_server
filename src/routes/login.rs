use actix_web::{Responder, Result, error, post, put, web};
use bcrypt::{DEFAULT_COST, hash, verify};
use mysql_quick::{MysqlQuickCount, Queryable, mycount, myfind, myget, myset, myupdate};
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::Res;
use super::utils_set::user_set::{user_set_with_union_open, user_upd_phone};
use crate::common::types::{FileDir, OssBucket};
use crate::common::{
    JWT_MANAGE_EXPIRES_SEC, JWT_NORMAL_EXPIRES_SEC, WECHAT_MINI_APP_ID, WECHAT_MINI_APP_SECRET,
};
use crate::control::app_data::{AppData, SlownWorker};
use crate::control::sms::sms_verify;
use crate::control::wx_info::{
    get_wx_gzh_web_silent, get_wx_gzh_web_user_info, get_wx_mini_access_token,
};
use crate::db::{my_run_drop, my_run_vec, mysql_conn};
use crate::middleware::AuthUser;
use crate::utils::files::{download_file_to_oss, get_file_url_sec, get_path_from_url};
use crate::utils::jwt::get_token;
use crate::utils::utils::log_err;

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct UserInfo {
    /// 用户 uid
    pub id: u64,
    /// 用户名
    pub username: String,
    /// 用户昵称
    pub nickname: String,
    /// 用户头像
    pub avatar_url: Option<String>,
    /// 用户性别 0 未知 , 1 男 , 2 女
    pub gender: u8,
    /// 用户角色列表：如 [1003, 1020]
    pub role: Vec<u16>,
    /// 登录授权 token
    pub token: String,
    /// 手机号，仅在绑定手机号接口时才返回
    pub phone: Option<String>,
}
#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct WechatSilent {
    code: String,
}
/// 【登录】小程序静默登录
#[utoipa::path(
    request_body = WechatSilent,
    responses((status = 200, description = "【请求：WechatSilent】【返回：UserInfo】", body = UserInfo))
)]
#[post("/login/silent/wechat/mini")]
pub async fn login_silent_wechat_mini(params: web::Json<WechatSilent>) -> Result<impl Responder> {
    let code = &params.code;

    let session_url = "https://api.weixin.qq.com/sns/jscode2session?appid=".to_string()
        + WECHAT_MINI_APP_ID
        + "&secret="
        + WECHAT_MINI_APP_SECRET
        + "&js_code="
        + code
        + "&grant_type=authorization_code";

    #[derive(Serialize, Deserialize, Debug)]
    struct SessionRes {
        expires_in: Option<usize>,
        openid: String,
        unionid: Option<String>,
        session_key: String,
    }
    let session_res: SessionRes = reqwest::get(session_url)
        .await
        .map_err(|e| error::ErrorGatewayTimeout(e))?
        .json()
        .await
        .map_err(|e| error::ErrorInternalServerError(log_err(&e, "")))?;

    let user_info = user_set_with_union_open(session_res.unionid, session_res.openid)?;

    Ok(web::Json(Res::success(user_info)))
}

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct WechatPhone {
    code: String,
}
/// 【登录】小程序绑定手机
#[utoipa::path(
    request_body = WechatPhone,
    responses((status = 200, description = "【请求：WechatSilent】【返回：UserInfo】", body = UserInfo))
)]
#[put("/login/wechat/phone/mini")]
pub async fn login_wechat_phone_mini(
    user: AuthUser,
    params: web::Json<WechatPhone>,
) -> Result<impl Responder> {
    let uid = user.id;
    let code = &params.code;
    let access_token = get_wx_mini_access_token().await?;

    let url = format!(
        "https://api.weixin.qq.com/wxa/business/getuserphonenumber?access_token={}",
        access_token
    );

    #[derive(Serialize, Deserialize, Debug)]
    struct Code {
        code: String,
    }
    #[derive(Deserialize, Debug)]
    struct WxPhoneRes {
        errcode: i32,
        errmsg: String,
        phone_info: WxPhoneInfo,
    }
    #[derive(Deserialize, Debug)]
    struct WxPhoneInfo {
        #[serde(rename = "phoneNumber")]
        phone_number: String,
        watermark: Watermark,
    }
    #[derive(Deserialize, Debug)]
    struct Watermark {
        appid: String,
    }
    let client = reqwest::Client::new();
    let data: WxPhoneRes = client
        .post(url)
        .json(&Code { code: code.clone() })
        .send()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?
        .json()
        .await
        .map_err(|e| error::ErrorBadGateway(log_err(&e, "小程序获取手机号")))?;

    if data.errcode != 0 {
        return Err(error::ErrorBadGateway(log_err(
            &data.errmsg,
            "小程序获取手机号失败",
        )));
    }
    if data.phone_info.watermark.appid != WECHAT_MINI_APP_ID {
        return Err(error::ErrorBadRequest("请求 code 错误"));
    }

    let user_info = user_upd_phone(uid, &data.phone_info.phone_number)?;

    Ok(web::Json(Res::success(user_info)))
}

/// 【登录】公众号静默登录
#[utoipa::path(
    request_body = WechatSilent,
    responses((status = 200, description = "【请求：WechatSilent】【返回：UserInfo】", body = UserInfo))
)]
#[post("/login/silent/wechat/gzh")]
pub async fn login_silent_wechat_gzh(params: web::Json<WechatSilent>) -> Result<impl Responder> {
    let code = &params.code;

    let res = get_wx_gzh_web_silent(code).await?;
    let user_info = user_set_with_union_open(res.unionid, res.openid)?;

    Ok(web::Json(Res::success(user_info)))
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct WechatLoginMiniInfo {
    #[schema(example = "zhao")]
    nickname: String,
    avatar_url: String,
    gender: Option<u8>,
    language: Option<String>,
    city: Option<String>,
    province: Option<String>,
    country: Option<String>,
}
/// 【登录】小程序用户完善
#[utoipa::path(
    request_body = WechatLoginMiniInfo,
    responses((status = 200, description = "【请求：WechatLoginMiniInfo】【返回：UserInfo】", body = Vec<UserInfo>)),
)]
#[put("/login/wechat/mini_info")]
pub async fn login_wechat_mini_info(
    user: AuthUser,
    params: web::Json<WechatLoginMiniInfo>,
) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;

    let avatar_url = get_path_from_url(&params.avatar_url, &OssBucket::EobFiles);
    // 更新用户信息
    my_run_drop(
        &mut conn,
        myupdate!("usr_silent", uid, {
            "nickname": &params.nickname,
            "avatar_url": &avatar_url,
            "gender": params.gender,
            "language": &params.language,
            "city": &params.city,
            "province": &params.province,
            "country": &params.country,
        }),
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
    }
    let user_get: Vec<UserInfoGet> = my_run_vec(
        &mut conn,
        myget!(
            "usr_silent",
            uid,
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

    Ok(web::Json(Res::success(user)))
}

/// 【登录】公众号授权登录
#[utoipa::path(
    request_body = WechatSilent,
    responses((status = 200, description = "【请求：WechatSilent】【返回：UserInfo】", body = UserInfo))
)]
#[post("/login/wechat/gzh_info")]
pub async fn login_wechat_gzh_info(
    params: web::Json<WechatSilent>,
    app_data: web::Data<AppData>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let data = &app_data;
    let code = &params.code;

    let res = get_wx_gzh_web_silent(code).await?;
    let user_init = user_set_with_union_open(res.unionid, res.openid.clone())?;

    if user_init.avatar_url.is_none() {
        let wx_user = get_wx_gzh_web_user_info(&res.openid).await?;
        // 将用户头像保存 / 更新
        let img_url = wx_user.headimgurl;
        let mut img_split = img_url.split("/").collect::<Vec<&str>>();
        img_split.pop();
        img_split.push("132");
        let img_132 = img_split.join("/");
        let file_type = "jpeg";
        let rand_name = data.rand_no(SlownWorker::OssFileName);
        let oss_dir = FileDir::Avatar;
        let dir_path = oss_dir.get_dir();
        let filepath = format!("{dir_path}/{rand_name}.{file_type}");
        download_file_to_oss(&img_132, &dir_path, &filepath, OssBucket::EobFiles)
            .await
            .map_err(|e| error::ErrorBadGateway(e))?;
        // 更新用户信息
        my_run_drop(
            &mut conn,
            myupdate!("usr_silent", user_init.id, {
               "nickname": &wx_user.nickname,
               "avatar_url": &filepath,
               "gender": wx_user.sex,
               "province": &wx_user.province,
               "city": &wx_user.city,
               "country": &wx_user.country,
            }),
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
        }
        let user_get: Vec<UserInfoGet> = my_run_vec(
            &mut conn,
            myget!(
                "usr_silent",
                user_init.id,
                "id,username,nickname,avatar_url,gender,role"
            ),
        )?;

        let user = UserInfo {
            id: user_get[0].id,
            username: user_get[0].username.clone(),
            nickname: user_get[0].nickname.clone(),
            avatar_url: if let Some(a) = user_get[0].avatar_url.clone() {
                if a == String::from("") {
                    None
                } else {
                    let temp_url = get_file_url_sec(Some(&a), JWT_NORMAL_EXPIRES_SEC)
                        .unwrap_or("".to_string());
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
            token: user_init.token,
        };

        Ok(web::Json(Res::success(user)))
    } else {
        Ok(web::Json(Res::success(user_init)))
    }
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct LoginManaRes {
    /// 用户 uid
    id: u64,
    /// 用户名
    username: String,
    /// 用户昵称
    nickname: String,
    /// 用户头像
    avatar_url: Option<String>,
    /// 用户性别
    gender: u8,
    /// 用户手机号
    phone: Option<String>,
    /// 用户角色
    role: String,
    /// 用户管理后台权限，小程序端用不到
    authority: Option<String>,
    /// 登录授权 token
    token: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ManageLogin {
    username: String,
    password: String,
}
/// 管理后台，的用户登录
#[post("/login/manage")]
pub async fn login_manage(params: web::Json<ManageLogin>) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    // 查寻有没有当前的用户
    let username = params.username.clone();
    let from_pass = params.password.clone();
    let user_pass: Option<String> = conn
        .query_first(
            "
        select password from usr_silent where username = \""
                .to_string()
                + username.as_str()
                + "\"",
        )
        .unwrap();

    let result: Res<LoginManaRes>;

    if let Some(p) = user_pass {
        let is_valid = verify(from_pass, &p).unwrap();
        if is_valid {
            let query_string = "select
            usr_silent.id,username,nickname,avatar_url,gender,phone,role,usr_authority.authority
            from usr_silent
            left join usr_authority on usr_silent.id = usr_authority.uid
            where username = \""
                .to_string()
                + username.as_str()
                + "\"";
            let mut user: Vec<LoginManaRes> = conn
                .query_map(
                    query_string.as_str(),
                    |(id, username, nickname, avatar_url, gender, phone, role, authority)| {
                        LoginManaRes {
                            id,
                            username,
                            nickname,
                            avatar_url,
                            gender,
                            phone,
                            role,
                            authority,
                            token: "".to_string(),
                        }
                    },
                )
                .unwrap();
            let token = get_token(AuthUser { id: user[0].id }, JWT_MANAGE_EXPIRES_SEC)?;
            user[0].token = token;
            user[0].avatar_url = if let Some(a) = user[0].avatar_url.clone() {
                if a == String::from("") {
                    Some(String::from(""))
                } else {
                    let temp_url = get_file_url_sec(Some(&a), JWT_MANAGE_EXPIRES_SEC)
                        .unwrap_or("".to_string());
                    Some(temp_url)
                }
            } else {
                None
            };
            result = Res::success(user[0].clone());
        } else {
            result = Res::fail("用户名或密码错误。");
        }
    } else {
        result = Res::fail("用户名或密码错误。");
    }

    Ok(web::Json(result))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ManageRegister {
    username: String,
    password: String,
    password2: String,
}
/// 管理后台，用户注册
#[post("/login/register/manage")]
pub async fn login_register_manage(params: web::Json<ManageRegister>) -> Result<impl Responder> {
    #[derive(Serialize, Debug, Deserialize)]
    struct UserTmpInfo {
        id: u64,
        username: String,
        nickname: String,
        avatar_url: Option<String>,
        gender: u8,
        phone: Option<String>,
        role: String,
        authority: Option<String>,
        // token: String,
    }

    let name = params.username.clone();
    let pass = params.password.clone();
    let pass2 = params.password2.clone();

    let re = Regex::new(r"^[0-9a-zA-Z_]{1,}$").unwrap();

    if !re.is_match(name.as_str()) {
        return Ok(web::Json(Res::fail("用户名不正确，只能数字、字母、下划线")));
    }
    if name.len() < 6 {
        return Ok(web::Json(Res::fail("用户名长度太短")));
    }
    if pass.len() < 8 {
        return Ok(web::Json(Res::fail("密码位数太短")));
    }
    if pass != pass2 {
        return Ok(web::Json(Res::fail("两次密码不匹配")));
    }

    let mut conn = mysql_conn()?;

    // 查看，有没有同名的用户
    let same_count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("usr_silent", {
            p0: ["username", "=", name.clone()],
            r: "p0",
        }),
    )?;

    // println!("同名用户： {:?}", same_count);

    if same_count[0].mysql_quick_count != 0 {
        return Ok(web::Json(Res::fail("用户已存在")));
    }
    // 没有同名用户，则可以新增
    let hash_pass = hash(pass, DEFAULT_COST).unwrap();
    let up_uid = my_run_drop(
        &mut conn,
        myset!("usr_silent", {
            "username": name.clone(),
            "nickname": name,
            "password": hash_pass,
        }),
    )?;

    let userinfo: Vec<UserTmpInfo> = my_run_vec(
        &mut conn,
        myfind!("usr_silent", {
                j0: ["id", "left", "usr_authority.uid"],
                p0: ["id", "=", up_uid],
                r: "p0",
                select: "id,username,nickname,avatar_url,gender,phone,role,usr_authority.authority",
            }
        ),
    )?;
    let tmp_user = LoginManaRes {
        id: userinfo[0].id,
        username: userinfo[0].username.clone(),
        nickname: userinfo[0].nickname.clone(),
        avatar_url: if let Some(a) = userinfo[0].avatar_url.clone() {
            if a == String::from("") {
                Some(String::from(""))
            } else {
                let temp_url =
                    get_file_url_sec(Some(&a), JWT_MANAGE_EXPIRES_SEC).unwrap_or("".to_string());
                Some(temp_url)
            }
        } else {
            None
        },
        gender: userinfo[0].gender,
        phone: userinfo[0].phone.clone(),
        role: userinfo[0].role.clone(),
        authority: userinfo[0].authority.clone(),
        token: get_token(AuthUser { id: up_uid }, JWT_MANAGE_EXPIRES_SEC)?,
    };

    Ok(web::Json(Res::success(tmp_user)))
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct BindPhone {
    phone: String,
    code: String,
}
/// 【登录】验证码绑定手机
#[utoipa::path(
    request_body = BindPhone,
    responses((status = 200, description = "【请求：BindPhone】【返回：String】", body = String)),
)]
#[put("/login/sms/bind/phone")]
pub async fn login_sms_bind_phone(
    user: AuthUser,
    params: web::Json<BindPhone>,
) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;
    let ver = sms_verify(&params.phone, &params.code)
        .map_err(|e| error::ErrorInternalServerError(log_err(&e, "")))?;
    if ver.status == 0 {
        return Ok(web::Json(Res::fail(&ver.message)));
    }
    // 验证通过，则更新用户信息
    my_run_drop(
        &mut conn,
        myupdate!("usr_silent", uid, {
            "phone": &params.phone,
        }),
    )?;

    Ok(web::Json(Res::success("")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test};
    use bcrypt::{DEFAULT_COST, hash};
    use serde_json::json;

    #[actix_web::test]
    async fn test_gen_new_pass() {
        let pass = "7n123Ha7jpR";
        let hash_pass = hash(pass, DEFAULT_COST).unwrap();
        println!("新密码：{}", hash_pass);
    }

    #[actix_web::test]
    async fn test_login_wechat_phone_mini() {
        let app = test::init_service(App::new().service(login_wechat_phone_mini)).await;

        let req = test::TestRequest::put()
            .uri("/login/wechat/phone/mini")
            .set_json(json!({
                "code": "e4335c34e62048309cb018ad812678cbd8d7c0cba4696aca0e2e370a66cfcf1d"
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
