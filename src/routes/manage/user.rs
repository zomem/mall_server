use actix_web::{Responder, Result, get, put, web};
use mysql_quick::{MysqlQuickCount, Queryable, TxOpts, mycount, myfind, myset, myupdate};
use serde::{Deserialize, Serialize};

use crate::PageData;
use crate::common::types::Role;
use crate::db::{my_run_tran_drop, mysql_tran};
use crate::routes::Res;
use crate::routes::utils_set::sales_set::{
    main_sale_add, sale_add, sale_and_main_del, user_and_sale_del,
};
use crate::utils::files::{get_file_url, get_file_urls};
use crate::utils::utils::hide_phone_number;
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::{AuthMana, AuthSuperMana},
};

#[derive(Serialize, Clone, Deserialize)]
struct SearchUsers {
    id: u64,
    username: String,
    nickname: String,
    phone: Option<String>,
    avatar_url: Option<String>,
    authority: Option<String>,
    role: String,
}
/// 搜索用户，
#[get("/manage/user/search/{keyword}")]
pub async fn manage_user_search(
    _mana: AuthMana,
    keyword: web::Path<String>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    let users: Vec<SearchUsers> = my_run_vec(
        &mut conn,
        myfind!("usr_silent", {
            j0: ["id", "left", "usr_authority.uid"],
            p0: ["nickname", "like", format!("%{}%", &keyword)],
            r: "p0",
            select: "id,username,nickname,phone,avatar_url,usr_authority.authority,role",
        }),
    )?;

    let users: Vec<SearchUsers> = users
        .into_iter()
        .map(|u| {
            let mut user = u.clone();
            user.avatar_url = if let Some(a) = u.avatar_url.clone() {
                if a == String::from("") {
                    Some(String::from(""))
                } else {
                    let temp_url = get_file_url(Some(&a)).unwrap_or("".to_owned());
                    Some(temp_url)
                }
            } else {
                None
            };
            user
        })
        .collect();
    Ok(web::Json(Res::success(users)))
}

