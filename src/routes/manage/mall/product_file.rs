use actix_web::{Responder, Result, get, post, put, web};
use mysql_quick::{MysqlQuickCount, mycount, myfind, myset, myupdate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::PageData;
use crate::common::types::OssBucket;
use crate::routes::Res;
use crate::utils::files::{get_file_url, get_path_from_url};
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::AuthMana,
};

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ProductFileAdd {
    /// 文件id
    id: u32,
    /// 产品编号
    product_sn: u32,
    /// 名称
    title: String,
    /// 文件地址
    file_url: Option<String>,
}
/// 【产品文件】新增或更新
#[utoipa::path(
    request_body = ProductFileAdd,
    responses((status = 200, description = "【请求：ProductFileAdd】【返回：String】有 id 表示更新，为0表示新增", body = String)),
)]
#[post("/manage/mall/product_file/add")]
pub async fn manage_mall_product_file_add(
    _mana: AuthMana,
    params: web::Json<ProductFileAdd>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let sql;

    let temp_file = if let Some(c) = &params.file_url {
        get_path_from_url(c, &OssBucket::EobFiles)
    } else {
        "null".to_string()
    };

    if params.id > 0 {
        // 有编号，则更新
        sql = myupdate!("spu_product_file", {"id": params.id}, {
            "product_sn": &params.product_sn,
            "title": &params.title,
            "file_url": temp_file,
        })
    } else {
        // 新增
        sql = myset!("spu_product_file", {
            "product_sn": &params.product_sn,
            "title": &params.title,
            "file_url": temp_file,
        })
    }
    my_run_drop(&mut conn, sql)?;

    Ok(web::Json(Res::success("")))
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ProductFileInfo {
    id: u64,
    product_sn: u32,
    /// 产品名
    product_name: String,
    /// 文件名
    title: String,
    /// 文件地址
    file_url: Option<String>,
    status: i8,
    created_at: String,
}
/// 【产品文件】列表
#[utoipa::path(
    responses((status = 200, description = "【返回：ProductFileInfo[]】", body = Res<PageData<ProductFileInfo>>)),
    params(("page", description="页码"), ("limit", description="每页数量"))
)]
#[get("/manage/mall/product_file/list/{page}/{limit}")]
pub async fn manage_mall_product_file_list(
    _mana: AuthMana,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (page, limit) = query.to_owned();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("spu_product_file", {
            p0: ["is_del", "=", 0],
            r: "p0",
        }),
    )?;
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ProductFileGet {
        id: u64,
        product_sn: u32,
        product_name: String,
        title: String,
        file_url: Option<String>,
        status: i8,
        created_at: String,
    }
    let list: Vec<ProductFileGet> = my_run_vec(
        &mut conn,
        myfind!("spu_product_file", {
            j0: ["product_sn", "inner", "spu_product.product_sn"],
            p0: ["is_del", "=", 0],
            r: "p0",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id,product_sn,spu_product.product_name,title,file_url,status,created_at",
        }),
    )?;

    let list: Vec<ProductFileInfo> = list
        .into_iter()
        .map(|x| {
            let temp_file = get_file_url(x.file_url);
            ProductFileInfo {
                id: x.id,
                product_sn: x.product_sn,
                product_name: x.product_name,
                title: x.title,
                file_url: temp_file,
                status: x.status,
                created_at: x.created_at,
            }
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ProductFileDel {
    id: u32,
}
/// 【产品文件】删除
#[utoipa::path(
    request_body = ProductFileDel,
    responses((status = 200, description = "【请求：ProductFileDel】【返回：String】id 删除的id", body = String)),
)]
#[put("/manage/mall/product_file/del")]
pub async fn manage_mall_product_file_del(
    _mana: AuthMana,
    params: web::Json<ProductFileDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("spu_product_file", {"id": params.id}, {"is_del": 1}),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ProductFileStatus {
    id: u32,
    status: i8,
}
/// 【产品文件】状态修改
#[utoipa::path(
    request_body = ProductFileStatus,
    responses((status = 200, description = "【请求：ProductFileStatus】【返回：String】id 更新的编号。status：2已上线，1审核中，0未通过，3已下线", body = String)),
)]
#[put("/manage/mall/product_file/status")]
pub async fn manage_mall_product_file_status(
    _mana: AuthMana,
    params: web::Json<ProductFileStatus>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("spu_product_file", {"id": params.id}, {
            "status": &params.status
        }),
    )?;
    Ok(web::Json(Res::success("成功")))
}
