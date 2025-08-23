use actix_web::{Responder, Result, get, post, put, web};
use mysql_quick::{MysqlQuickCount, mycount, myfind, myset, myupdate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::PageData;
use crate::common::BRAND_START_CODE;
use crate::common::types::OssBucket;
use crate::routes::Res;
use crate::utils::files::{get_file_url, get_path_from_url};
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::AuthMana,
};

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct BrandAdd {
    /// 品牌编号
    brand_code: u32,
    /// 品牌名称
    brand_name: String,
    /// 品牌第二个名称（英文名）
    brand_sec_name: Option<String>,
    /// 排序权重（越大越靠前）
    sort: Option<i32>,
    /// 品牌logo
    brand_logo: Option<String>,
    /// 品牌描述
    brand_des: Option<String>,
}
/// 【品牌】新增或更新
#[utoipa::path(
    request_body = BrandAdd,
    responses((status = 200, description = "【请求：BrandAdd】【返回：String】有 brand_code 表示更新，为0表示新增", body = String)),
)]
#[post("/manage/mall/brand/add")]
pub async fn manage_mall_brand_add(
    _mana: AuthMana,
    params: web::Json<BrandAdd>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    let sort = if let Some(s) = params.sort { s } else { 0 };
    let mut code_max = BRAND_START_CODE;
    if params.brand_code >= BRAND_START_CODE {
        code_max = params.brand_code;
    } else {
        #[derive(Deserialize)]
        struct LastMax {
            last: Option<u32>,
        }
        let last_max: Vec<LastMax> = my_run_vec(
            &mut conn,
            "select Max(brand_code) as last from brd_brand".to_string(),
        )?;
        if last_max.len() > 0 {
            if let Some(mx) = last_max[0].last {
                if mx >= BRAND_START_CODE {
                    code_max = mx + 1;
                }
            }
        }
    }
    let sql;

    let temp_logo = if let Some(c) = &params.brand_logo {
        get_path_from_url(c, &OssBucket::EobFiles)
    } else {
        "null".to_string()
    };

    if params.brand_code >= BRAND_START_CODE {
        // 有编号，则更新
        sql = myupdate!("brd_brand", {"brand_code": code_max}, {
            "brand_name": &params.brand_name,
            "brand_sec_name": &params.brand_sec_name,
            "sort": sort,
            "brand_logo": &temp_logo,
            "brand_des": &params.brand_des,
        })
    } else {
        // 新增
        sql = myset!("brd_brand", {
            "brand_code": code_max,
            "brand_name": &params.brand_name,
            "brand_sec_name": &params.brand_sec_name,
            "sort": sort,
            "brand_logo": &temp_logo,
            "brand_des": &params.brand_des,
        })
    }
    my_run_drop(&mut conn, sql)?;

    Ok(web::Json(Res::success("")))
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct BrandInfo {
    id: u64,
    brand_code: u32,
    brand_name: String,
    brand_sec_name: Option<String>,
    sort: i32,
    brand_logo: Option<String>,
    brand_des: Option<String>,
    status: i8,
    created_at: String,
}
/// 【品牌】品牌列表
#[utoipa::path(
    responses((status = 200, description = "【返回：BrandInfo[]】", body = Res<PageData<BrandInfo>>)),
    params(("page", description="页码"), ("limit", description="每页数量"))
)]
#[get("/manage/mall/brand/list/{page}/{limit}")]
pub async fn manage_mall_brand_list(
    _mana: AuthMana,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (page, limit) = query.to_owned();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("brd_brand", {
            p0: ["is_del", "=", 0],
            r: "p0",
        }),
    )?;
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct BrandGet {
        id: u64,
        brand_code: u32,
        brand_name: String,
        brand_sec_name: Option<String>,
        sort: i32,
        brand_logo: Option<String>,
        brand_des: Option<String>,
        status: i8,
        created_at: String,
    }
    let list: Vec<BrandGet> = my_run_vec(
        &mut conn,
        myfind!("brd_brand", {
            p0: ["is_del", "=", 0],
            r: "p0",
            page: page,
            limit: limit,
            order_by: "-sort,-created_at",
            select: "id,brand_code,brand_name,brand_sec_name,sort,brand_des,brand_logo,status,created_at",
        }),
    )?;

    let list: Vec<BrandInfo> = list
        .into_iter()
        .map(|x| {
            let temp_cover = get_file_url(x.brand_logo);
            return BrandInfo {
                id: x.id,
                brand_code: x.brand_code,
                brand_name: x.brand_name,
                brand_sec_name: x.brand_sec_name,
                sort: x.sort,
                brand_logo: temp_cover,
                brand_des: x.brand_des,
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

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct BrandSearchInfo {
    label: String,
    value: u32,
}
/// 【品牌】品牌搜索
#[utoipa::path(
    responses((status = 200, description = "【返回：BrandSearchInfo[]】", body = Res<Vec<BrandSearchInfo>>)),
    params(("keyword", description="搜索关键字"))
)]
#[get("/manage/mall/brand/search/{keyword}")]
pub async fn manage_mall_brand_search(
    _mana: AuthMana,
    query: web::Path<String>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let keyword = query.to_owned();
    #[derive(Deserialize)]
    struct SearchGet {
        brand_code: u32,
        brand_name: String,
    }
    let list: Vec<SearchGet> = my_run_vec(
        &mut conn,
        myfind!("brd_brand", {
            p0: ["brand_name", "like", format!("%{keyword}%")],
            p1: ["is_del", "=", 0],
            p2: ["brand_code", "=", &keyword],
            r: "(p0 || p2) && p1",
        }),
    )?;
    let list: Vec<BrandSearchInfo> = list
        .into_iter()
        .map(|x| BrandSearchInfo {
            label: x.brand_name,
            value: x.brand_code,
        })
        .collect();
    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct BrandDel {
    brand_code: u32,
}
/// 【品牌】删除
#[utoipa::path(
    request_body = BrandDel,
    responses((status = 200, description = "【请求：BrandDel】【返回：String】brand_code 删除的编号", body = String)),
)]
#[put("/manage/mall/brand/del")]
pub async fn manage_mall_brand_del(
    _mana: AuthMana,
    params: web::Json<BrandDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("brd_brand", {"brand_code": params.brand_code}, {"is_del": 1}),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct BrandStatus {
    brand_code: u32,
    status: i8,
}
/// 【品牌】状态修改
#[utoipa::path(
    request_body = BrandStatus,
    responses((status = 200, description = "【请求：BrandStatus】【返回：String】brand_code 更新的编号。status：2已上线，1审核中，0未通过，3已下线", body = String)),
)]
#[put("/manage/mall/brand/status")]
pub async fn manage_mall_brand_status(
    _mana: AuthMana,
    params: web::Json<BrandStatus>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("brd_brand", {"brand_code": params.brand_code}, {
            "status": &params.status
        }),
    )?;
    Ok(web::Json(Res::success("成功")))
}
