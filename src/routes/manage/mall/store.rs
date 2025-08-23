use actix_web::{Responder, Result, get, post, put, web};
use mysql_quick::{MysqlQuickCount, mycount, myfind, myset, myupdate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::PageData;
use crate::common::STORE_START_CODE;
use crate::common::types::{NormalStatus, OssBucket};
use crate::routes::Res;
use crate::utils::files::{get_file_url, get_file_urls, get_path_from_url, get_path_from_urls};
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::AuthMana,
};

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct StoreAddrInfo {
    detail: String,
    lat: f64,
    lng: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct StoreAdd {
    code: u32,
    name: String,
    com_store_type: Option<u32>,
    des: Option<String>,
    cover_img: Option<String>,
    imgs: Option<Vec<String>>,
    addr_info: Option<StoreAddrInfo>,
    province_info: Option<[String; 3]>,
    html: Option<String>,
}
/// 【公司店铺】新增或更新
#[utoipa::path(
    request_body = StoreAdd,
    responses((status = 200, description = "【请求：StoreAdd】【返回：String】有code表示更新，为0表示新增", body = String)),
)]
#[post("/manage/mall/store/add")]
pub async fn manage_mall_store_add(
    _mana: AuthMana,
    params: web::Json<StoreAdd>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    let mut code_max = STORE_START_CODE;
    if params.code >= STORE_START_CODE {
        code_max = params.code;
    } else {
        #[derive(Deserialize)]
        struct LastMax {
            last: Option<u32>,
        }
        let last_max: Vec<LastMax> = my_run_vec(
            &mut conn,
            "select Max(code) as last from com_store".to_string(),
        )?;
        if last_max.len() > 0 {
            if let Some(mx) = last_max[0].last {
                if mx >= STORE_START_CODE {
                    code_max = mx + 1;
                }
            }
        }
    }
    let sql;
    let temp_com_store_type = if let Some(t) = params.com_store_type {
        t.to_string()
    } else {
        "null".to_string()
    };
    let temp_des = if let Some(d) = &params.des { d } else { "null" };
    let temp_cover_img = if let Some(c) = &params.cover_img {
        get_path_from_url(c, &OssBucket::EobFiles)
    } else {
        "null".to_string()
    };
    let temp_imgs = if let Some(d) = &params.imgs {
        get_path_from_urls(d, &OssBucket::EobFiles).join(",")
    } else {
        "null".to_string()
    };
    let temp_province;
    let temp_city;
    let temp_area;
    if let Some(a) = &params.province_info {
        temp_province = a[0].clone();
        temp_city = a[1].clone();
        temp_area = a[2].clone();
    } else {
        temp_province = "null".to_string();
        temp_city = "null".to_string();
        temp_area = "null".to_string();
    }
    let temp_addr_detail;
    let temp_lat;
    let temp_lng;
    if let Some(addr) = &params.addr_info {
        temp_addr_detail = addr.detail.clone();
        temp_lat = addr.lat.to_string();
        temp_lng = addr.lng.to_string();
    } else {
        temp_addr_detail = "null".to_string();
        temp_lat = "null".to_string();
        temp_lng = "null".to_string();
    }
    if params.code >= STORE_START_CODE {
        // 有编号，则更新
        sql = myupdate!("com_store", {"code": code_max}, {
            "name": &params.name,
            "com_store_type": temp_com_store_type,
            "des": temp_des,
            "html": &params.html,
            "cover_img": &temp_cover_img,
            "imgs": &temp_imgs,
            "province": &temp_province,
            "city": &temp_city,
            "area": &temp_area,
            "addr_detail": &temp_addr_detail,
            "lat": &temp_lat,
            "lng": &temp_lng,
        })
    } else {
        // 新增
        sql = myset!("com_store", {
            "code": code_max,
            "name": &params.name,
            "com_store_type": temp_com_store_type,
            "des": temp_des,
            "html": &params.html,
            "cover_img": &temp_cover_img,
            "imgs": &temp_imgs,
            "province": &temp_province,
            "city": &temp_city,
            "area": &temp_area,
            "addr_detail": &temp_addr_detail,
            "lat": &temp_lat,
            "lng": &temp_lng,
        })
    }
    my_run_drop(&mut conn, sql)?;

    Ok(web::Json(Res::success("")))
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct StoreInfo {
    id: u64,
    code: u32,
    name: String,
    des: Option<String>,
    cover_img: Option<String>,
    imgs: Option<Vec<String>>,
    html: Option<String>,
    province: Option<String>,
    city: Option<String>,
    area: Option<String>,
    addr_detail: Option<String>,
    lat: Option<f64>,
    lng: Option<f64>,
    com_store_type: Option<u8>,
    status: i8,
    created_at: String,
}
/// 【公司店铺】公司店铺列表
#[utoipa::path(
    responses((status = 200, description = "【返回：StoreInfo[]】", body = Res<PageData<StoreInfo>>)),
    params(("page", description="页码"), ("limit", description="每页数量"))
)]
#[get("/manage/mall/store/list/{page}/{limit}")]
pub async fn manage_mall_store_list(
    _mana: AuthMana,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (page, limit) = query.to_owned();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("com_store", {
            p0: ["is_del", "=", 0],
            r: "p0",
        }),
    )?;
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct StoreGet {
        id: u64,
        code: u32,
        name: String,
        des: Option<String>,
        cover_img: Option<String>,
        imgs: Option<String>,
        html: Option<String>,
        province: Option<String>,
        city: Option<String>,
        area: Option<String>,
        addr_detail: Option<String>,
        lat: Option<f64>,
        lng: Option<f64>,
        com_store_type: Option<u8>,
        status: i8,
        created_at: String,
    }
    let list: Vec<StoreGet> = my_run_vec(
        &mut conn,
        myfind!("com_store", {
            p0: ["is_del", "=", 0],
            r: "p0",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id,code,name,des,cover_img,imgs,html,province,city,area,addr_detail,lat,lng,com_store_type,status,created_at",
        }),
    )?;

    let list: Vec<StoreInfo> = list
        .into_iter()
        .map(|x| {
            let temp_cover = get_file_url(x.cover_img);
            let temp_imgs = if let Some(p) = x.imgs {
                Some(get_file_urls(Some(&p)))
            } else {
                None
            };
            return StoreInfo {
                id: x.id,
                code: x.code,
                name: x.name,
                des: x.des,
                cover_img: temp_cover,
                imgs: temp_imgs,
                html: x.html,
                province: x.province,
                city: x.city,
                area: x.area,
                addr_detail: x.addr_detail,
                lat: x.lat,
                lng: x.lng,
                com_store_type: x.com_store_type,
                status: x.status,
                created_at: x.created_at,
            };
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize)]
pub struct StoreSearchInfo {
    label: String,
    value: u32,
}
/// 搜索页面
#[get("/manage/mall/store/search/{keyword}")]
pub async fn manage_mall_store_search(
    _mana: AuthMana,
    query: web::Path<String>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let keyword = query.to_owned();
    #[derive(Deserialize)]
    struct SearchGet {
        code: u32,
        name: String,
    }
    let list: Vec<SearchGet> = my_run_vec(
        &mut conn,
        myfind!("com_store", {
            p0: ["name", "like", format!("%{keyword}%")],
            p1: ["is_del", "=", 0],
            p2: ["code", "=", &keyword],
            r: "(p0 || p2) && p1",
        }),
    )?;
    let list: Vec<StoreSearchInfo> = list
        .into_iter()
        .map(|x| StoreSearchInfo {
            label: x.name,
            value: x.code,
        })
        .collect();
    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoreDel {
    code: u32,
}
/// 删除
#[put("/manage/mall/store/del")]
pub async fn manage_mall_store_del(
    _mana: AuthMana,
    params: web::Json<StoreDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("com_store", {"code": params.code}, {"is_del": 1}),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoreStatus {
    code: u32,
    status: i8,
}
/// 修改状态
#[put("/manage/mall/store/status")]
pub async fn manage_mall_store_status(
    _mana: AuthMana,
    params: web::Json<StoreStatus>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("com_store", {"code": params.code}, {
            "status": &params.status
        }),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoreEmployeeAdd {
    id: u32,
    com_store_code: u32,
    uid: u64,
}
/// 新增或更新
#[post("/manage/mall/store/employee/add")]
pub async fn manage_mall_store_employee_add(
    _mana: AuthMana,
    params: web::Json<StoreEmployeeAdd>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let sql;
    if params.id > 0 {
        // 有id，则更新
        sql = myupdate!("com_store_employee", {"id": params.id}, {
            "uid": params.uid,
            "com_store_code": params.com_store_code,
        })
    } else {
        // 查找是否已有
        #[derive(Deserialize)]
        struct EmployeeGet {
            id: u32,
        }
        let data: Vec<EmployeeGet> = my_run_vec(
            &mut conn,
            myfind!("com_store_employee", {
                p0: ["uid", "=", params.uid],
                p1: ["com_store_code", "=", params.com_store_code],
                r: "p0 && p1",
            }),
        )?;
        if data.len() > 0 {
            // 将之前的状态修改了
            sql = myupdate!("com_store_employee", {"id": data[0].id}, {
                "is_del": 0,
                "status": NormalStatus::UnderReview as i8,
            })
        } else {
            // 新增
            sql = myset!("com_store_employee", {
                "uid": params.uid,
                "com_store_code": params.com_store_code,
            })
        }
    }
    my_run_drop(&mut conn, sql)?;

    Ok(web::Json(Res::success("添加成功")))
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct StoreEmployeeInfo {
    id: u32,
    com_store_code: u32,
    com_store_name: String,
    uid: u64,
    nickname: String,
    avatar_url: Option<String>,
    status: i8,
    created_at: String,
}
/// 员工列表
#[get("/manage/mall/store/employee/list/{com_store_code}/{page}/{limit}")]
pub async fn manage_mall_store_employee_list(
    _mana: AuthMana,
    query: web::Path<(String, String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (com_store_code, page, limit) = query.to_owned();
    let com_store_code: u32 = com_store_code.to_owned().parse().unwrap();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("com_store_employee", {
            p0: ["is_del", "=", 0],
            p1: ["com_store_code", "=", com_store_code],
            r: "p0 && p1",
        }),
    )?;
    #[derive(Serialize, Deserialize, Clone)]
    pub struct StoreEmployeeGet {
        id: u32,
        com_store_code: u32,
        com_store_name: String,
        uid: u64,
        nickname: String,
        avatar_url: Option<String>,
        status: i8,
        created_at: String,
    }
    let list: Vec<StoreEmployeeGet> = my_run_vec(
        &mut conn,
        myfind!("com_store_employee", {
            j0: ["com_store_code", "inner", "com_store.code"],
            j1: ["uid", "inner", "usr_silent.id"],
            p0: ["is_del", "=", 0],
            p1: ["com_store_code", "=", com_store_code],
            r: "p0 && p1",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id, com_store_code, com_store.name as com_store_name, uid, usr_silent.nickname, usr_silent.avatar_url, status, created_at",
        }),
    )?;

    let list: Vec<StoreEmployeeInfo> = list
        .into_iter()
        .map(|x| {
            return StoreEmployeeInfo {
                id: x.id,
                com_store_code: x.com_store_code,
                com_store_name: x.com_store_name,
                uid: x.uid,
                nickname: x.nickname,
                avatar_url: get_file_url(x.avatar_url),
                status: x.status,
                created_at: x.created_at,
            };
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoreEmployeeDel {
    id: u32,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoreEmployeeStatus {
    id: u32,
    status: i8,
}
/// 修改状态
#[put("/manage/mall/store/employee/status")]
pub async fn manage_mall_store_employee_status(
    _mana: AuthMana,
    params: web::Json<StoreEmployeeStatus>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("com_store_employee", {"id": params.id}, {
            "status": &params.status
        }),
    )?;
    Ok(web::Json(Res::success("修改成功")))
}

/// 删除
#[put("/manage/mall/store/employee/del")]
pub async fn manage_mall_store_employee_del(
    _mana: AuthMana,
    params: web::Json<StoreEmployeeDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("com_store_employee", {"id": params.id}, {"is_del": 1}),
    )?;
    Ok(web::Json(Res::success("删除成功")))
}
