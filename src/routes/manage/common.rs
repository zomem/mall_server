use actix_web::{Responder, Result, get, post, put, web};
use mysql_quick::{myfind, myget, myset, myupdate};
use serde::{Deserialize, Serialize};

use crate::common::types::{NormalStatus, OssBucket};
use crate::control::amap::amap_geocode_regeo;
use crate::routes::{BaseInfo, BaseNumInfo, BaseStrInfo, PdAttr, PdCat, Res};
use crate::utils::files::{get_file_urls, get_path_from_urls};
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::AuthMana,
};

#[derive(Serialize, Deserialize)]
pub struct BaseInfoStr {
    id: u32,
    label: String,
    value: String,
    children: Vec<BaseInfoStr>,
}
#[derive(Serialize, Deserialize)]
pub struct BaseRes {
    com_store_type: Vec<BaseNumInfo>,
    unit_attr: Vec<BaseInfo>,
    product_cat: Vec<BaseInfo>,
    product_attr: Vec<BaseInfo>,
    delivery_type: Vec<BaseStrInfo>,
    delivery: Vec<BaseStrInfo>,
    brand: Vec<BaseNumInfo>,
    province: Vec<BaseInfoStr>,
    product_layout: Vec<BaseStrInfo>,
    article_cat: Vec<BaseNumInfo>,
    roles: Vec<BaseNumInfo>,
}
/// 获取产品属性列表
#[get("/manage/common/base_info")]
pub async fn manage_common_base_info(_mana: AuthMana) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    let mut pd_list = BaseRes {
        com_store_type: vec![],
        unit_attr: vec![],
        product_cat: vec![],
        product_attr: vec![],
        brand: vec![],
        province: vec![],
        delivery_type: vec![],
        delivery: vec![],
        product_layout: vec![],
        article_cat: vec![],
        roles: vec![],
    };
    // let sql = myfind!("sku_attr", {
    //     p0: ["is_del", "=", 0],
    //     r: "p0",
    // });
    // let list: Vec<PdAttr> = my_run_vec(&mut conn, sql).unwrap();
    // // 一级
    // for i in 0..list.len() {
    //     if list[i].primary_id > 0 && list[i].secondary_id == 0 {
    //         pd_list.unit_attr.push(BaseInfo {
    //             value: list[i].primary_id,
    //             label: list[i].name.clone(),
    //             children: vec![],
    //         });
    //     }
    // }
    // // 二级
    // for i in 0..list.len() {
    //     if list[i].primary_id != 0 && list[i].secondary_id > 0 {
    //         for j in 0..pd_list.unit_attr.len() {
    //             if pd_list.unit_attr[j].value == list[i].primary_id {
    //                 pd_list.unit_attr[j].children.push(BaseInfo {
    //                     value: list[i].secondary_id,
    //                     label: list[i].name.clone(),
    //                     children: vec![],
    //                 });
    //             }
    //         }
    //     }
    // }

    // 获取产品的物流类别
    let sql = myfind!("sys_constants", {
        p0: ["key", "=", "delivery_type"],
        p1: ["is_del", "=", 0],
        r: "p0 && p1",
    });
    let list: Vec<BaseStrInfo> = my_run_vec(&mut conn, sql)?;
    pd_list.delivery_type = list;

    // 获取产品布局方式
    let sql = myfind!("sys_constants", {
        p0: ["key", "=", "product_layout"],
        p1: ["is_del", "=", 0],
        r: "p0 && p1",
    });
    let list: Vec<BaseStrInfo> = my_run_vec(&mut conn, sql)?;
    pd_list.product_layout = list;

    // 获取物流公司
    let sql = myfind!("sys_delivery", {
        p0: ["status", "=", NormalStatus::Online as i8],
        p1: ["is_del", "=", 0],
        r: "p0 && p1",
        select: "delivery_id as value, delivery_name as label",
    });
    let list: Vec<BaseStrInfo> = my_run_vec(&mut conn, sql)?;
    pd_list.delivery = list;

    // 获取文章分类
    let sql = myfind!("art_article_cat", {
        p0: ["is_del", "=", 0],
        p1: ["status", "=", NormalStatus::Online as i8],
        r: "p0 && p1",
        select: "id as value, name as label",
    });
    let list: Vec<BaseNumInfo> = my_run_vec(&mut conn, sql)?;
    pd_list.article_cat = list;

    // 获取角色列表
    let sql = myfind!("sys_role", {
        p0: ["is_del", "=", 0],
        r: "p0",
        select: "identifier as value, name as label",
    });
    let list: Vec<BaseNumInfo> = my_run_vec(&mut conn, sql)?;
    pd_list.roles = list;

    // 获取公司店铺分类
    let sql = myfind!("sys_com_store_type", {
        p0: ["is_del", "=", 0],
        p1: ["status", "=", 2],
        r: "p0 && p1",
    });
    #[derive(Serialize, Deserialize)]
    pub struct ComStore {
        title: String,
        code: u32,
    }
    let list: Vec<ComStore> = my_run_vec(&mut conn, sql)?;
    // 一级
    for i in 0..list.len() {
        pd_list.com_store_type.push(BaseNumInfo {
            value: list[i].code,
            label: list[i].title.clone(),
        });
    }

    let sql = myfind!("spu_attr", {
        p0: ["is_del", "=", 0],
        r: "p0",
    });
    let list: Vec<PdAttr> = my_run_vec(&mut conn, sql)?;
    // 一级
    for i in 0..list.len() {
        if list[i].primary_id > 0 && list[i].secondary_id == 0 {
            pd_list.product_attr.push(BaseInfo {
                value: list[i].primary_id,
                label: list[i].name.clone(),
                children: vec![],
            });
        }
    }
    // 二级
    for i in 0..list.len() {
        if list[i].primary_id != 0 && list[i].secondary_id > 0 {
            for j in 0..pd_list.product_attr.len() {
                if pd_list.product_attr[j].value == list[i].primary_id {
                    pd_list.product_attr[j].children.push(BaseInfo {
                        value: list[i].secondary_id,
                        label: list[i].name.clone(),
                        children: vec![],
                    });
                }
            }
        }
    }

    let sql = myfind!("spu_cat", {
        p0: ["is_del", "=", 0],
        r: "p0",
    });
    let list: Vec<PdCat> = my_run_vec(&mut conn, sql)?;
    // 一级
    for i in 0..list.len() {
        if list[i].primary_id > 0 && list[i].secondary_id == 0 && list[i].tertiary_id == 0 {
            pd_list.product_cat.push(BaseInfo {
                value: list[i].primary_id,
                label: list[i].name.clone(),
                children: vec![],
            });
        }
    }
    // 二级
    for i in 0..list.len() {
        if list[i].primary_id != 0 && list[i].secondary_id > 0 && list[i].tertiary_id == 0 {
            for j in 0..pd_list.product_cat.len() {
                if pd_list.product_cat[j].value == list[i].primary_id {
                    pd_list.product_cat[j].children.push(BaseInfo {
                        value: list[i].secondary_id,
                        label: list[i].name.clone(),
                        children: vec![],
                    });
                }
            }
        }
    }
    // 三级
    for i in 0..list.len() {
        if list[i].primary_id != 0 && list[i].secondary_id != 0 && list[i].tertiary_id > 0 {
            for j in 0..pd_list.product_cat.len() {
                if pd_list.product_cat[j].value == list[i].primary_id {
                    for k in 0..pd_list.product_cat[j].children.len() {
                        if pd_list.product_cat[j].children[k].value == list[i].secondary_id {
                            pd_list.product_cat[j].children[k].children.push(BaseInfo {
                                value: list[i].tertiary_id,
                                label: list[i].name.clone(),
                                children: vec![],
                            });
                        }
                    }
                }
            }
        }
    }

    // 获取品牌列表
    let sql = myfind!("brd_brand", {
        p0: ["is_del", "=", 0],
        p1: ["status", "=", 2],
        r: "p0 && p1",
    });
    #[derive(Serialize, Deserialize)]
    pub struct BrandItem {
        brand_code: u32,
        brand_name: String,
    }
    let list: Vec<BrandItem> = my_run_vec(&mut conn, sql)?;
    // 一级
    for i in 0..list.len() {
        pd_list.brand.push(BaseNumInfo {
            value: list[i].brand_code,
            label: list[i].brand_name.clone(),
        });
    }

    let sql = myfind!("cmn_province", {
        p0: ["town", "=", 0],
        r: "p0",
    });
    #[derive(Serialize, Deserialize)]
    pub struct ProvinceItem {
        name: String,
        province: u32,
        city: u32,
        area: u32,
        town: u32,
    }
    let list: Vec<ProvinceItem> = my_run_vec(&mut conn, sql)?;
    // 一级
    for i in 0..list.len() {
        if list[i].province > 0 && list[i].city == 0 && list[i].area == 0 && list[i].town == 0 {
            pd_list.province.push(BaseInfoStr {
                id: list[i].province,
                value: list[i].name.clone(),
                label: list[i].name.clone(),
                children: vec![],
            });
        }
    }
    // 二级
    // let zhi_city = ["北京市", "重庆市", "上海市", "天津市"];
    let zhi_city = [11, 50, 31, 12];
    for i in 0..list.len() {
        if zhi_city.contains(&list[i].province) {
            if list[i].province > 0 && list[i].city == 0 && list[i].area == 0 && list[i].town == 0 {
                for j in 0..pd_list.province.len() {
                    if pd_list.province[j].id == list[i].province {
                        pd_list.province[j].children.push(BaseInfoStr {
                            id: list[i].province,
                            value: list[i].name.clone(),
                            label: list[i].name.clone(),
                            children: vec![],
                        });
                    }
                }
            }
        } else {
            if list[i].province != 0 && list[i].city > 0 && list[i].area == 0 && list[i].town == 0 {
                for j in 0..pd_list.province.len() {
                    if pd_list.province[j].id == list[i].province {
                        pd_list.province[j].children.push(BaseInfoStr {
                            id: list[i].city,
                            value: list[i].name.clone(),
                            label: list[i].name.clone(),
                            children: vec![],
                        });
                    }
                }
            }
        }
    }
    // 三级
    for i in 0..list.len() {
        if list[i].province != 0 && list[i].city != 0 && list[i].area > 0 && list[i].town == 0 {
            for j in 0..pd_list.province.len() {
                if pd_list.province[j].id == list[i].province {
                    for k in 0..pd_list.province[j].children.len() {
                        if zhi_city.contains(&list[i].province) {
                            if pd_list.province[j].children[k].id == list[i].province {
                                pd_list.province[j].children[k].children.push(BaseInfoStr {
                                    id: list[i].area,
                                    value: list[i].name.clone(),
                                    label: list[i].name.clone(),
                                    children: vec![],
                                });
                            }
                        } else {
                            if pd_list.province[j].children[k].id == list[i].city {
                                pd_list.province[j].children[k].children.push(BaseInfoStr {
                                    id: list[i].area,
                                    value: list[i].name.clone(),
                                    label: list[i].name.clone(),
                                    children: vec![],
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    // // 四级
    // for i in 0..list.len() {
    //     if list[i].province != 0 && list[i].city != 0 && list[i].area != 0 && list[i].town > 0 {
    //         for j in 0..pd_list.province.len() {
    //             if pd_list.province[j].value == list[i].province {
    //                 for k in 0..pd_list.province[j].children.len() {
    //                     if pd_list.province[j].children[k].value == list[i].city {
    //                         for n in 0..pd_list.province[j].children[k].children.len() {
    //                             if pd_list.province[j].children[k].children[n].value == list[i].area
    //                             {
    //                                 pd_list.province[j].children[k].children.push(BaseInfo {
    //                                     value: list[i].town,
    //                                     label: list[i].name.clone(),
    //                                     children: vec![],
    //                                 });
    //                             }
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    Ok(web::Json(Res::success(pd_list)))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BannerAdd {
    id: u32,
    img_urls: Vec<String>,
    path_urls: Option<String>,
    name: String,
    page: Option<String>,
    color: Option<String>,
}
/// 新增banner
#[post("/manage/common/banner/add")]
pub async fn manage_common_banner_add(
    _user: AuthMana,
    params: web::Json<BannerAdd>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    if params.id > 0 {
        my_run_drop(
            &mut conn,
            myupdate!("cmn_banner", params.id, {
                "img_urls": get_path_from_urls(&params.img_urls, &OssBucket::EobFiles).join(","),
                "path_urls": if let Some(p) = params.path_urls.clone() { p } else { "null".to_owned() },
                "name": &params.name,
                "page": if let Some(p) = params.page.clone() { p } else { "null".to_owned() },
                "color":  if let Some(p) = params.color.clone() { p } else { "null".to_owned() },
            }),
        )?;
    } else {
        my_run_drop(
            &mut conn,
            myset!("cmn_banner", {
                "img_urls": get_path_from_urls(&params.img_urls, &OssBucket::EobFiles).join(","),
                "path_urls": if let Some(p) = params.path_urls.clone() { p } else { "null".to_owned() },
                "name": &params.name,
                "page": if let Some(p) = params.page.clone() { p } else { "null".to_owned() },
                "color":  if let Some(p) = params.color.clone() { p } else { "null".to_owned() },
            }),
        )?;
    }

    Ok(web::Json(Res::success("ok")))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Banner {
    id: u32,
    imgs: Vec<String>,
    path_urls: Option<String>,
    name: String,
    page: Option<String>,
    color: Option<String>,
    status: u8,
}
#[derive(Deserialize)]
struct BannerGet {
    id: u32,
    img_urls: String,
    path_urls: Option<String>,
    name: String,
    page: Option<String>,
    color: Option<String>,
    status: u8,
}
/// banner列表
#[get("/manage/common/banner/list")]
pub async fn manage_common_banner_list(_user: AuthMana) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let list: Vec<BannerGet> = my_run_vec(
        &mut conn,
        myfind!("cmn_banner", {
            p0: ["is_del", "=", 0],
            r: "p0",
            select: "id,img_urls, path_urls, name, page, color, status",
        }),
    )?;
    let list: Vec<Banner> = list
        .into_iter()
        .map(|x| Banner {
            id: x.id,
            name: x.name,
            imgs: get_file_urls(Some(&x.img_urls)),
            page: x.page,
            path_urls: x.path_urls,
            color: x.color,
            status: x.status,
        })
        .collect();

    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BannerId {
    id: u32,
}
/// banner删除
#[put("/manage/common/banner/del")]
pub async fn manage_common_banner_del(
    _user: AuthMana,
    params: web::Json<BannerId>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let ban: Vec<BannerGet> = my_run_vec(
        &mut conn,
        myget!(
            "cmn_banner",
            params.id,
            "id,img_urls, path_urls, name, page, color, status"
        ),
    )?;
    if ban.len() > 0 {
        my_run_drop(&mut conn, myupdate!("cmn_banner", params.id, {"is_del": 1}))?;
    }

    Ok(web::Json(Res::success("")))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BannerStatus {
    id: u32,
    status: u8,
}
/// 修改banner状态
#[put("/manage/common/banner/status")]
pub async fn manage_common_banner_status(
    _mana: AuthMana,
    params: web::Json<BannerStatus>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("cmn_banner", params.id, {
            "status": params.status
        }),
    )?;
    Ok(web::Json(Res::success("成功")))
}

/// 通过经纬获取地址信息
#[get("/manage/common/geocode/regeo/{lat}/{lng}")]
pub async fn manage_common_geocode_regeo(
    _mana: AuthMana,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let lat = query.0.parse::<f64>().unwrap();
    let lng = query.1.parse::<f64>().unwrap();
    let info = amap_geocode_regeo((lat, lng)).await?;
    Ok(web::Json(Res::success(info)))
}