#[derive(Serialize, Clone, Deserialize)]
struct SearchUsersPhone {
    id: u64,
    username: String,
    nickname: String,
    gender: Option<i8>,
    phone: Option<String>,
    avatar_url: Option<String>,
    role: String,
    label: String,
    value: u64,
}
/// 手机号搜索用户，
#[get("/manage/user/search/phone/{keyword}")]
pub async fn manage_user_search_phone(
    _mana: AuthMana,
    keyword: web::Path<String>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let key = keyword.to_owned();

    #[derive(Serialize, Clone, Deserialize)]
    struct SearchUsersPhoneGet {
        id: u64,
        username: String,
        nickname: String,
        gender: Option<i8>,
        phone: Option<String>,
        avatar_url: Option<String>,
        role: String,
    }
    let users: Vec<SearchUsersPhoneGet> = my_run_vec(
        &mut conn,
        myfind!("usr_silent", {
            p0: ["phone", "like", format!("{}%", key.trim())],
            r: "p0",
            select: "id,username,nickname,gender,phone,avatar_url,role",
        }),
    )?;

    let users: Vec<SearchUsersPhone> = users
        .into_iter()
        .map(|u| SearchUsersPhone {
            id: u.id,
            username: u.username,
            nickname: u.nickname.clone(),
            gender: u.gender,
            phone: u.phone.clone(),
            avatar_url: if let Some(a) = u.avatar_url.clone() {
                if a == String::from("") {
                    Some(String::from(""))
                } else {
                    let temp_url = get_file_url(Some(&a)).unwrap_or("".to_owned());
                    Some(temp_url)
                }
            } else {
                None
            },
            role: u.role,
            label: if let Some(p) = u.phone {
                p + "【" + &u.nickname + "】"
            } else {
                "".to_string()
            },
            value: u.id,
        })
        .collect();

    Ok(web::Json(Res::success(users)))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthChange {
    uid: u64,
    authority: String,
}
/// 更改用户权限
#[put("/manage/user/update/authority")]
pub async fn manage_user_update_authority(
    _super_mana: AuthSuperMana,
    params: web::Json<AuthChange>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let sql = mycount!("usr_authority", {
        p0: ["uid", "=", params.uid],
        r: "p0",
    });
    let user_auth: Vec<MysqlQuickCount> = my_run_vec(&mut conn, sql)?;
    if user_auth[0].mysql_quick_count > 0 {
        // 有
        conn.query_drop(
            "update usr_authority set authority = \"".to_string()
                + &params.authority
                + "\" where uid = "
                + params.uid.to_string().as_str(),
        )
        .unwrap();
    } else {
        // 没有，新增
        my_run_drop(
            &mut conn,
            myset!("usr_authority", {
                "authority": &params.authority,
                "uid": params.uid,
            }),
        )?;
    }

    Ok(web::Json(Res::<u8>::info(2, "更新成功")))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoleChange {
    uid: u64,
    role: String,
}
/// 更改用户角色
#[put("/manage/user/update/user/role")]
pub async fn manage_user_update_user_role(
    _mana: AuthMana,
    params: web::Json<RoleChange>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    // ----- 事务开始
    let mut tran = mysql_tran(&mut conn)?;
    let uid = params.uid;
    let role = params.role.clone();
    let mut role_list = role
        .split(",")
        .filter(|x| !x.is_empty())
        .map(|x| x.parse::<u16>().unwrap())
        .collect::<Vec<u16>>();
    if role_list.contains(&(Role::MainSale as u16)) {
        // 有总销售
        if !role_list.contains(&(Role::Sale as u16)) {
            // 没有销售
            role_list.push(Role::Sale as u16);
        }
        main_sale_add(&mut tran, uid)?;
    } else {
        // 没有总销售，但有销售
        if role_list.contains(&(Role::Sale as u16)) {
            sale_add(&mut tran, uid)?;
            sale_and_main_del(&mut tran, uid, uid)?;
        } else {
            // 没有总销售，也没有销售
            user_and_sale_del(&mut tran, uid, uid)?;
        }
    }
    let role = role_list
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(",");
    my_run_tran_drop(
        &mut tran,
        myupdate!("usr_silent", uid, {
            "role": &role,
        }),
    )?;
    tran.commit().unwrap();
    // ---- 事务结束

    Ok(web::Json(Res::<u8>::info(2, "更新成功")))
}

#[derive(Serialize, Deserialize, Clone)]
struct UserItem {
    id: u64,
    username: String,
    nickname: Option<String>,
    avatar_url: Option<String>,
    gender: u8,
    created_at: String,
}
/// 获取所有用户列表, 至少需要管理员的身份
#[get("/manage/user/all_users/{page}/{limit}")]
pub async fn manage_user_all_users(
    _mana: AuthMana,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (page, limit) = query.to_owned();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();
    let all_users: Vec<UserItem> = my_run_vec(
        &mut conn,
        myfind!("usr_silent", {
            p0: ["id", ">", 0],
            r: "p0",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id,username,nickname,avatar_url,gender,created_at",
        }),
    )?;

    let users: Vec<UserItem> = all_users
        .into_iter()
        .map(|u| {
            let mut user = u.clone();
            user.avatar_url = if let Some(a) = u.avatar_url.clone() {
                if a == String::from("") {
                    Some(String::from(""))
                } else {
                    let temp_url = get_file_url(Some(&a)).unwrap_or("".to_owned());
                    Some(temp_url)
                }
            } else {
                None
            };
            user
        })
        .collect();

    let count: Vec<MysqlQuickCount> = my_run_vec(&mut conn, mycount!("usr_silent", {}))?;

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        users,
    ))))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FeedbackRes {
    id: u32,
    nickname: Option<String>,
    imgs: Vec<String>,
    content: Option<String>,
    created_at: String,
}
#[derive(Serialize, Deserialize, Clone)]
struct FeedbackInfo {
    id: u32,
    images: Option<String>,
    content: Option<String>,
    nickname: Option<String>,
    created_at: String,
}
/// 反馈列表
#[get("/manage/user/feedback/list/{page}/{limit}")]
pub async fn manage_user_feedback_list(
    _user: AuthMana,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (page, limit) = query.to_owned();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(&mut conn, mycount!("usr_feedback", {}))?;

    let list: Vec<FeedbackInfo> = my_run_vec(
        &mut conn,
        myfind!("usr_feedback", {
            j0: ["uid", "inner", "usr_silent.id"],
            p0: ["id", ">", 0],
            r: "p0",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id,images,content,usr_silent.nickname,created_at",
        }),
    )?;

    let list: Vec<FeedbackRes> = list
        .into_iter()
        .map(|x| FeedbackRes {
            id: x.id,
            nickname: x.nickname,
            imgs: if let Some(img) = x.images {
                get_file_urls(Some(&img))
            } else {
                vec![]
            },
            content: x.content,
            created_at: x.created_at,
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize, Clone)]
struct Credential {
    id: u32,
    uid: u64,
    nickname: Option<String>,
    phone: Option<String>,
    gender: Option<u8>,
    title: String,
    role: String,
    role_name: String,
    content: Option<String>,
    imgs: Vec<String>,
    reason: Option<String>,
    status: u8,
    created_at: String,
}
/// 获取用户角色申请表
#[get("/manage/user/credential/{status}/{page}/{limit}")]
pub async fn manage_user_credential(
    _mana: AuthMana,
    query: web::Path<(String, String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (status, page, limit) = query.to_owned();
    let status: u8 = status.to_owned().parse().unwrap();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("usr_credential", {
            p0: ["is_del", "=", 0],
            p1: ["status", "=", status],
            r: "p0 && p1",
        }),
    )?;

    #[derive(Deserialize)]
    struct CredentialGet {
        id: u32,
        uid: u64,
        nickname: Option<String>,
        phone: Option<String>,
        gender: Option<u8>,
        title: String,
        role: String,
        role_name: String,
        content: Option<String>,
        imgs: Option<String>,
        reason: Option<String>,
        status: u8,
        created_at: String,
    }
    let list: Vec<CredentialGet> = my_run_vec(
        &mut conn,
        myfind!("usr_credential", {
            j0: ["uid", "inner", "usr_silent.id"],
            j1: ["role", "inner", "sys_role.identifier"],
            p0: ["is_del", "=", 0],
            p1: ["status", "=", status],
            p2: ["sys_role.is_del", "=", 0],
            r: "p0 && p1 && p2",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "
                id,uid,usr_silent.nickname,usr_silent.phone,usr_silent.gender,title,role,content,imgs,
                reason,status,created_at,sys_role.name as role_name",
        }),
    )?;

    let list: Vec<Credential> = list
        .into_iter()
        .map(|x| Credential {
            id: x.id,
            uid: x.uid,
            nickname: x.nickname,
            phone: x.phone,
            gender: x.gender,
            title: x.title,
            role: x.role,
            role_name: x.role_name,
            content: x.content,
            imgs: get_file_urls(x.imgs),
            reason: x.reason,
            status: x.status,
            created_at: x.created_at,
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CredentialStatus {
    id: u32,
    status: u8,
    reason: Option<String>,
}
/// 修改用户角色申请的状态
#[put("/manage/user/credential/status")]
pub async fn manage_user_credential_status(
    _mana: AuthMana,
    params: web::Json<CredentialStatus>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    // 获取审核的角色信息
    #[derive(Deserialize)]
    struct CrenGet {
        uid: u64,
        role: String,
    }
    let cre_info: Vec<CrenGet> = my_run_vec(
        &mut conn,
        myfind!("usr_credential", {
            p0: ["id", "=", params.id],
            r: "p0",
            select: "uid, role",
        }),
    )?;

    if cre_info.len() == 0 {
        return Ok(web::Json(Res::fail("没有申请信息")));
    }

    // 获取用户的角色信息
    #[derive(Deserialize)]
    struct UserRoleGet {
        role: String,
    }
    let user_role: Vec<UserRoleGet> = my_run_vec(
        &mut conn,
        myfind!("usr_silent", {
            p0: ["id", "=", cre_info[0].uid],
            r: "p0",
            select: "role",
        }),
    )?;
    if user_role.len() == 0 {
        return Ok(web::Json(Res::fail("用户不存在")));
    }
    let mut user_role_list: Vec<&str> = user_role[0]
        .role
        .split(",")
        .filter(|x| !x.is_empty())
        .collect();

    let mut tran = conn.start_transaction(TxOpts::default()).unwrap();
    match my_run_tran_drop(
        &mut tran,
        myupdate!("usr_credential", params.id, {
            "status": params.status,
            "reason": &params.reason,
        }),
    ) {
        Ok(_) => (),
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    if params.status == 2 {
        // 添加权限
        if !user_role_list.contains(&cre_info[0].role.as_str()) {
            user_role_list.push(cre_info[0].role.as_str())
        }
    } else {
        // 去掉权限
        let index = user_role_list
            .iter()
            .position(|&x| x == cre_info[0].role.as_str());
        if let Some(i) = index {
            user_role_list.remove(i);
        }
    }
    // 更新用户权限信息
    match my_run_tran_drop(
        &mut tran,
        myupdate!("usr_silent", cre_info[0].uid, {
            "role": user_role_list.join(","),
        }),
    ) {
        Ok(_) => (),
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    tran.commit().unwrap();

    Ok(web::Json(Res::success("成功")))
}

#[derive(Deserialize)]
struct RoleUserSearch {
    name: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct RoleUserInfo {
    id: u32,
    nickname: String,
    avatar_url: String,
    phone: Option<String>,
    gender: i8,
    role: Vec<u16>,
    created_at: String,
}
#[get("/manage/user/roles/list/{role}/{page}/{limit}")]
pub async fn manage_user_roles_list(
    _mana: AuthMana,
    query: web::Path<(String, String, String)>,
    search: web::Query<RoleUserSearch>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (role, page, limit) = query.to_owned();
    let role: u32 = role.to_owned().parse().unwrap();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let role: Role = role.into();

    let nick_name = search.name.clone().unwrap_or("".to_string());

    let r = if nick_name.is_empty() {
        "p0"
    } else {
        "p0 && p1"
    };

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("usr_silent", {
            p0: ["role", "like", format!("%{}%", role as u32)],
            p1: ["nickname", "like", format!("%{}%", nick_name)],
            r: r,
        }),
    )?;

    #[derive(Serialize, Deserialize, Debug)]
    struct Get {
        id: u32,
        nickname: String,
        avatar_url: Option<String>,
        phone: Option<String>,
        gender: i8,
        created_at: String,
        role: String,
    }

    let list: Vec<Get> = my_run_vec(
        &mut conn,
        myfind!("usr_silent", {
            p0: ["role", "like", format!("%{}%", role as u32)],
            p1: ["nickname", "like", format!("%{}%", nick_name)],
            r: r,
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id,nickname,avatar_url,phone,gender,role,created_at",
        }),
    )?;

    let list: Vec<RoleUserInfo> = list
        .into_iter()
        .map(|x| {
            return RoleUserInfo {
                id: x.id,
                avatar_url: get_file_url(x.avatar_url).unwrap_or("".to_string()),
                nickname: x.nickname,
                created_at: x.created_at,
                phone: x.phone.map_or(None, |p| Some(hide_phone_number(&p))),
                gender: x.gender,
                role: x
                    .role
                    .split(",")
                    .filter(|x| !x.is_empty())
                    .map(|r| r.parse::<u16>().unwrap())
                    .collect::<Vec<_>>(),
            };
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}
