use actix_web::{Responder, Result, get, web};
use mysql_quick::myfind;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::common::types::{NormalStatus, ProductLayout};
use crate::db::{my_run_vec, mysql_conn};
use crate::routes::Res;
use crate::utils::files::get_file_url;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct ProductGroupItem {
    id: u32,
    product_sn: u32,
    product_name: String,
    /// 产品英文名
    product_sec_name: Option<String>,
    /// 产品描述
    product_des: Option<String>,
    /// 封面图
    product_cover_img: String,
    /// 布局
    product_layout: ProductLayout,
}
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct ProductGroup {
    id: u32,
    /// 分类/品牌名
    name: String,
    /// 分类/品牌 第二个名，英文名
    sec_name: Option<String>,
    /// 分类/品牌 图片
    img: Option<String>,
    /// 分类/品牌 下面的产品列表
    product_list: Vec<ProductGroupItem>,
}
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct ProductGroupAll {
    /// 品牌产品列表
    brand_product_list: Vec<ProductGroup>,
    /// 分类产品列表
    cat_product_list: Vec<ProductGroup>,
}
#[derive(Serialize, Deserialize, Debug, IntoParams, ToSchema)]
pub struct ProductGroupSearch {
    /// 搜索关键词
    search: Option<String>,
}
/// 【产品组合】各类产品
#[utoipa::path(
    responses((status = 200, description = "【返回：ProductGroupAll】", body = Res<ProductGroupAll>)),
    params(ProductGroupSearch),
)]
#[get("/mall/product_group/all")]
pub async fn mall_product_group_all(
    query: web::Query<ProductGroupSearch>,
) -> Result<impl Responder> {
    let mut keyword = "".to_string();
    let mut conn = mysql_conn()?;

    if let Some(kw) = query.0.search {
        keyword = kw;
    }

    // 查寻，所有品牌code的产品
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct SearchProductGet {
        id: u32,
        product_sn: u32,
        brand_code: u32,
        tertiary_id: u32,
        product_name: String,
        product_cover_img: String,
        product_sec_name: Option<String>,
        product_des: Option<String>,
        product_layout: Option<String>,
    }
    // 如果有搜索，要对产品进行单独搜索
    let mut search_prd_list: Vec<SearchProductGet> = vec![];
    if !keyword.is_empty() {
        search_prd_list = my_run_vec(
            &mut conn,
            myfind!("spu_product", {
                j0: ["product_sn", "right", "spu_product_cat.product_sn"],
                p0: ["status", "=", 2],
                p1: ["is_del", "=", 0],
                p2: ["spu_product_cat.is_del", "=", 0],
                p7: ["product_name", "like", format!("%{}%", keyword)],
                r: "p0 && p1 && p2 && p7",
                order_by: "-sort,-created_at",
                select: "id,product_sn,brand_code,spu_product_cat.tertiary_id,product_name,product_des,product_sec_name,product_cover_img,product_layout",
            }),
        )?;
    }

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
        .filter(|x| {
            if keyword.is_empty() {
                true
            } else {
                x.brand_name.contains(&keyword)
            }
        })
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
        product_des: Option<String>,
        product_layout: Option<String>,
    }
    let prd_list: Vec<ProductGet> = my_run_vec(
        &mut conn,
        myfind!("spu_product", {
            p0: ["status", "=", 2],
            p1: ["is_del", "=", 0],
            p2: ["brand_code", "in", brand_ids.join(",")],
            p7: ["product_name", "like", &keyword],
            r: "p0 && p1 && p2",
            order_by: "-sort,-created_at",
            select: "id,product_sn,brand_code,product_name,product_des,product_sec_name,product_cover_img,product_layout",
        }),
    )?;

    // 将品牌和产品关联起来
    let list_brand = brand_list
        .iter()
        .map(|x| {
            let mut pd: Vec<ProductGroupItem> = vec![];
            prd_list.iter().for_each(|p| {
                if p.brand_code == x.brand_code {
                    pd.push(ProductGroupItem {
                        id: p.id,
                        product_sn: p.product_sn,
                        product_name: p.product_name.clone(),
                        product_sec_name: p.product_sec_name.clone(),
                        product_des: p.product_des.clone(),
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
            search_prd_list.iter().for_each(|p| {
                if p.brand_code == x.brand_code {
                    pd.push(ProductGroupItem {
                        id: p.id,
                        product_sn: p.product_sn,
                        product_name: p.product_name.clone(),
                        product_sec_name: p.product_sec_name.clone(),
                        product_des: p.product_des.clone(),
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
            ProductGroup {
                id: x.id,
                name: x.brand_name.clone(),
                sec_name: x.brand_sec_name.clone(),
                img: get_file_url(x.brand_logo.clone()),
                product_list: pd,
            }
        })
        .collect::<Vec<_>>();

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
        .filter(|x| {
            if keyword.is_empty() {
                true
            } else {
                x.name.contains(&keyword)
            }
        })
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
        product_des: Option<String>,
        product_layout: Option<String>,
    }
    let prd_list: Vec<CatProductGet> = my_run_vec(
        &mut conn,
        myfind!("spu_product_cat", {
            j0: ["product_sn", "inner", "spu_product.product_sn"],
            p0: ["spu_product.status", "=", NormalStatus::Online as u8],
            p1: ["is_del", "=", 0],
            p2: ["tertiary_id", "in", cat_ids.join(",")],
            r: "p0 && p1 && p2",
            order_by: "-spu_product.sort,-created_at",
            select: "id,product_sn,tertiary_id,spu_product.product_name,spu_product.product_des,spu_product.product_sec_name,spu_product.product_cover_img,spu_product.product_layout",
        }),
    )?;

    // 将品牌和产品关联起来
    let list_cat = cat_list
        .iter()
        .map(|x| {
            let mut pd: Vec<ProductGroupItem> = vec![];
            prd_list.iter().for_each(|p| {
                if p.tertiary_id == x.tertiary_id {
                    pd.push(ProductGroupItem {
                        id: p.id,
                        product_sn: p.product_sn,
                        product_name: p.product_name.clone(),
                        product_sec_name: p.product_sec_name.clone(),
                        product_des: p.product_des.clone(),
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
            search_prd_list.iter().for_each(|p| {
                if p.tertiary_id == x.tertiary_id {
                    pd.push(ProductGroupItem {
                        id: p.id,
                        product_sn: p.product_sn,
                        product_name: p.product_name.clone(),
                        product_sec_name: p.product_sec_name.clone(),
                        product_des: p.product_des.clone(),
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
            ProductGroup {
                id: x.id,
                name: x.name.clone(),
                sec_name: None,
                img: get_file_url(x.icon.clone()),
                product_list: pd,
            }
        })
        .collect::<Vec<_>>();

    Ok(web::Json(Res::success(ProductGroupAll {
        brand_product_list: list_brand
            .into_iter()
            .filter_map(|x| {
                if x.product_list.is_empty() {
                    None
                } else {
                    Some(x)
                }
            })
            .collect::<Vec<_>>(),
        cat_product_list: list_cat
            .into_iter()
            .filter_map(|x| {
                if x.product_list.is_empty() {
                    None
                } else {
                    Some(x)
                }
            })
            .collect::<Vec<_>>(),
    })))
}
