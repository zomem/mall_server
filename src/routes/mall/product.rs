use actix_web::{Responder, Result, error, get, post, web};
use mysql_quick::{myfind, myset, mysetmany, myupdate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::PRODUCT_START_SN;
use crate::common::types::{NormalStatus, OssBucket};
use crate::db::{my_run_drop, my_run_vec, mysql_conn};
use crate::middleware::AuthUser;
use crate::routes::{Brand, ProductAttr, Res};
use crate::utils::files::{get_file_url, get_file_urls, get_path_from_url};
use crate::utils::html::to_html_image_urls;
use crate::{ProductAddCat, ProductAddrInfo};

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct ProductRes {
    /// 产品自增id
    id: u64,
    /// 产品编号
    product_sn: u32,
    /// 产品名
    product_name: String,
    /// 产品描述
    product_des: Option<String>,
    /// 产品封面图
    product_cover_img: String,
    /// 产品图片列表
    product_imgs: Vec<String>,
    /// 售出的累计数量
    sell_total: u32,
    /// 产品价格，只用于显示
    combined_price: Option<f64>,
    /// 店铺编号
    store_code: Option<u32>,
    /// 时间
    created_at: String,
}
/// 【产品】获取产品列表
#[utoipa::path(
    responses((status = 200, description = "【返回：ProductRes[]】", body = Vec<ProductRes>)),
    params(("cat_id", description="产品三级分类id  0表示全部类别"),("type", description="类型：1为综合,2为销量降序,3为价格降序,4为价格升序"),("page", description="页码"))
)]
#[get("/mall/product/list/{cat_id}/{type}/{page}")]
pub async fn mall_product_list(
    query: web::Path<(String, String, String)>,
) -> Result<impl Responder> {
    let (cat_id, p_type, page) = query.to_owned();
    let page: u32 = page.to_owned().parse().unwrap();
    let cat_id: u32 = cat_id.to_owned().parse().unwrap();
    let p_type: i8 = p_type.to_owned().parse().unwrap();

    let mut conn = mysql_conn()?;
    let select = "id,product_sn,product_name,product_des,product_cover_img,product_imgs,sell_total,store_code,combined_price,created_at";
    let r;
    let order_by;

    if p_type == 1 {
        order_by = "-created_at";
    } else if p_type == 2 {
        order_by = "-sell_total";
    } else if p_type == 3 {
        order_by = "-combined_price";
    } else if p_type == 4 {
        order_by = "combined_price";
    } else {
        order_by = "-created_at";
    }
    if cat_id == 0 {
        r = "p0 && p2";
    } else {
        r = "p0 && p1 && p2 && p3";
    }

    let sql = if cat_id == 0 {
        myfind!("spu_product", {
            p0: ["is_del", "=", 0],
            p2: ["status", "=", NormalStatus::Online as u8],
            r: r,
            page: page,
            limit: 15,
            order_by: order_by,
            select: select,
        })
    } else {
        myfind!("spu_product", {
            j0: ["product_sn", "left", "spu_product_cat.product_sn"],
            p0: ["is_del", "=", 0],
            p1: ["spu_product_cat.is_del", "=", 0],
            p2: ["status", "=", NormalStatus::Online as u8],
            p3: ["spu_product_cat.tertiary_id", "=", cat_id],
            r: r,
            page: page,
            limit: 15,
            order_by: order_by,
            select: select,
        })
    };
    #[derive(Deserialize)]
    struct ProductGet {
        id: u64,
        product_sn: u32,
        product_name: String,
        product_des: Option<String>,
        product_cover_img: Option<String>,
        product_imgs: Option<String>,
        store_code: Option<u32>,
        sell_total: u32,
        created_at: String,
        combined_price: Option<String>,
    }
    let list: Vec<ProductGet> = my_run_vec(&mut conn, sql)?;

    let list: Vec<ProductRes> = list
        .into_iter()
        .map(|x| {
            let temp_cover = if let Some(p) = x.product_cover_img {
                p
            } else {
                "".to_string()
            };
            let temp_imgs = if let Some(p) = x.product_imgs {
                p
            } else {
                "".to_string()
            };
            return ProductRes {
                id: x.id,
                product_sn: x.product_sn,
                product_name: x.product_name,
                product_des: x.product_des,
                product_cover_img: get_file_url(Some(&temp_cover)).unwrap_or("".to_string()),
                product_imgs: get_file_urls(Some(&temp_imgs)),
                sell_total: x.sell_total,
                store_code: x.store_code,
                created_at: x.created_at,
                combined_price: x
                    .combined_price
                    .map_or(None, |m| Some(m.parse::<f64>().unwrap())),
            };
        })
        .collect();
    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct ProductDetail {
    /// 产品自增id
    id: u32,
    /// 产品编号
    product_sn: u32,
    /// 产品名
    product_name: String,
    /// 产品另一个名称，英文名
    product_sec_name: Option<String>,
    /// 产品描述
    product_des: Option<String>,
    /// 产品封面图
    product_cover_img: String,
    /// 产品图片列表
    product_imgs: Vec<String>,
    /// 购买的累计数量
    sell_total: u32,
    /// 产品价格，只用于显示
    combined_price: Option<f64>,
    /// 店铺编号
    store_code: Option<u32>,
    /// 品牌信息，没有则返回 null
    brand: Option<Brand>,
    /// 产品属性，没有则返回 null
    product_attr: Option<Vec<ProductAttr>>,
    /// 产品详情，富文本
    html: Option<String>,
    /// 产品特点，说明等，富文本
    peculiarity_html: Option<String>,
    /// 时间
    created_at: String,
}
/// 【产品】获取产品详情
#[utoipa::path(
    responses((status = 200, description = "【返回：ProductDetail】", body = ProductDetail)),
    params(("product_sn", description="产品编号"))
)]
#[get("/mall/product/detail/{product_sn}")]
pub async fn mall_product_detail(query: web::Path<String>) -> Result<impl Responder> {
    let prod_sn = query
        .to_owned()
        .parse::<u32>()
        .map_err(|_| error::ErrorNotFound("访问的内容不存在"))?;
    let mut conn = mysql_conn()?;
    let sql = myfind!("spu_product", {
        j0: ["product_sn", "right", "spu_product_cat.product_sn"],
        j1: ["brand_code", "left", "brd_brand.brand_code"],
        p0: ["is_del", "=", 0],
        p1: ["spu_product_cat.is_del", "=", 0], // 控制整个类别的产品显示不显示
        p2: ["status", "=", NormalStatus::Online as u8],
        p3: ["product_sn", "=", prod_sn],
        r: "p0 && p1 && p2 && p3",
        page: 1,
        limit: 1,
        select: "id,product_sn,product_name,product_sec_name,combined_price,product_des,product_cover_img,product_imgs,sell_total,store_code,html,peculiarity_html,
        created_at,brd_brand.id as bid,brd_brand.brand_name,brd_brand.brand_logo,brd_brand.brand_code,brd_brand.brand_sec_name,brd_brand.brand_des",
    });

    #[derive(Deserialize)]
    struct ProductGet {
        id: u32,
        product_sn: u32,
        product_name: String,
        product_sec_name: Option<String>,
        product_des: Option<String>,
        product_cover_img: Option<String>,
        product_imgs: Option<String>,
        combined_price: Option<String>,
        store_code: Option<u32>,
        sell_total: u32,
        created_at: String,
        html: Option<String>,
        peculiarity_html: Option<String>,

        bid: Option<u32>,
        brand_code: Option<u32>,
        brand_name: Option<String>,
        brand_logo: Option<String>,
        brand_sec_name: Option<String>,
        brand_des: Option<String>,
    }
    let list: Vec<ProductGet> = my_run_vec(&mut conn, sql)?;
    if list.len() == 0 {
        return Err(error::ErrorNotFound("产品不存在或已下架"));
    }
    // 查寻当前产品，的产品属性
    let attr: Vec<ProductAttr> = my_run_vec(
        &mut conn,
        myfind!("spu_product_attr", {
            p0: ["is_del", "=", 0],
            p1: ["product_sn", "=", prod_sn],
            r: "p0 && p1",
            select: "id,primary_id,secondary_id,primary_name,secondary_name,content",
        }),
    )?;

    let list: Vec<ProductDetail> = list
        .into_iter()
        .map(|x| {
            let temp_cover = if let Some(p) = x.product_cover_img {
                p
            } else {
                "".to_string()
            };
            let temp_imgs = if let Some(p) = x.product_imgs {
                p
            } else {
                "".to_string()
            };
            ProductDetail {
                id: x.id,
                product_sn: x.product_sn,
                product_name: x.product_name,
                product_sec_name: x.product_sec_name,
                product_des: x.product_des,
                product_cover_img: get_file_url(Some(&temp_cover)).unwrap_or("".to_string()),
                product_imgs: get_file_urls(Some(&temp_imgs)),
                combined_price: x
                    .combined_price
                    .map_or(None, |m| Some(m.parse::<f64>().unwrap())),
                sell_total: x.sell_total,
                store_code: x.store_code,
                created_at: x.created_at,
                html: if let Some(h) = x.html {
                    Some(to_html_image_urls(&h))
                } else {
                    None
                },
                peculiarity_html: if let Some(h) = x.peculiarity_html {
                    Some(to_html_image_urls(&h))
                } else {
                    None
                },
                brand: if x.bid.is_some() {
                    Some(Brand {
                        id: x.bid.unwrap_or_default(),
                        brand_code: x.brand_code.unwrap_or_default(),
                        brand_name: x.brand_name.unwrap_or_default(),
                        brand_logo: get_file_url(x.brand_logo),
                        brand_sec_name: x.brand_sec_name,
                        brand_des: x.brand_des,
                    })
                } else {
                    None
                },
                product_attr: if attr.is_empty() {
                    None
                } else {
                    Some(attr.clone())
                },
            }
        })
        .collect();
    Ok(web::Json(Res::success(list[0].clone())))
}

