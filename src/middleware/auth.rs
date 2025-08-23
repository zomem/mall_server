use crate::common::SUPER_SYSTEM_USER_ID;
use crate::db::mysql_conn;
use crate::utils::jwt::validate_token;
use actix_web::{Error, FromRequest, HttpRequest, dev::Payload, error};
use mysql_quick::Queryable;
use std::future::{Ready, ready};

/// 判断用户是否登录的中间件，如果用户未登录，则会返回 401
#[derive(Debug)]
pub struct AuthOptionUser {
    pub id: Option<u64>,
}
impl FromRequest for AuthOptionUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            let auth = req.headers().get("Authorization");
            if let Some(val) = auth {
                if val == "Bearer" {
                    Ok(Self { id: None })
                } else {
                    let token = val
                        .to_str()
                        .unwrap()
                        .split("Bearer ")
                        .collect::<Vec<&str>>()
                        .pop()
                        .unwrap();
                    let result = validate_token(token);
                    match result {
                        Ok(data) => Ok(Self {
                            id: Some(data.claims.id),
                        }),
                        Err(_e) => Ok(Self { id: None }),
                    }
                }
            } else {
                Ok(Self { id: None })
            }
        })
    }
}

/// 判断用户是否登录的中间件，如果用户未登录，则会返回 401
#[derive(Debug)]
pub struct AuthUser {
    pub id: u64,
}
impl FromRequest for AuthUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            let auth = req.headers().get("Authorization");
            if let Some(val) = auth {
                if val == "Bearer" {
                    Err(error::ErrorUnauthorized("用户未登录"))
                } else {
                    let token = val
                        .to_str()
                        .unwrap()
                        .split("Bearer ")
                        .collect::<Vec<&str>>()
                        .pop()
                        .unwrap();
                    let result = validate_token(token);
                    match result {
                        Ok(data) => Ok(Self { id: data.claims.id }),
                        Err(_e) => Err(error::ErrorUnauthorized("登录过期，请重新登录")),
                    }
                }
            } else {
                Err(error::ErrorUnauthorized("用户未登录"))
            }
        })
    }
}

/// 判断用户是否登录，及用户当前角色的中间件。不通过，则返回 401 403
#[allow(unused)]
pub struct AuthRole {
    pub id: u64,
    pub role: Vec<u16>,
}
impl FromRequest for AuthRole {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            let auth = req.headers().get("Authorization");
            let uri = req.path();
            if let Some(val) = auth {
                if val == "Bearer" {
                    Err(error::ErrorUnauthorized("用户未登录"))
                } else {
                    let token = val
                        .to_str()
                        .unwrap()
                        .split("Bearer ")
                        .collect::<Vec<&str>>()
                        .pop()
                        .unwrap();
                    let result = validate_token(token);
                    match result {
                        Ok(data) => {
                            // 用户 登录成功， 进行角色api权限校验
                            let uid = data.claims.id;
                            match mysql_conn() {
                                Ok(c) => {
                                    let mut conn = c;
                                    let user_role: Option<String> = conn
                                        .query_first(
                                            "select role from usr_silent where id = ".to_string()
                                                + uid.to_string().as_str(),
                                        )
                                        .unwrap();
                                    if let Some(r) = user_role {
                                        if !r.is_empty() {
                                            let user_role2 = r
                                                .clone()
                                                .split(",")
                                                .map(|i| i.parse::<u16>().unwrap())
                                                .collect::<Vec<u16>>();
                                            let user_r = user_role2
                                                .clone()
                                                .iter()
                                                .map(|x| x.to_string())
                                                .collect::<Vec<String>>()
                                                .join(",");
                                            struct ApiPaths {
                                                api_paths: String,
                                            }
                                            // 根据用户 role,查找role的api权限
                                            let role_api = conn.query_map(
                                                "select api_paths from sys_role where is_del = 0 and identifier in (".to_string()
                                                + user_r.as_str() + ")",
                                                |api_paths| {
                                                    ApiPaths {api_paths}
                                                }
                                            ).unwrap();
                                            if role_api.len() == 0 {
                                                Err(error::ErrorForbidden("暂无访问权限"))
                                            } else {
                                                let mut is_pass: bool = false;
                                                let user_all = role_api
                                                    .iter()
                                                    .map(|x| x.api_paths.as_str())
                                                    .collect::<Vec<&str>>()
                                                    .join(",")
                                                    .split(",")
                                                    .filter(|x| !x.is_empty())
                                                    .map(|x| x.to_string())
                                                    .collect::<Vec<String>>();

                                                for p in user_all {
                                                    if uri.contains(p.as_str()) {
                                                        is_pass = true;
                                                        break;
                                                    }
                                                }

                                                if !is_pass {
                                                    Err(error::ErrorForbidden("暂无访问权限"))
                                                } else {
                                                    Ok(Self {
                                                        id: uid,
                                                        role: user_role2,
                                                    })
                                                }
                                            }
                                        } else {
                                            Err(error::ErrorForbidden("暂无访问权限"))
                                        }
                                    } else {
                                        Err(error::ErrorForbidden("暂无访问权限"))
                                    }
                                }
                                Err(_) => {
                                    Err(error::ErrorInternalServerError("Auth 数据库连接错误"))
                                }
                            }
                        }
                        Err(_e) => Err(error::ErrorUnauthorized("登录过期，请重新登录")),
                    }
                }
            } else {
                Err(error::ErrorUnauthorized("用户未登录"))
            }
        })
    }
}

