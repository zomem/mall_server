use actix_web::{Responder, Result, get, post, put, web};
use mysql_quick::{MysqlQuickCount, mycount, myfind, myset, mysetmany, myupdate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::types::{DeliveryType, OssBucket};
use crate::common::{PRODUCT_START_SN, UNIT_START_SN};
use crate::routes::{BaseInfo, BaseNumInfo, PageData, PdAttr, Res, StoreInfo};
use crate::utils::files::{get_file_url, get_file_urls, get_path_from_url, get_path_from_urls};
use crate::utils::html::{to_html_image_paths, to_html_image_urls};
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::AuthMana,
};

/// 获取产品的商品属性列表
#[get("/manage/mall/product/unit_attr/{product_sn}")]
pub async fn manage_mall_product_unit_attr(
    _mana: AuthMana,
    query: web::Path<String>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let p_sn: u32 = query.to_owned().parse().unwrap();

    let mut pd_list: Vec<BaseInfo> = vec![];
    let sql = myfind!("sku_attr", {
        p0: ["is_del", "=", 0],
        p1: ["product_sn", "=", p_sn],
        r: "p0 && p1",
    });
    let list: Vec<PdAttr> = my_run_vec(&mut conn, sql)?;
    // 一级
    for i in 0..list.len() {
        if list[i].primary_id > 0 && list[i].secondary_id == 0 {
            pd_list.push(BaseInfo {
                value: list[i].primary_id,
                label: list[i].name.clone(),
                children: vec![],
            });
        }
    }
    // 二级
    for i in 0..list.len() {
        if list[i].primary_id != 0 && list[i].secondary_id > 0 {
            for j in 0..pd_list.len() {
                if pd_list[j].value == list[i].primary_id {
                    pd_list[j].children.push(BaseInfo {
                        value: list[i].secondary_id,
                        label: list[i].name.clone(),
                        children: vec![],
                    });
                }
            }
        }
    }

    Ok(web::Json(Res::success(pd_list)))
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ProductAddrInfo {
    pub detail: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ProductAddCat {
    /// 一级分类
    pub primary_id: u32,
    pub primary_name: String,
    /// 二级分类
    pub secondary_id: u32,
    pub secondary_name: String,
    /// 三级分类
    pub tertiary_id: u32,
    pub tertiary_name: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ProductAddAttr {
    pub primary_id: u32,
    pub primary_name: String,
    pub secondary_id: u32,
    pub secondary_name: String,
    pub content: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ProductAdd {
    store_code: Option<u32>,
    product_sn: u32,
    product_name: String,
    product_sec_name: Option<String>,
    product_des: String,
    product_cover_img: String,
    delivery_type: Option<String>,
    product_imgs: Vec<String>,
    product_brand: Option<u32>,
    product_cat: Vec<ProductAddCat>,
    product_attr: Vec<ProductAddAttr>,
    addr_info: Option<ProductAddrInfo>,
    uid: Option<u64>,
    province_info: Option<[String; 3]>,
    /// 产品详情
    html: Option<String>,
    /// 产品特色
    peculiarity_html: Option<String>,
    /// 排序
    sort: Option<u32>,
    /// 布局方式
    product_layout: Option<String>,
    /// 产品价格
    combined_price: Option<f64>,
}
/// 【产品】新增或更新
#[utoipa::path(
    request_body = ProductAdd,
    responses((status = 200, description = "【请求：ProductAdd】【返回：String】有 product_sn 表示更新，为0表示新增", body = String)),
)]
#[post("/manage/mall/product/add")]
pub async fn manage_mall_product_add(
    _mana: AuthMana,
    params: web::Json<ProductAdd>,
) -> Result<impl Responder> {
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
    let sort = if let Some(s) = params.sort { s } else { 0 };
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
    let temp_province;
    let temp_city;
    let temp_area;
    let temp_delivery;
    if let Some(delivery) = &params.delivery_type {
        temp_delivery = delivery.clone();
    } else {
        temp_delivery = DeliveryType::NoDelivery.to_string();
    }
    if let Some(a) = &params.province_info {
        temp_province = a[0].clone();
        temp_city = a[1].clone();
        temp_area = a[2].clone();
    } else {
        temp_province = "null".to_string();
        temp_city = "null".to_string();
        temp_area = "null".to_string();
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
    let html = if let Some(h) = &params.html {
        to_html_image_paths(h)
    } else {
        "null".to_string()
    };
    let peculiarity_html = if let Some(h) = &params.peculiarity_html {
        to_html_image_paths(h)
    } else {
        "null".to_string()
    };
    let sql;
    if params.product_sn >= PRODUCT_START_SN {
        // 有产品编号，则更新
        sql = myupdate!("spu_product", {"product_sn": product_sn_max}, {
            "store_code": &temp_store_code,
            "product_name": product_name,
            "product_sec_name": &params.product_sec_name,
            "product_des": &params.product_des,
            "delivery_type": &temp_delivery,
            "html": &html,
            "peculiarity_html": &peculiarity_html,
            "product_cover_img": get_path_from_url(&params.product_cover_img, &OssBucket::EobFiles),
            "product_imgs": get_path_from_urls(&params.product_imgs, &OssBucket::EobFiles).join(","),
            "brand_code": &params.product_brand,
            "province": &temp_province,
            "city": &temp_city,
            "area": &temp_area,
            "addr_detail": &temp_addr_detail,
            "lat": &temp_lat,
            "lng": &temp_lng,
            "uid": &params.uid,
            "sort": sort,
            "product_layout": &params.product_layout,
            "combined_price": &params.combined_price,
        })
    } else {
        // 新增
        sql = myset!("spu_product", {
            "store_code": &temp_store_code,
            "product_name": product_name,
            "product_sec_name": &params.product_sec_name,
            "product_des": &params.product_des,
            "delivery_type": &temp_delivery,
            "html": &html,
            "peculiarity_html": &peculiarity_html,
            "product_cover_img": get_path_from_url(&params.product_cover_img, &OssBucket::EobFiles),
            "product_imgs": get_path_from_urls(&params.product_imgs, &OssBucket::EobFiles).join(","),
            "brand_code": &params.product_brand,
            "product_sn": product_sn_max,
            "province": &temp_province,
            "city": &temp_city,
            "area": &temp_area,
            "addr_detail": &temp_addr_detail,
            "lat": &temp_lat,
            "lng": &temp_lng,
            "uid": &params.uid,
            "sort": sort,
            "product_layout": &params.product_layout,
            "combined_price": &params.combined_price,
        })
    }
    my_run_drop(&mut conn, sql)?;

    // 添加产品的属性
    // 先删除
    my_run_drop(
        &mut conn,
        myupdate!("spu_product_attr", {"product_sn": product_sn_max}, {
            "is_del": 1,
        }),
    )?;
    // 批量新增
    #[derive(Serialize, Deserialize, Debug)]
    struct ProductAddAttrSet {
        product_sn: u32,
        primary_id: u32,
        primary_name: String,
        secondary_id: u32,
        secondary_name: String,
        content: String,
    }
    let data: Vec<ProductAddAttrSet> = params
        .product_attr
        .clone()
        .into_iter()
        .map(|x| ProductAddAttrSet {
            product_sn: product_sn_max,
            primary_id: x.primary_id,
            primary_name: x.primary_name,
            secondary_id: x.secondary_id,
            secondary_name: x.secondary_name,
            content: x.content,
        })
        .collect();
    my_run_drop(&mut conn, mysetmany!("spu_product_attr", data))?;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProductDel {
    product_sn: u32,
}
/// 删除产品
#[put("/manage/mall/product/del")]
pub async fn manage_mall_product_del(
    _mana: AuthMana,
    params: web::Json<ProductDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("spu_product", {"product_sn": params.product_sn}, {"is_del": 1}),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProductStatus {
    product_sn: u32,
    status: i8,
}
/// 修改产品状态
#[put("/manage/mall/product/status")]
pub async fn manage_mall_product_status(
    _mana: AuthMana,
    params: web::Json<ProductStatus>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("spu_product", {"product_sn": params.product_sn}, {
            "status": &params.status
        }),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ProductAddCatRes {
    product_sn: u32,
    primary_id: u32,
    primary_name: String,
    secondary_id: u32,
    secondary_name: String,
    tertiary_id: u32,
    tertiary_name: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ProductAddAttrRes {
    product_sn: u32,
    primary_id: u32,
    primary_name: String,
    secondary_id: u32,
    secondary_name: String,
    content: String,
}
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ProductInfoRes {
    id: u64,
    product_sn: u32,
    product_name: String,
    product_sec_name: Option<String>,
    product_des: String,
    product_cover_img: String,
    delivery_type: String,
    product_imgs: Vec<String>,
    product_brand: Option<u32>,
    store_code: Option<u32>,
    created_at: String,
    status: i8,
    product_cat: Vec<ProductAddCatRes>,
    product_attr: Vec<ProductAddAttrRes>,
    province: Option<String>,
    city: Option<String>,
    area: Option<String>,
    addr_detail: Option<String>,
    lat: Option<f64>,
    lng: Option<f64>,
    uid: Option<f64>,
    phone: Option<String>,
    html: Option<String>,
    peculiarity_html: Option<String>,
    sort: Option<i32>,
    product_layout: Option<String>,
    combined_price: Option<f64>,
}
/// 【产品】产品列表
#[utoipa::path(
    responses((status = 200, description = "【返回：ProductInfoRes[]】", body = Res<PageData<StoreInfo>>)),
    params(("page", description="页码"), ("limit", description="每页数量"))
)]
#[get("/manage/mall/product/list/{page}/{limit}")]
pub async fn manage_mall_product_list(
    _mana: AuthMana,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (page, limit) = query.to_owned();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("spu_product", {
            p0: ["is_del", "=", 0],
            r: "p0",
        }),
    )?;

    #[derive(Serialize, Deserialize, Debug)]
    struct ProductInfoGet {
        id: u64,
        product_sn: u32,
        product_name: String,
        product_sec_name: Option<String>,
        product_des: String,
        delivery_type: String,
        product_cover_img: Option<String>,
        product_imgs: Option<String>,
        brand_code: Option<u32>,
        store_code: Option<u32>,
        created_at: String,
        status: i8,
        province: Option<String>,
        city: Option<String>,
        area: Option<String>,
        addr_detail: Option<String>,
        lat: Option<f64>,
        lng: Option<f64>,
        uid: Option<f64>,
        phone: Option<String>,
        html: Option<String>,
        peculiarity_html: Option<String>,
        sort: Option<i32>,
        product_layout: Option<String>,
        combined_price: Option<String>,
    }

    let list: Vec<ProductInfoGet> = my_run_vec(
        &mut conn,
        myfind!("spu_product", {
            j0: ["uid", "left", "usr_silent.id"],
            p0: ["is_del", "=", 0],
            r: "p0",
            page: page,
            limit: limit,
            order_by: "-sort,-created_at",
            select: "id,product_sn,product_name,product_sec_name,product_des,store_code,product_cover_img,province,city,area,addr_detail,lat,lng,uid,usr_silent.phone,product_imgs,delivery_type,html,peculiarity_html,brand_code,sort,product_layout,combined_price,created_at,status",
        }),
    )?;

    let pd_sn_list = list
        .iter()
        .map(|x| x.product_sn.to_string())
        .collect::<Vec<String>>();
    let pd_attr_list: Vec<ProductAddAttrRes> = my_run_vec(
        &mut conn,
        myfind!("spu_product_attr", {
            p0: ["product_sn", "in", pd_sn_list.join(",")],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "product_sn,primary_id,primary_name,secondary_id,secondary_name,content",
        }),
    )?;
    let pd_cat_list: Vec<ProductAddCatRes> = my_run_vec(
        &mut conn,
        myfind!("spu_product_cat", {
            p0: ["product_sn", "in", pd_sn_list.join(",")],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "product_sn,primary_id,primary_name,secondary_id,secondary_name,tertiary_id,tertiary_name",
        }),
    )?;

    let list: Vec<ProductInfoRes> = list
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
            let pattr: Vec<ProductAddAttrRes> = pd_attr_list
                .clone()
                .into_iter()
                .filter(|o| o.product_sn == x.product_sn)
                .collect();
            let pcat: Vec<ProductAddCatRes> = pd_cat_list
                .clone()
                .into_iter()
                .filter(|o| o.product_sn == x.product_sn)
                .collect();
            return ProductInfoRes {
                id: x.id,
                product_sn: x.product_sn,
                product_name: x.product_name,
                product_sec_name: x.product_sec_name,
                product_des: x.product_des,
                delivery_type: x.delivery_type,
                product_cover_img: get_file_url(Some(&temp_cover)).unwrap_or("".to_string()),
                product_imgs: get_file_urls(Some(&temp_imgs)),
                product_brand: x.brand_code,
                store_code: x.store_code,
                created_at: x.created_at,
                status: x.status,
                product_cat: pcat,
                product_attr: pattr,
                province: x.province,
                city: x.city,
                area: x.area,
                addr_detail: x.addr_detail,
                lat: x.lat,
                lng: x.lng,
                uid: x.uid,
                phone: x.phone,
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
                sort: x.sort,
                product_layout: x.product_layout,
                combined_price: x
                    .combined_price
                    .map_or(None, |m| Some(m.parse::<f64>().unwrap())),
            };
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

/// 【产品】产品搜索
#[utoipa::path(
    responses((status = 200, description = "【返回：BaseNumInfo[]】", body = Res<Vec<BaseNumInfo>>)),
    params(("keyword", description="搜索关键字"))
)]
#[get("/manage/mall/product/search/{keyword}")]
pub async fn manage_mall_product_search(
    _mana: AuthMana,
    query: web::Path<String>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let keyword = query.to_owned();
    #[derive(Deserialize)]
    struct ProductSearch {
        product_sn: u32,
        product_name: String,
    }
    let list: Vec<ProductSearch> = my_run_vec(
        &mut conn,
        myfind!("spu_product", {
            p0: ["product_name", "like", format!("%{keyword}%")],
            p1: ["is_del", "=", 0],
            p2: ["product_sn", "=", &keyword],
            r: "(p0 || p2) && p1",
        }),
    )?;
    let list: Vec<BaseNumInfo> = list
        .into_iter()
        .map(|x| BaseNumInfo {
            label: x.product_name,
            value: x.product_sn,
        })
        .collect();
    Ok(web::Json(Res::success(list)))
}

/// 商品关键字搜索，用于 BaseNumInfo
#[get("/manage/mall/product/unit/search/{keyword}")]
pub async fn manage_mall_product_unit_search(
    _mana: AuthMana,
    query: web::Path<String>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let keyword = query.to_owned();
    #[derive(Deserialize)]
    struct UnitSearch {
        unit_sn: u32,
        unit_name: String,
    }
    let list: Vec<UnitSearch> = my_run_vec(
        &mut conn,
        myfind!("sku_unit", {
            p0: ["unit_name", "like", format!("%{keyword}%")],
            p1: ["is_del", "=", 0],
            p2: ["unit_sn", "=", &keyword],
            r: "(p0 || p2) && p1",
        }),
    )?;
    let list: Vec<BaseNumInfo> = list
        .into_iter()
        .map(|x| BaseNumInfo {
            label: x.unit_name,
            value: x.unit_sn,
        })
        .collect();
    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnitAddAttr {
    primary_id: u32,
    primary_name: String,
    secondary_id: u32,
    secondary_name: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnitAdd {
    product_sn: u32,
    unit_sn: u32,
    unit_name: String,
    price: f64,
    quantity: u32,
    unit_cover: String,
    unit_imgs: Vec<String>,
    unit_attr: Vec<UnitAddAttr>,
    main_sale_split: Option<f64>,
    sale_split: Option<f64>,
    is_split: bool,
}
/// 新增商品
#[post("/manage/mall/product/unit/add")]
pub async fn manage_mall_product_unit_add(
    _mana: AuthMana,
    params: web::Json<UnitAdd>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    if params.product_sn < PRODUCT_START_SN {
        return Ok(web::Json(Res::fail("请选择所属产品")));
    }
    if params.unit_name.trim().is_empty() {
        return Ok(web::Json(Res::fail("请输入商品名")));
    }
    let m_amount = if let Some(ms) = params.main_sale_split {
        ms
    } else {
        0.0
    };
    let s_amount = if let Some(ss) = params.sale_split {
        ss
    } else {
        0.0
    };
    let is_split = if params.is_split { 1 } else { 0 };

    if is_split == 1 {
        if m_amount == 0. && s_amount == 0. {
            return Ok(web::Json(Res::fail(
                "总销售价和销售价的分成金额，不能同时为0",
            )));
        }
    }

    if params.price <= m_amount + s_amount {
        return Ok(web::Json(Res::fail(
            "总销售价和销售价的分成金额，不能超过商品价格",
        )));
    }

    let mut unit_sn_max = UNIT_START_SN;
    if params.unit_sn >= UNIT_START_SN {
        unit_sn_max = params.unit_sn;
    } else {
        #[derive(Deserialize)]
        struct LastMax {
            last: Option<u32>,
        }
        let last_max: Vec<LastMax> = my_run_vec(
            &mut conn,
            "select Max(unit_sn) as last from sku_unit".to_string(),
        )?;
        if last_max.len() > 0 {
            if let Some(mx) = last_max[0].last {
                if mx >= UNIT_START_SN {
                    unit_sn_max = mx + 1;
                }
            }
        }
    }

    let sql;
    if params.unit_sn >= UNIT_START_SN {
        // 有商品编号，则更新
        sql = myupdate!("sku_unit", {"unit_sn": unit_sn_max}, {
            "unit_name": &params.unit_name.trim(),
            "product_sn": params.product_sn,
            "price": params.price,
            "quantity": params.quantity,
            "unit_cover": get_path_from_url(&params.unit_cover, &OssBucket::EobFiles),
            "unit_imgs": get_path_from_urls(&params.unit_imgs, &OssBucket::EobFiles).join(","),
            "main_sale_split": params.main_sale_split,
            "sale_split": params.sale_split,
            "is_split": is_split,
        })
    } else {
        // 新增
        sql = myset!("sku_unit", {
            "unit_name": &params.unit_name.trim(),
            "unit_sn": unit_sn_max,
            "product_sn": params.product_sn,
            "price": params.price,
            "quantity": params.quantity,
            "unit_cover": get_path_from_url(&params.unit_cover, &OssBucket::EobFiles),
            "unit_imgs": get_path_from_urls(&params.unit_imgs, &OssBucket::EobFiles).join(","),
            "main_sale_split": params.main_sale_split,
            "sale_split": params.sale_split,
            "is_split": is_split,
        })
    }
    my_run_drop(&mut conn, sql)?;

    // 添加产品的属性
    // 先删除
    my_run_drop(
        &mut conn,
        myupdate!("sku_unit_attr", {"unit_sn": unit_sn_max}, {
            "is_del": 1,
        }),
    )?;
    if params.unit_attr.len() > 0 {
        // 批量新增
        #[derive(Serialize, Deserialize, Debug)]
        pub struct UnitAddAttrSet {
            unit_sn: u32,
            primary_id: u32,
            primary_name: String,
            secondary_id: u32,
            secondary_name: String,
        }
        let data: Vec<UnitAddAttrSet> = params
            .unit_attr
            .clone()
            .into_iter()
            .map(|x| UnitAddAttrSet {
                unit_sn: unit_sn_max,
                primary_id: x.primary_id,
                primary_name: x.primary_name,
                secondary_id: x.secondary_id,
                secondary_name: x.secondary_name,
            })
            .collect();
        my_run_drop(&mut conn, mysetmany!("sku_unit_attr", data))?;
    }

    Ok(web::Json(Res::success("")))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnitAddAttrRes {
    unit_sn: u32,
    primary_id: u32,
    primary_name: String,
    secondary_id: u32,
    secondary_name: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnitInfoRes {
    id: u64,
    unit_sn: u32,
    unit_name: String,
    price: f64,
    quantity: u32,
    product_sn: u32,
    unit_cover: String,
    unit_imgs: Vec<String>,
    created_at: String,
    status: i8,
    unit_attr: Vec<UnitAddAttrRes>,
    main_sale_split: Option<f64>,
    sale_split: Option<f64>,
    is_split: bool,
}
/// 获取产品的商品列表
#[get("/manage/mall/product/unit/list/{product_sn}/{page}/{limit}")]
pub async fn manage_mall_product_unit_list(
    _mana: AuthMana,
    query: web::Path<(String, String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (product_sn, page, limit) = query.to_owned();
    let product_sn: u32 = product_sn.to_owned().parse().unwrap();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("sku_unit", {
            p0: ["is_del", "=", 0],
            p1: ["product_sn", "=", product_sn],
            r: "p0 && p1",
        }),
    )?;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct UnitInfoGet {
        id: u64,
        product_sn: u32,
        unit_name: String,
        unit_sn: u32,
        price: String,
        quantity: u32,
        unit_cover: Option<String>,
        unit_imgs: Option<String>,
        created_at: String,
        status: i8,
        main_sale_split: Option<String>,
        sale_split: Option<String>,
        is_split: u8,
    }

    let list: Vec<UnitInfoGet> = my_run_vec(
        &mut conn,
        myfind!("sku_unit", {
            p0: ["is_del", "=", 0],
            p1: ["product_sn", "=", product_sn],
            r: "p0 && p1",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id,product_sn,unit_sn,unit_name,price,quantity,unit_cover,unit_imgs,created_at,status,main_sale_split,sale_split,is_split",
        }),
    )?;

    let pd_sn_list = list
        .iter()
        .map(|x| x.unit_sn.to_string())
        .collect::<Vec<String>>();
    let pd_attr_list: Vec<UnitAddAttrRes> = my_run_vec(
        &mut conn,
        myfind!("sku_unit_attr", {
            p0: ["unit_sn", "in", pd_sn_list.join(",")],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "unit_sn,primary_id,primary_name,secondary_id,secondary_name",
        }),
    )?;

    let list: Vec<UnitInfoRes> = list
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
                .collect();

            return UnitInfoRes {
                id: x.id,
                product_sn: x.product_sn,
                unit_sn: x.unit_sn,
                unit_name: x.unit_name,
                price: x.price.parse::<f64>().unwrap(),
                quantity: x.quantity,
                unit_cover: get_file_url(Some(&temp_cover)).unwrap_or("".to_string()),
                unit_imgs: get_file_urls(Some(&temp_imgs)),
                created_at: x.created_at,
                status: x.status,
                unit_attr: pattr,
                main_sale_split: x
                    .main_sale_split
                    .map_or(None, |m| Some(m.parse::<f64>().unwrap())),
                sale_split: x
                    .sale_split
                    .map_or(None, |s| Some(s.parse::<f64>().unwrap())),
                is_split: if x.is_split == 1 { true } else { false },
            };
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnitDel {
    unit_sn: u32,
}
/// 删除商品
#[put("/manage/mall/product/unit/del")]
pub async fn manage_mall_product_unit_del(
    _mana: AuthMana,
    params: web::Json<UnitDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("sku_unit", {"unit_sn": params.unit_sn}, {"is_del": 1}),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnitStatus {
    unit_sn: u32,
    status: u8,
}
/// 修改产品状态
#[put("/manage/mall/product/unit/status")]
pub async fn manage_mall_product_unit_status(
    _mana: AuthMana,
    params: web::Json<UnitStatus>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    my_run_drop(
        &mut conn,
        myupdate!("sku_unit", {"unit_sn": params.unit_sn}, {
            "status": &params.status
        }),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[cfg(test)]
mod test {
    use mysql_quick::myupdate;
    #[test]
    fn test_sql() {
        let s = 2;
        let sql = myupdate!("sku_unit", {"unit_sn": 100000}, {
            "status": &s
        });
        println!("{}", sql);
    }
}