#[derive(Serialize, Deserialize, Debug, ToSchema, Clone)]
pub struct UnitAddAttrRes {
    primary_id: u32,
    primary_name: String,
    secondary_id: u32,
    secondary_name: String,
}
#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct UnitRes {
    /// 商品自增id
    id: u64,
    /// 商品编号
    unit_sn: u32,
    /// 商品名
    unit_name: String,
    /// 商品价格
    price: f64,
    /// 商品库存
    quantity: u32,
    /// 商品封面图
    unit_cover: String,
    /// 商品图片列表
    unit_imgs: Vec<String>,
    /// 时间
    created_at: String,
    /// 商品属性
    unit_attr: Vec<UnitAddAttrRes>,
}
/// 【商品】商品列表
#[utoipa::path(
    responses((status = 200, description = "【返回：UnitRes[]】", body = Vec<UnitRes>)),
    params(("product_sn", description="产品编号"))
)]
#[get("/mall/product/unit/list/{product_sn}")]
pub async fn mall_product_unit_list(query: web::Path<String>) -> Result<impl Responder> {
    let p_sn = query.to_owned().parse::<u32>().unwrap();
    let mut conn = mysql_conn()?;
    let sql = myfind!("sku_unit", {
        p0: ["is_del", "=", 0],
        p1: ["status", "=", NormalStatus::Online as u8],
        p2: ["product_sn", "=", p_sn],
        r: "p0 && p1 &&p2",
    });

    #[derive(Deserialize)]
    struct UnitGet {
        id: u64,
        unit_sn: u32,
        unit_name: String,
        price: String,
        quantity: u32,
        unit_cover: Option<String>,
        unit_imgs: Option<String>,
        created_at: String,
    }
    let list: Vec<UnitGet> = my_run_vec(&mut conn, sql)?;

    let pd_sn_list = list
        .iter()
        .map(|x| x.unit_sn.to_string())
        .collect::<Vec<String>>();
    #[derive(Deserialize, Clone)]
    struct UnitAddAttrGet {
        unit_sn: u32,
        primary_id: u32,
        primary_name: String,
        secondary_id: u32,
        secondary_name: String,
    }
    let pd_attr_list: Vec<UnitAddAttrGet> = my_run_vec(
        &mut conn,
        myfind!("sku_unit_attr", {
            p0: ["unit_sn", "in", pd_sn_list.join(",")],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "unit_sn,primary_id,primary_name,secondary_id,secondary_name",
        }),
    )?;

    let list: Vec<UnitRes> = list
        .into_iter()
        .map(|x| {
            let temp_cover = if let Some(p) = x.unit_cover {
                p
            } else {
                "".to_string()
            };
            let temp_imgs = if let Some(p) = x.unit_imgs {
                p
            } else {
                "".to_string()
            };

            let pattr: Vec<UnitAddAttrRes> = pd_attr_list
                .clone()
                .into_iter()
                .filter(|o| o.unit_sn == x.unit_sn)
                .map(|ua| UnitAddAttrRes {
                    primary_id: ua.primary_id,
                    primary_name: ua.primary_name,
                    secondary_id: ua.secondary_id,
                    secondary_name: ua.secondary_name,
                })
                .collect();

            return UnitRes {
                id: x.id,
                unit_sn: x.unit_sn,
                unit_name: x.unit_name,
                price: x.price.parse::<f64>().unwrap(),
                quantity: x.quantity,
                unit_cover: get_file_url(Some(&temp_cover)).unwrap_or("".to_string()),
                unit_imgs: get_file_urls(Some(&temp_imgs)),
                created_at: x.created_at,
                unit_attr: pattr,
            };
        })
        .collect();
    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct UserPubProduct {
    /// 店铺编号，可选
    store_code: Option<u32>,
    /// 产品编号
    product_sn: u32,
    /// 产品名称
    product_name: String,
    /// 产品封面图
    product_cover_img: String,
    /// 产品分类，product cat
    product_cat: Vec<ProductAddCat>,
    addr_info: Option<ProductAddrInfo>,
}
/// 【产品】用户发布产品
#[utoipa::path(
    request_body = UserPubProduct,
    responses((status = 200, description = "【请求：UserPubProduct】【返回：String】", body = String)),
)]
#[post("/mall/product/user/publish")]
pub async fn mall_product_user_publish(
    user: AuthUser,
    params: web::Json<UserPubProduct>,
) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;

    let product_name = params.product_name.trim();
    if product_name.is_empty() {
        return Ok(web::Json(Res::fail("产品名不能为空")));
    }
    let temp_store_code = if let Some(s) = params.store_code {
        s.to_string()
    } else {
        "null".to_string()
    };

    let mut product_sn_max = PRODUCT_START_SN;
    if params.product_sn >= PRODUCT_START_SN {
        product_sn_max = params.product_sn;
    } else {
        #[derive(Deserialize)]
        struct LastMax {
            last: Option<u32>,
        }
        let last_max: Vec<LastMax> = my_run_vec(
            &mut conn,
            "select Max(product_sn) as last from spu_product".to_string(),
        )?;
        if last_max.len() > 0 {
            if let Some(mx) = last_max[0].last {
                if mx >= PRODUCT_START_SN {
                    product_sn_max = mx + 1;
                }
            }
        }
    }
    let mut temp_addr_detail = None;
    let temp_lat;
    let temp_lng;
    if let Some(addr) = &params.addr_info {
        temp_addr_detail = addr.detail.clone();
        temp_lat = if let Some(x) = addr.lat {
            x.to_string()
        } else {
            "null".to_string()
        };
        temp_lng = if let Some(x) = addr.lng {
            x.to_string()
        } else {
            "null".to_string()
        };
    } else {
        temp_lat = "null".to_string();
        temp_lng = "null".to_string();
    }
    let sql;
    if params.product_sn >= PRODUCT_START_SN {
        // 有产品编号，则更新
        sql = myupdate!("spu_product", {"product_sn": product_sn_max}, {
            "store_code": &temp_store_code,
            "product_name": product_name,
            "product_cover_img": get_path_from_url(&params.product_cover_img, &OssBucket::EobFiles),
            "addr_detail": &temp_addr_detail,
            "lat": &temp_lat,
            "lng": &temp_lng,
        })
    } else {
        // 新增
        sql = myset!("spu_product", {
            "store_code": &temp_store_code,
            "product_name": product_name,
            "product_cover_img": get_path_from_url(&params.product_cover_img, &OssBucket::EobFiles),
            "product_sn": product_sn_max,
            "addr_detail": &temp_addr_detail,
            "lat": &temp_lat,
            "lng": &temp_lng,
            "uid": uid,
        })
    }
    my_run_drop(&mut conn, sql)?;

    // // 添加产品的属性
    // // 先删除
    // my_run_drop(
    //     &mut conn,
    //     myupdate!("spu_product_attr", {"product_sn": product_sn_max}, {
    //         "is_del": 1,
    //     }),
    // )
    // .unwrap();
    // // 批量新增
    // #[derive(Serialize, Deserialize, Debug)]
    // pub struct ProductAddAttrSet {
    //     product_sn: u32,
    //     primary_id: u32,
    //     primary_name: String,
    //     secondary_id: u32,
    //     secondary_name: String,
    //     content: String,
    // }
    // let data: Vec<ProductAddAttrSet> = params
    //     .product_attr
    //     .clone()
    //     .into_iter()
    //     .map(|x| ProductAddAttrSet {
    //         product_sn: product_sn_max,
    //         primary_id: x.primary_id,
    //         primary_name: x.primary_name,
    //         secondary_id: x.secondary_id,
    //         secondary_name: x.secondary_name,
    //         content: x.content,
    //     })
    //     .collect();
    // my_run_drop(&mut conn, mysetmany!("spu_product_attr", data)).unwrap();

    // 添加产品的分类
    // 先删除
    my_run_drop(
        &mut conn,
        myupdate!("spu_product_cat", {"product_sn": product_sn_max}, {
            "is_del": 1,
        }),
    )?;
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct ProductAddCatSet {
        product_sn: u32,
        primary_id: u32,
        primary_name: String,
        secondary_id: u32,
        secondary_name: String,
        tertiary_id: u32,
        tertiary_name: String,
    }
    let data: Vec<ProductAddCatSet> = params
        .product_cat
        .clone()
        .into_iter()
        .map(|x| ProductAddCatSet {
            product_sn: product_sn_max,
            primary_id: x.primary_id,
            primary_name: x.primary_name,
            secondary_id: x.secondary_id,
            secondary_name: x.secondary_name,
            tertiary_id: x.tertiary_id,
            tertiary_name: x.tertiary_name,
        })
        .collect();
    my_run_drop(&mut conn, mysetmany!("spu_product_cat", data))?;

    Ok(web::Json(Res::success("")))
}
