use actix_web::{Responder, Result, error, get, post, put, web};
use mysql_quick::{myfind, myset, myupdate};
use serde::{Deserialize, Serialize};

use crate::common::PRODUCT_START_SN;
use crate::common::types::OssBucket;
use crate::routes::Res;
use crate::utils::files::{get_file_url, get_path_from_url};
use crate::utils::utils::log_err;
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::{AuthMana, AuthSuperMana},
};

#[derive(Deserialize, Debug)]
pub struct PdAttr {
    pub icon: Option<String>,
    pub name: String,
    pub primary_id: u32,
    pub secondary_id: u32,
    pub is_del: u8,
}

#[derive(Serialize, Deserialize)]
pub struct ProductAttrInfo {
    id: u32,
    icon: Option<String>,
    name: String,
    children: Vec<ProductAttrInfo>,
}
/// 获取产品属性列表
#[get("/manage/mall/attr/product/list")]
pub async fn manage_mall_attr_product_list(_mana: AuthMana) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let sql = myfind!("spu_attr", {
        p0: ["is_del", "=", 0],
        r: "p0",
    });
    let list: Vec<PdAttr> = my_run_vec(&mut conn, sql)?;
    let mut pd_list: Vec<ProductAttrInfo> = vec![];
    // 一级
    for i in 0..list.len() {
        if list[i].primary_id > 0 && list[i].secondary_id == 0 {
            pd_list.push(ProductAttrInfo {
                id: list[i].primary_id,
                icon: get_file_url(list[i].icon.clone()),
                name: list[i].name.clone(),
                children: vec![],
            });
        }
    }
    // 二级
    for i in 0..list.len() {
        if list[i].secondary_id > 0 {
            for j in 0..pd_list.len() {
                if pd_list[j].id == list[i].primary_id {
                    pd_list[j].children.push(ProductAttrInfo {
                        id: list[i].secondary_id,
                        icon: get_file_url(list[i].icon.clone()),
                        name: list[i].name.clone(),
                        children: vec![],
                    });
                }
            }
        }
    }

    Ok(web::Json(Res::success(pd_list)))
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ProductAttrAddType {
    Primary,
    Secondary,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ProductAttrAdd {
    primary_id: i32,
    secondary_id: i32,
    product_attr_type: ProductAttrAddType,
    name: String,
    icon: Option<String>,
}
/// 新增修改产品分类
#[post("/manage/mall/attr/product/add")]
pub async fn manage_mall_attr_product_add(
    _: AuthMana,
    params: web::Json<ProductAttrAdd>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let max_sql = "SELECT MAX(primary_id) as p_max,MAX(secondary_id)  as s_max FROM spu_attr";

    #[derive(Deserialize)]
    struct Max {
        p_max: Option<u32>,
        s_max: Option<u32>,
    }
    let max: Vec<Max> = my_run_vec(&mut conn, max_sql.to_string())?;

    let icon_path = if let Some(p) = &params.icon {
        get_path_from_url(p, &OssBucket::EobFiles)
    } else {
        "".to_owned()
    };

    let pd_cat: Vec<PdAttr> = my_run_vec(
        &mut conn,
        myfind!("spu_attr", {
            p0: ["name", "=", &params.name],
            r: "p0",
        }),
    )?;

    let sql;
    if pd_cat.len() > 0 {
        // 修改
        sql = match params.product_attr_type {
            ProductAttrAddType::Primary => {
                if params.primary_id == 0 {
                    return Ok(web::Json(Res::fail("分类id不能为0")));
                }
                if pd_cat[0].is_del == 1 {
                    myupdate!("spu_attr", {"primary_id": pd_cat[0].primary_id}, {
                        "icon": &icon_path,
                        "is_del": 0,
                    })
                } else {
                    myupdate!("spu_attr", {"primary_id": pd_cat[0].primary_id}, {
                        "name": &params.name,
                        "icon": &icon_path,
                    })
                }
            }
            ProductAttrAddType::Secondary => {
                if params.secondary_id == 0 {
                    return Ok(web::Json(Res::fail("分类id不能为0")));
                }
                if pd_cat[0].is_del == 1 {
                    myupdate!("spu_attr", {"secondary_id": pd_cat[0].secondary_id}, {
                        "icon": &icon_path,
                        "is_del": 0,
                    })
                } else {
                    myupdate!("spu_attr", {"secondary_id": pd_cat[0].secondary_id}, {
                        "name": &params.name,
                        "icon": &icon_path,
                    })
                }
            }
        };
    } else {
        // 新增
        sql = match params.product_attr_type {
            ProductAttrAddType::Primary => myset!("spu_attr", {
                "primary_id": if let Some(n) = max[0].p_max { n + 1} else { 1 },
                "name": &params.name,
                "icon": &icon_path,
            }),
            ProductAttrAddType::Secondary => {
                if params.primary_id == 0 {
                    return Ok(web::Json(Res::fail("一级分类不能为空")));
                }
                myset!("spu_attr", {
                    "primary_id": params.primary_id,
                    "secondary_id": if let Some(n) = max[0].s_max { n + 1} else { 1 },
                    "name": &params.name,
                    "icon": &icon_path,
                })
            }
        };
    }
    my_run_drop(&mut conn, sql)?;
    Ok(web::Json(Res::success("")))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProductAttrDel {
    name: String,
}
/// 删除产品分类
#[put("/manage/mall/attr/product/del")]
pub async fn manage_mall_attr_product_del(
    _: AuthSuperMana,
    params: web::Json<ProductAttrDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("spu_attr", { "name": &params.name }, {"is_del": 1}),
    )?;

    Ok(web::Json(Res::success("")))
}

/// 获取商品属性列表
#[get("/manage/mall/attr/unit/list/{product_sn}")]
pub async fn manage_mall_attr_unit_list(
    _mana: AuthMana,
    query: web::Path<String>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let p_sn: u32 = query.to_owned().parse().unwrap();

    let sql = myfind!("sku_attr", {
        p0: ["is_del", "=", 0],
        p1: ["product_sn", "=", p_sn],
        r: "p0 && p1",
    });
    let list: Vec<PdAttr> = my_run_vec(&mut conn, sql)?;
    let mut pd_list: Vec<ProductAttrInfo> = vec![];
    // 一级
    for i in 0..list.len() {
        if list[i].primary_id > 0 && list[i].secondary_id == 0 {
            pd_list.push(ProductAttrInfo {
                id: list[i].primary_id,
                icon: get_file_url(list[i].icon.clone()),
                name: list[i].name.clone(),
                children: vec![],
            });
        }
    }
    // 二级
    for i in 0..list.len() {
        if list[i].secondary_id > 0 {
            for j in 0..pd_list.len() {
                if pd_list[j].id == list[i].primary_id {
                    pd_list[j].children.push(ProductAttrInfo {
                        id: list[i].secondary_id,
                        icon: get_file_url(list[i].icon.clone()),
                        name: list[i].name.clone(),
                        children: vec![],
                    });
                }
            }
        }
    }

    Ok(web::Json(Res::success(pd_list)))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UnitAttrAdd {
    product_sn: u32,
    primary_id: i32,
    secondary_id: i32,
    product_attr_type: ProductAttrAddType,
    name: String,
    icon: Option<String>,
}
/// 新增修改商品分类
#[post("/manage/mall/attr/unit/add")]
pub async fn manage_mall_attr_unit_add(
    _: AuthMana,
    params: web::Json<UnitAttrAdd>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let max_sql = "SELECT MAX(primary_id) as p_max,MAX(secondary_id)  as s_max FROM sku_attr";

    if params.product_sn < PRODUCT_START_SN {
        return Err(error::ErrorBadRequest(log_err("产品编号错误", &params)));
    }

    #[derive(Deserialize)]
    struct Max {
        p_max: Option<u32>,
        s_max: Option<u32>,
    }
    let max: Vec<Max> = my_run_vec(&mut conn, max_sql.to_string())?;

    let icon_path = if let Some(p) = &params.icon {
        get_path_from_url(p, &OssBucket::EobFiles)
    } else {
        "".to_owned()
    };

    let pd_cat: Vec<PdAttr> = my_run_vec(
        &mut conn,
        myfind!("sku_attr", {
            p0: ["name", "=", &params.name],
            p1: ["product_sn", "=", params.product_sn],
            r: "p0 && p1",
        }),
    )?;

    let sql;
    if pd_cat.len() > 0 {
        // 修改
        sql = match params.product_attr_type {
            ProductAttrAddType::Primary => {
                if params.primary_id == 0 {
                    return Ok(web::Json(Res::fail("分类id不能为0")));
                }
                if pd_cat[0].is_del == 1 {
                    myupdate!("sku_attr", {"primary_id": pd_cat[0].primary_id}, {
                        "icon": &icon_path,
                        "is_del": 0,
                    })
                } else {
                    myupdate!("sku_attr", {"primary_id": pd_cat[0].primary_id}, {
                        "name": &params.name,
                        "icon": &icon_path,
                    })
                }
            }
            ProductAttrAddType::Secondary => {
                if params.secondary_id == 0 {
                    return Ok(web::Json(Res::fail("分类id不能为0")));
                }
                if pd_cat[0].is_del == 1 {
                    myupdate!("sku_attr", {"secondary_id": pd_cat[0].secondary_id}, {
                        "icon": &icon_path,
                        "is_del": 0,
                    })
                } else {
                    myupdate!("sku_attr", {"secondary_id": pd_cat[0].secondary_id}, {
                        "name": &params.name,
                        "icon": &icon_path,
                    })
                }
            }
        };
    } else {
        // 新增
        sql = match params.product_attr_type {
            ProductAttrAddType::Primary => myset!("sku_attr", {
                "product_sn": params.product_sn,
                "primary_id": if let Some(n) = max[0].p_max { n + 1} else { 1 },
                "name": &params.name,
                "icon": &icon_path,
            }),
            ProductAttrAddType::Secondary => {
                if params.primary_id == 0 {
                    return Ok(web::Json(Res::fail("一级分类不能为空")));
                }
                myset!("sku_attr", {
                    "product_sn": params.product_sn,
                    "primary_id": params.primary_id,
                    "secondary_id": if let Some(n) = max[0].s_max { n + 1} else { 1 },
                    "name": &params.name,
                    "icon": &icon_path,
                })
            }
        };
    }
    my_run_drop(&mut conn, sql)?;
    Ok(web::Json(Res::success("")))
}

/// 删除商品分类
#[put("/manage/mall/attr/unit/del")]
pub async fn manage_mall_attr_unit_del(
    _: AuthSuperMana,
    params: web::Json<ProductAttrDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("sku_attr", { "name": &params.name }, {"is_del": 1}),
    )?;

    Ok(web::Json(Res::success("")))
}
