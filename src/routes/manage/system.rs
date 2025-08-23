use actix_web::{Responder, Result, get, post, put, web};
use mysql_quick::{MysqlQuickCount, Queryable, mycount, myfind, myupdate};
use serde::{Deserialize, Serialize};

use crate::db::my_run_drop;
use crate::routes::Res;
use crate::utils::files::get_file_url;
use crate::{
    db::{my_run_vec, mysql_conn},
    middleware::{AuthMana, AuthSuperMana},
};

#[derive(Serialize, Deserialize)]
pub struct NumberLabel {
    label: String,
    value: u32,
}

#[derive(Serialize, Deserialize)]
pub struct StringLabel {
    label: String,
    value: String,
}

#[derive(Serialize, Debug, Clone)]
struct SysPath {
    id: u64,
    name: Option<String>,
    sub_name: Option<String>,
    path: String,
    sub_path: Option<String>,
    icon_name: Option<String>,
    path_type: i8,
    uni_key: String,
    sort_num: u32,
}

#[derive(Serialize)]
struct PathsList {
    id: u64,
    name: String,
    path: String,
    icon_name: String,
    path_type: u8,
    sort_num: u32,
}

#[derive(Serialize)]
struct SubPathsList {
    id: u64,
    sub_name: String,
    path: String,
    sub_path: String,
    path_type: u8,
    sort_num: u32,
}
/// 获取用户可操作的 菜单列表。
#[get("/manage/system/menu/list")]
pub async fn manage_system_menu_list(mana: AuthMana) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let user_authority = mana.authority;

    #[derive(Serialize)]
    struct PathList {
        id: u64,
        name: Option<String>,
        sub_name: Option<String>,
        path: String,
        sub_path: Option<String>,
        icon_name: Option<String>,
        path_type: i8,
        uni_key: String,
        sort_num: u32,
        sub_list: Vec<SysPath>,
    }
    let mut path_list: Vec<PathList> = vec![];

    let check_auth = user_authority.join(",");
    //  查寻用户权限的 所有子页面的path；
    let temp_sub_list = conn
        .query_map(
            "select id,name,sub_name,path,sub_path,icon_name,path_type,uni_key,sort_num
        from sys_paths where path_type = 2 and sub_path in ("
                .to_string()
                + check_auth.as_str()
                + ") order by sort_num desc",
            |(id, name, sub_name, path, sub_path, icon_name, path_type, uni_key, sort_num)| {
                SysPath {
                    id,
                    name,
                    sub_name,
                    path,
                    sub_path,
                    icon_name,
                    path_type,
                    uni_key,
                    sort_num,
                }
            },
        )
        .unwrap();
    let temp_path = temp_sub_list
        .iter()
        .map(|s| "\"".to_string() + s.path.clone().as_str() + "\"")
        .collect::<Vec<String>>()
        .join(",");

    //  查寻用户权限的 所有页面的path；
    let mut temp_list: Vec<SysPath> = vec![];
    if temp_path != "".to_string() {
        temp_list = conn
            .query_map(
                "select id,name,sub_name,path,sub_path,icon_name,path_type,uni_key,sort_num
            from sys_paths where path_type = 1 and path in ("
                    .to_string()
                    + temp_path.as_str()
                    + ") order by sort_num desc",
                |(id, name, sub_name, path, sub_path, icon_name, path_type, uni_key, sort_num)| {
                    SysPath {
                        id,
                        name,
                        sub_name,
                        path,
                        sub_path,
                        icon_name,
                        path_type,
                        uni_key,
                        sort_num,
                    }
                },
            )
            .unwrap();
    }

    for item in temp_list {
        let mut o = PathList {
            id: item.id,
            name: item.name,
            sub_name: item.sub_name,
            path: item.path.clone(),
            sub_path: item.sub_path,
            icon_name: item.icon_name,
            path_type: item.path_type,
            uni_key: item.uni_key,
            sort_num: item.sort_num,
            sub_list: vec![],
        };
        let clone_sub_list = temp_sub_list.clone();
        for sub_item in clone_sub_list {
            if item.path == sub_item.path {
                o.sub_list.push(sub_item);
            }
        }
        path_list.push(o);
    }
    Ok(web::Json(Res::success(path_list)))
}

/// 获取父模块
#[get("/manage/system/paths/list")]
pub async fn manage_system_paths_list(_super_mana: AuthSuperMana) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let paths = conn
        .query_map(
            "select id,name,path,icon_name,path_type,sort_num from sys_paths where path_type = 1",
            |(id, name, path, icon_name, path_type, sort_num)| PathsList {
                id,
                name,
                path,
                icon_name,
                path_type,
                sort_num,
            },
        )
        .unwrap();

    Ok(web::Json(Res::success(paths)))
}