/// 管理后台：判断用户是否登录，及用户是否有 普通 管理员 的功能。
#[allow(unused)]
pub struct AuthMana {
    pub id: u64,
    pub authority: Vec<String>,
}
impl FromRequest for AuthMana {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            let auth = req.headers().get("Authorization");
            if let Some(val) = auth {
                if val == "Bearer" {
                    Err(error::ErrorUnauthorized("用户未登录"))
                } else {
                    let token = val
                        .to_str()
                        .unwrap()
                        .split("Bearer ")
                        .collect::<Vec<&str>>()
                        .pop()
                        .unwrap();
                    let result = validate_token(token);
                    match result {
                        Ok(data) => {
                            let uid = data.claims.id;
                            match mysql_conn() {
                                Ok(c) => {
                                    let mut conn = c;
                                    let user_manage_auth: Option<String> = conn
                                        .query_first(
                                            "
                                        select authority from usr_authority where uid = "
                                                .to_string()
                                                + uid.to_string().as_str(),
                                        )
                                        .unwrap();
                                    if let Some(a) = user_manage_auth {
                                        let a_vec = a
                                            .split(",")
                                            .map(|i| "\"".to_string() + i + "\"")
                                            .collect::<Vec<_>>();
                                        Ok(Self {
                                            id: uid,
                                            authority: a_vec,
                                        })
                                    } else {
                                        Err(error::ErrorForbidden("没有管理员权限"))
                                    }
                                }
                                Err(_) => {
                                    Err(error::ErrorInternalServerError("Auth 数据库连接错误"))
                                }
                            }
                        }
                        Err(_e) => Err(error::ErrorUnauthorized("登录过期，请重新登录")),
                    }
                }
            } else {
                Err(error::ErrorUnauthorized("用户未登录"))
            }
        })
    }
}

/// 管理后台：判断用户是否登录，及用户是否有 超级 管理员 的功能。
#[allow(unused)]
pub struct AuthSuperMana {
    pub id: u64,
    pub authority: Vec<String>,
}
impl FromRequest for AuthSuperMana {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            let auth = req.headers().get("Authorization");
            if let Some(val) = auth {
                if val == "Bearer" {
                    Err(error::ErrorUnauthorized("用户未登录"))
                } else {
                    let token = val
                        .to_str()
                        .unwrap()
                        .split("Bearer ")
                        .collect::<Vec<&str>>()
                        .pop()
                        .unwrap();
                    let result = validate_token(token);
                    match result {
                        Ok(data) => {
                            let uid = data.claims.id;
                            match mysql_conn() {
                                Ok(c) => {
                                    let mut conn = c;
                                    let user_manage_auth: Option<String> = conn
                                        .query_first(
                                            "
                                        select authority from usr_authority where uid = "
                                                .to_string()
                                                + uid.to_string().as_str(),
                                        )
                                        .unwrap();
                                    if let Some(a) = user_manage_auth {
                                        let s_uid = SUPER_SYSTEM_USER_ID;
                                        if s_uid == uid {
                                            let a_vec = a
                                                .split(",")
                                                .map(|i| "\"".to_string() + i + "\"")
                                                .collect::<Vec<_>>();
                                            Ok(Self {
                                                id: uid,
                                                authority: a_vec,
                                            })
                                        } else {
                                            Err(error::ErrorForbidden("没有超级管理员权限"))
                                        }
                                    } else {
                                        Err(error::ErrorForbidden("没有管理员权限"))
                                    }
                                }
                                Err(_) => {
                                    Err(error::ErrorInternalServerError("Auth 数据库连接错误"))
                                }
                            }
                        }
                        Err(_e) => Err(error::ErrorUnauthorized("登录过期，请重新登录")),
                    }
                }
            } else {
                Err(error::ErrorUnauthorized("用户未登录"))
            }
        })
    }
}
