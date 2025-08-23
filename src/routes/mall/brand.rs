use actix_web::{Responder, Result, get, web};
use mysql_quick::myfind;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::types::ProductLayout;
use crate::db::{my_run_vec, mysql_conn};
use crate::routes::{BaseNumInfo, Res};
use crate::utils::files::get_file_url;

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct Brand {
    pub id: u32,
    pub brand_code: u32,
    pub brand_name: String,
    pub brand_logo: Option<String>,
    pub brand_sec_name: Option<String>,
    pub brand_des: Option<String>,
}

/// 【品牌】获取品牌选项
#[utoipa::path(
    responses((status = 200, description = "【返回：BaseNumInfo[]】", body = Res<Vec<BaseNumInfo>>))
)]
#[get("/mall/brand/options")]
pub async fn mall_brand_options() -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    #[derive(Serialize, Deserialize)]
    struct BrandItem {
        brand_code: u32,
        brand_name: String,
    }
    // 获取品牌列表
    let sql = myfind!("brd_brand", {
        p0: ["is_del", "=", 0],
        p1: ["status", "=", 2],
        r: "p0 && p1",
    });
    let list: Vec<BrandItem> = my_run_vec(&mut conn, sql)?;

    Ok(web::Json(Res::success(
        list.iter()
            .map(|x| BaseNumInfo {
                label: x.brand_name.clone(),
                value: x.brand_code,
            })
            .collect::<Vec<BaseNumInfo>>(),
    )))
}

/// 【品牌】某个品牌产品
#[utoipa::path(
    responses((status = 200, description = "【返回：BaseNumInfo[]】", body = Res<Vec<BaseNumInfo>>)),
    params(("brand_code", description="品牌编号"))
)]
#[get("/mall/brand/products/{brand_code}")]
pub async fn mall_brand_products(path: web::Path<String>) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let brand_code: u32 = path.to_owned().parse().unwrap();

    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct ProductGet {
        product_sn: u32,
        product_name: String,
    }
    let list: Vec<ProductGet> = my_run_vec(
        &mut conn,
        myfind!("spu_product", {
            p0: ["status", "=", 2],
            p1: ["is_del", "=", 0],
            p2: ["brand_code", "=", brand_code],
            r: "p0 && p1 && p2",
            order_by: "-sort,-created_at",
            select: "product_sn,product_name",
        }),
    )?;
    let list = list
        .iter()
        .map(|x| BaseNumInfo {
            label: x.product_name.clone(),
            value: x.product_sn,
        })
        .collect::<Vec<_>>();
    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct BrandProductItem {
    id: u32,
    product_sn: u32,
    product_name: String,
    product_sec_name: Option<String>,
    product_cover_img: String,
    product_layout: ProductLayout,
}
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct BrandProduct {
    id: u32,
    name: String,
    img: Option<String>,
    product_list: Vec<BrandProductItem>,
}
/// 【品牌】所有品牌产品
#[utoipa::path(
    responses((status = 200, description = "【返回：BrandProduct[]】", body = Res<Vec<BrandProduct>>))
)]
#[get("/mall/brand/products_all")]
pub async fn mall_brand_products_all() -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    // 获取所有可用品牌
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct BrandGet {
        id: u32,
        brand_code: u32,
        brand_name: String,
        brand_logo: Option<String>,
        brand_sec_name: Option<String>,
        brand_des: Option<String>,
    }
    let brand_list: Vec<BrandGet> = my_run_vec(
        &mut conn,
        myfind!("brd_brand", {
            p0: ["status", "=", 2],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            order_by: "-sort,-created_at",
            select: "id,brand_code,brand_name,brand_logo,brand_sec_name,brand_des",
        }),
    )?;
    let brand_ids = brand_list
        .iter()
        .map(|x| x.brand_code.to_string())
        .collect::<Vec<_>>();

    // 查寻，所有品牌code的产品
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct ProductGet {
        id: u32,
        product_sn: u32,
        brand_code: u32,
        product_name: String,
        product_cover_img: String,
        product_sec_name: Option<String>,
        product_layout: Option<String>,
    }
    let prd_list: Vec<ProductGet> = my_run_vec(
        &mut conn,
        myfind!("spu_product", {
            p0: ["status", "=", 2],
            p1: ["is_del", "=", 0],
            p2: ["brand_code", "in", brand_ids.join(",")],
            r: "p0 && p1 && p2",
            order_by: "-sort,-created_at",
            select: "id,product_sn,brand_code,product_name,product_sec_name,product_cover_img,product_layout",
        }),
    )?;

    // 将品牌和产品关联起来
    let list = brand_list
        .iter()
        .map(|x| {
            let mut pd: Vec<BrandProductItem> = vec![];
            prd_list.iter().for_each(|p| {
                if p.brand_code == x.brand_code {
                    pd.push(BrandProductItem {
                        id: p.id,
                        product_sn: p.product_sn,
                        product_name: p.product_name.clone(),
                        product_sec_name: p.product_sec_name.clone(),
                        product_cover_img: get_file_url(Some(&p.product_cover_img))
                            .unwrap_or("".to_string()),
                        product_layout: if let Some(pl) = p.product_layout.clone() {
                            pl.into()
                        } else {
                            "".into()
                        },
                    });
                }
            });
            BrandProduct {
                id: x.id,
                name: x.brand_name.clone(),
                img: get_file_url(x.brand_logo.clone()),
                product_list: pd,
            }
        })
        .collect::<Vec<_>>();

    Ok(web::Json(Res::success(list)))
}