/// 子模块儿
#[get("/manage/system/sub_paths/list/{f_path}")]
pub async fn manage_system_sub_paths_list(
    _super_mana: AuthSuperMana,
    f_path: web::Path<String>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let sub_paths = conn
        .query_map(
            "select id,sub_name,path,sub_path,path_type,sort_num from sys_paths
            where path_type = 2 and path = \""
                .to_string()
                + f_path.as_str()
                + "\"",
            |(id, sub_name, path, sub_path, path_type, sort_num)| SubPathsList {
                id,
                sub_name,
                path,
                sub_path,
                path_type,
                sort_num,
            },
        )
        .unwrap();

    Ok(web::Json(Res::success(sub_paths)))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PathAdd {
    icon: Option<String>,
    path: String,
    sub_path: String,
    name: String,
    sub_name: String,
    sort_num: Option<u32>,
}
/// 新增模块
#[post("/manage/system/paths/add")]
pub async fn manage_system_paths_add(
    _super_mana: AuthSuperMana,
    params: web::Json<PathAdd>,
) -> Result<impl Responder> {
    let path = params.path.clone();
    let path_uni = path.clone() + "_" + "1";
    let sub_path = params.sub_path.clone();
    let sub_path_uni = sub_path.clone() + "_" + "2";
    let name = params.name.clone();
    let sub_name = params.sub_name.clone();
    let icon_name = if let Some(icon_n) = params.icon.clone() {
        icon_n
    } else {
        "SettingOutlined".to_string()
    };
    let sort_num = if let Some(s_num) = params.sort_num.clone() {
        s_num
    } else {
        50
    };

    let mut conn = mysql_conn()?;

    let fpath_id: Option<u64> = conn
        .query_first(
            "select id from sys_paths where uni_key = \"".to_string() + path.as_str() + "_1\"",
        )
        .unwrap();

    let spath_id: Option<u64> = conn
        .query_first(
            "select id from sys_paths where uni_key = \"".to_string() + sub_path.as_str() + "_2\"",
        )
        .unwrap();

    if let Some(fid) = fpath_id {
        conn.query_drop(
            "update sys_paths set name=\"".to_string()
                + name.as_str()
                + "\""
                + ", icon_name=\""
                + icon_name.as_str()
                + "\" "
                + "where id = "
                + fid.to_string().as_str(),
        )
        .unwrap();
    } else {
        conn.exec_drop(
            "insert into sys_paths (name,path,icon_name,uni_key,path_type,sort_num)
            values (?,?,?,?,?,?)
            ",
            (name, path.clone(), icon_name, path_uni, 1, sort_num), // params! {
                                                                    //     "name" => name,
                                                                    //     "path" => path.clone(),
                                                                    //     "icon_name" => icon_name,
                                                                    //     "uni_key" => path_uni,
                                                                    //     "path_type" => 1,
                                                                    //     "sort_num" => sort_num,
                                                                    // }
        )
        .unwrap();
    }

    if let Some(sid) = spath_id {
        conn.query_drop(
            "update sys_paths set sub_name=\"".to_string()
                + sub_name.as_str()
                + "\" "
                + "where id = "
                + sid.to_string().as_str(),
        )
        .unwrap();
    } else {
        conn.exec_drop(
            "insert into sys_paths (sub_name,sub_path,path,uni_key,path_type,sort_num)
            values (?,?,?,?,?,?)
            ",
            (sub_name, sub_path, path, sub_path_uni, 2, sort_num + 1), // params! {
                                                                       //     "sub_name" => sub_name,
                                                                       //     "sub_path" => sub_path,
                                                                       //     "path" => path,
                                                                       //     "uni_key" => sub_path_uni,
                                                                       //     "path_type" => 2,
                                                                       //     "sort_num" => sort_num + 1,
                                                                       // }
        )
        .unwrap();
    }

    Ok(web::Json(Res::<u8>::info(1, "操作成功")))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PathId {
    id: u64,
}

/// 删除模块儿
#[post("/manage/system/paths/delete")]
pub async fn manage_system_paths_delete(
    _super_mana: AuthSuperMana,
    params: web::Json<PathId>,
) -> Result<impl Responder> {
    let mut result = Res::<u8>::info(0, "失败");

    let id = params.id;
    let mut conn = mysql_conn()?;

    let path_data: Option<(u8, String, Option<String>)> = conn
        .query_first(
            "select path_type,path,sub_path from sys_paths where id = ".to_string()
                + id.to_string().as_str(),
        )
        .unwrap();

    if let Some((t, p, s)) = path_data {
        if t == 2 {
            if let Some(sp) = s {
                let fms = format!("%{sp}%");
                let has_count: Vec<MysqlQuickCount> = my_run_vec(
                    &mut conn,
                    mycount!("usr_authority", {
                        p0: ["authority", "like", fms],
                        r: "p0",
                    }),
                )?;
                if has_count[0].mysql_quick_count > 0 {
                    return Ok(web::Json(Res::<u8>::fail("删除失败，有用户具有该权限")));
                }
            }
            conn.query_drop(
                "delete from sys_paths where id = ".to_string() + id.to_string().as_str(),
            )
            .unwrap();
            result = Res::<u8>::info(1, "删除成功");
        } else {
            let count: Option<u32> = conn
                .query_first(
                    "select count(*) from sys_paths where path_type = 2 and path = \"".to_string()
                        + p.as_str()
                        + "\"",
                )
                .unwrap();
            if let Some(c) = count {
                if c > 0 {
                    result = Res::<u8>::info(0, "删除失败，请先删除子模块");
                } else {
                    conn.query_drop(
                        "delete from sys_paths where id = ".to_string() + id.to_string().as_str(),
                    )
                    .unwrap();
                    result = Res::<u8>::info(1, "删除成功");
                }
            } else {
                result = Res::<u8>::info(0, "未执行操作");
            }
        }
    }

    Ok(web::Json(result))
}

#[derive(Serialize, Clone)]
struct PathsAllChild {
    title: String,
    key: String,
}
#[derive(Serialize, Clone)]
pub struct PathsAll {
    title: String,
    key: String,
    children: Vec<PathsAllChild>,
}

/// 获取所有模块
#[get("/manage/system/paths/all")]
pub async fn manage_system_paths_all(_super_mana: AuthSuperMana) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    #[derive(Serialize, Deserialize, Clone)]
    struct PathsList {
        id: u64,
        name: Option<String>,
        sub_name: Option<String>,
        path: String,
        sub_path: Option<String>,
        path_type: u8,
        sort_num: u32,
    }

    let temp_list = conn.query_map(
        "select id,name,sub_name,path,sub_path,path_type,sort_num from sys_paths order by sort_num",
        |(id,name,sub_name,path,sub_path,path_type,sort_num)| {
            PathsList {id,name,sub_name,path,sub_path,path_type,sort_num}
        }
    ).unwrap();

    let temp_list2 = temp_list.clone();

    let mut temp_data: Vec<PathsAll> = vec![];
    let mut result: Vec<PathsAll> = vec![];

    // 生成父数据
    for f in temp_list {
        if f.path_type == 1 {
            let o = PathsAll {
                title: if let Some(n) = f.name {
                    n
                } else {
                    "".to_string()
                },
                key: f.path,
                children: vec![],
            };
            temp_data.push(o);
        }
    }

    // 找出对应子数据
    for td in temp_data {
        let mut res = td.clone();
        let temp_l = temp_list2.clone();
        for l in temp_l {
            if l.path_type == 2 && td.key == l.path {
                let oo = PathsAllChild {
                    title: if let Some(n) = l.sub_name {
                        n
                    } else {
                        "".to_string()
                    },
                    key: if let Some(sp) = l.sub_path {
                        sp
                    } else {
                        "".to_string()
                    },
                };
                res.children.push(oo);
            }
        }
        result.push(res);
    }

    Ok(web::Json(Res::success(result)))
}

#[derive(Serialize)]
struct RoleList {
    id: u64,
    name: String,
    identifier: String,
    is_del: u8,
    api_paths: String,
}
/// 角色列表
#[get("/manage/system/role/list")]
pub async fn manage_system_role_list(_super_mana: AuthSuperMana) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    let roles = conn
        .query_map(
            "select id,name,identifier,is_del,api_paths from sys_role where is_del = 0",
            |(id, name, identifier, is_del, api_paths)| RoleList {
                id,
                name,
                identifier,
                is_del,
                api_paths,
            },
        )
        .unwrap();

    Ok(web::Json(Res::success(roles)))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoleAdd {
    name: String,
    identifier: u16,
    api_paths: String,
}
/// 新增用户角色
#[post("/manage/system/role/add")]
pub async fn manage_system_role_add(
    _super_mana: AuthSuperMana,
    params: web::Json<RoleAdd>,
) -> Result<impl Responder> {
    let name = params.name.clone();
    let identifier = params.identifier;
    let api_paths = params.api_paths.clone();

    if identifier < 1000 || identifier > 9999 {
        return Ok(web::Json(Res::<u8>::fail(
            "角色编号的取值在 1000~9999 之间",
        )));
    }

    let mut conn = mysql_conn()?;

    let role_num: Option<u32> = conn
        .query_first(
            "select count(*) from sys_role where identifier = ".to_string()
                + identifier.to_string().as_str()
                + " or name = \""
                + name.as_str()
                + "\"",
        )
        .unwrap();

    if let Some(n) = role_num {
        if n > 0 {
            return Ok(web::Json(Res::<u8>::fail("已存在相同角色名或角色编号")));
        }
    }

    conn.exec_drop(
        "insert into sys_role (name,identifier,api_paths) values (?,?,?)",
        (name, identifier, api_paths), // params! {
                                       //     "name" => name,
                                       //     "identifier" => identifier,
                                       //     "api_paths" => api_paths
                                       // }
    )
    .unwrap();

    Ok(web::Json(Res::<u8>::info(1, "新增成功")))
}

#[derive(Serialize)]
struct SysRole {
    id: u64,
    name: String,
    identifier: String,
    is_del: i8,
    api_paths: String,
}
/// 获取角色详情
#[get("/manage/system/role/info/{id}")]
pub async fn manage_system_role_info(
    _super_mana: AuthSuperMana,
    id: web::Path<String>,
) -> Result<impl Responder> {
    let role_id = id.as_str();
    let mut conn = mysql_conn()?;

    let info: Option<(u64, String, String, i8, String)> = conn
        .query_first(
            "select id,name,identifier,is_del,api_paths from sys_role where id = ".to_string()
                + role_id,
        )
        .unwrap();

    if let Some(data) = info {
        return Ok(web::Json(Res::success(SysRole {
            id: data.0,
            name: data.1,
            identifier: data.2,
            is_del: data.3,
            api_paths: data.4,
        })));
    }

    Ok(web::Json(Res::success(SysRole {
        id: 0,
        name: "".to_string(),
        identifier: "".to_string(),
        is_del: 0,
        api_paths: "".to_string(),
    })))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoleUpdate {
    id: u64,
    name: String,
    identifier: u16,
    api_paths: String,
}
/// 更新角色详情
#[put("/manage/system/role/update")]
pub async fn manage_system_role_update(
    _super_mana: AuthSuperMana,
    params: web::Json<RoleUpdate>,
) -> Result<impl Responder> {
    let name = params.name.clone();
    let identifier = params.identifier;
    let api_paths = params.api_paths.clone();
    let id = params.id;

    let mut conn = mysql_conn()?;

    if identifier < 1000 || identifier > 9999 {
        return Ok(web::Json(Res::fail("角色编号的取值在 1000~9999 之间")));
    }

    let ident: Option<String> = conn
        .query_first(
            "select identifier from sys_role where id = ".to_string() + id.to_string().as_str(),
        )
        .unwrap();

    if let Some(ide) = ident {
        if identifier.to_string() != ide {
            let count_role: Option<i8> = conn
                .query_first(
                    "select count(*) from usr_silent where role like \"%".to_string()
                        + ide.as_str()
                        + "%\"",
                )
                .unwrap();

            if let Some(_cr) = count_role {
                return Ok(web::Json(Res::fail("已有用户使用该角色编号，不能更改")));
            }
        }
    } else {
        return Ok(web::Json(Res::fail("未找到该角色")));
    }

    conn.query_drop(
        "update sys_role set identifier = ".to_string()
            + identifier.to_string().as_str()
            + ", api_paths = \""
            + api_paths.as_str()
            + "\""
            + ", name = \""
            + name.as_str()
            + "\""
            + " where id = "
            + id.to_string().as_str(),
    )
    .unwrap();

    Ok(web::Json(Res::<u8>::info(1, "更新成功")))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoleDel {
    id: u64,
}
/// 删除角色
#[post("/manage/system/role/del")]
pub async fn manage_system_role_del(
    _super_mana: AuthSuperMana,
    params: web::Json<RoleDel>,
) -> Result<impl Responder> {
    let role_id = params.id;

    let mut conn = mysql_conn()?;
    let ident: Option<String> = conn
        .query_first(
            "select identifier from sys_role where id = ".to_string()
                + role_id.to_string().as_str(),
        )
        .unwrap();

    if let Some(ide) = ident {
        let count: Option<i8> = conn
            .query_first(
                "select count(*) from usr_silent where role like \"%".to_string()
                    + ide.as_str()
                    + "%\"",
            )
            .unwrap();
        if let Some(c) = count {
            if c > 0 {
                return Ok(web::Json(Res::fail("不能删除，当前有用户属于该角色")));
            } else {
                conn.query_drop(
                    "update sys_role set is_del = 1 where id = ".to_string()
                        + role_id.to_string().as_str(),
                )
                .unwrap();

                return Ok(web::Json(Res::<u8>::info(1, "删除成功")));
            }
        } else {
            return Ok(web::Json(Res::fail("删除失败")));
        }
    } else {
        Ok(web::Json(Res::fail("删除失败，未找到该角色的编号")))
    }
}

#[derive(Serialize, Clone)]
struct RoleUsers {
    id: u64,
    avatar_url: Option<String>,
    username: String,
    nickname: Option<String>,
}
/// 通过角色编辑，查寻当前角色，有哪些用户
#[get("/manage/system/role/{role_author}/users")]
pub async fn manage_system_role_user(
    _super_mana: AuthSuperMana,
    role_author: web::Path<String>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    let role_users = conn
        .query_map(
            "select id,avatar_url,username,nickname from usr_silent where role like \"%"
                .to_string()
                + role_author.as_str()
                + "%\"",
            |(id, avatar_url, username, nickname)| RoleUsers {
                id,
                avatar_url,
                username,
                nickname,
            },
        )
        .unwrap();
    let users: Vec<RoleUsers> = role_users
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
struct ModuleSwitch {
    id: u32,
    name: String,
    is_on: u8,
    des: Option<String>,
}
/// 功能模块的开关
#[get("/manage/system/module/switch_list")]
pub async fn manage_system_module_switch_list(
    _super_mana: AuthSuperMana,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let list: Vec<ModuleSwitch> = my_run_vec(
        &mut conn,
        myfind!("sys_module_switch", {
            p0: ["is_del", "=", 0],
            r: "p0",
            select: "id,name,is_on,des",
        }),
    )?;
    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Clone, Deserialize)]
struct ModuleSwitchChange {
    id: u32,
    is_on: u8,
}
/// 修改功能模块状态
#[put("/manage/system/module/switch_change")]
pub async fn manage_system_module_switch_change(
    _super_mana: AuthSuperMana,
    params: web::Json<ModuleSwitchChange>,
) -> Result<impl Responder> {
    if params.is_on != 1 && params.is_on != 0 {
        return Ok(web::Json(Res::fail("is_on 参数错误")));
    };
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("sys_module_switch", params.id, {
            "is_on": params.is_on,
        }),
    )?;
    Ok(web::Json(Res::success("")))
}
