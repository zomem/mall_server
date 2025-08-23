use actix_web::{Responder, Result, get, web};
use mysql_quick::myfind;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::types::ProductLayout;
use crate::db::{my_run_vec, mysql_conn};
use crate::routes::{PdCat, Res};
use crate::utils::files::get_file_url;

#[derive(Serialize, Clone, Debug, ToSchema)]
pub struct ProductCatItem {
    id: u32,
    icon: Option<String>,
    name: String,
    children: Vec<ProductCatItem>,
}
/// 【产品分类】分类列表
#[utoipa::path(
    responses((status = 200, description = "【返回：ProductCatItem[]】", body = Vec<ProductCatItem>)),
)]
#[get("/mall/cat/list")]
pub async fn mall_cat_list() -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let mut pd_list = vec![];

    let sql = myfind!("spu_cat", {
        p0: ["is_del", "=", 0],
        r: "p0",
    });
    let list: Vec<PdCat> = my_run_vec(&mut conn, sql)?;
    // 一级
    for i in 0..list.len() {
        if list[i].primary_id > 0 && list[i].secondary_id == 0 && list[i].tertiary_id == 0 {
            pd_list.push(ProductCatItem {
                id: list[i].primary_id,
                icon: get_file_url(list[i].icon.clone()),
                name: list[i].name.clone(),
                children: vec![],
            });
        }
    }
    // 二级
    for i in 0..list.len() {
        if list[i].primary_id != 0 && list[i].secondary_id > 0 && list[i].tertiary_id == 0 {
            for j in 0..pd_list.len() {
                if pd_list[j].id == list[i].primary_id {
                    pd_list[j].children.push(ProductCatItem {
                        id: list[i].secondary_id,
                        icon: get_file_url(list[i].icon.clone()),
                        name: list[i].name.clone(),
                        children: vec![],
                    });
                }
            }
        }
    }
    // 三级
    for i in 0..list.len() {
        if list[i].primary_id != 0 && list[i].secondary_id != 0 && list[i].tertiary_id > 0 {
            for j in 0..pd_list.len() {
                if pd_list[j].id == list[i].primary_id {
                    for k in 0..pd_list[j].children.len() {
                        if pd_list[j].children[k].id == list[i].secondary_id {
                            pd_list[j].children[k].children.push(ProductCatItem {
                                id: list[i].tertiary_id,
                                icon: get_file_url(list[i].icon.clone()),
                                name: list[i].name.clone(),
                                children: vec![],
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(web::Json(Res::success(pd_list)))
}

/// 【产品分类】三级分类
#[utoipa::path(
    responses((status = 200, description = "【返回：ProductCatItem[]】", body = Vec<ProductCatItem>)),
)]
#[get("/mall/cat/tertiary_of/{secondary_id}")]
pub async fn mall_cat_tertiary_of(query: web::Path<String>) -> Result<impl Responder> {
    let secondary_id = query.parse::<u32>().unwrap();
    let mut conn = mysql_conn()?;
    let mut pd_list = vec![];

    let sql = myfind!("spu_cat", {
        p0: ["is_del", "=", 0],
        p1: ["secondary_id", "=", secondary_id],
        r: "p0 && p1",
    });
    let list: Vec<PdCat> = my_run_vec(&mut conn, sql)?;
    // 三级
    for i in 0..list.len() {
        if list[i].tertiary_id > 0 {
            pd_list.push(ProductCatItem {
                id: list[i].tertiary_id,
                icon: get_file_url(list[i].icon.clone()),
                name: list[i].name.clone(),
                children: vec![],
            });
        }
    }
    Ok(web::Json(Res::success(pd_list)))
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct CatProductItem {
    id: u32,
    product_sn: u32,
    product_name: String,
    product_sec_name: Option<String>,
    product_cover_img: String,
    product_layout: ProductLayout,
}
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct CatProduct {
    id: u32,
    name: String,
    img: Option<String>,
    product_list: Vec<CatProductItem>,
}
/// 【产品分类】特定接口
#[utoipa::path(
    responses((status = 200, description = "【返回：CatProduct[]】", body = Res<Vec<CatProduct>>))
)]
#[get("/mall/cat/products_all")]
pub async fn mall_cat_products_all() -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    // 获取所有可用三级分类
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct CatGet {
        id: u32,
        tertiary_id: u32,
        name: String,
        icon: Option<String>,
    }
    let cat_list: Vec<CatGet> = my_run_vec(
        &mut conn,
        myfind!("spu_cat", {
            p0: ["tertiary_id", ">", 0],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            order_by: "-sort,-created_at",
            select: "id,tertiary_id,name,icon",
        }),
    )?;
    let cat_ids = cat_list
        .iter()
        .map(|x| x.tertiary_id.to_string())
        .collect::<Vec<_>>();

    // 查寻，所有品牌code的产品
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct CatProductGet {
        id: u32,
        product_sn: u32,
        tertiary_id: u32,
        product_name: String,
        product_cover_img: String,
        product_sec_name: Option<String>,
        product_layout: Option<String>,
    }
    let prd_list: Vec<CatProductGet> = my_run_vec(
        &mut conn,
        myfind!("spu_product_cat", {
            j0: ["product_sn", "inner", "spu_product.product_sn"],
            p1: ["is_del", "=", 0],
            p2: ["tertiary_id", "in", cat_ids.join(",")],
            r: "p1 && p2",
            order_by: "-spu_product.sort,-created_at",
            select: "id,product_sn,tertiary_id,spu_product.product_name,spu_product.product_sec_name,spu_product.product_cover_img,spu_product.product_layout",
        }),
    )?;

    // 将品牌和产品关联起来
    let list = cat_list
        .iter()
        .map(|x| {
            let mut pd: Vec<CatProductItem> = vec![];
            prd_list.iter().for_each(|p| {
                if p.tertiary_id == x.tertiary_id {
                    pd.push(CatProductItem {
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
            CatProduct {
                id: x.id,
                name: x.name.clone(),
                img: get_file_url(x.icon.clone()),
                product_list: pd,
            }
        })
        .collect::<Vec<_>>();

    Ok(web::Json(Res::success(list)))
}
