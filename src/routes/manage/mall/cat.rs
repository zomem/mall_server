use actix_web::{Responder, Result, get, post, put, web};
use mysql_quick::{myfind, myget, myset, myupdate, myupdatemany};
use serde::{Deserialize, Serialize};

use crate::common::types::OssBucket;
use crate::routes::Res;
use crate::utils::files::{get_file_url, get_path_from_url};
use crate::utils::random::rand_string;
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::AuthMana,
};

#[derive(Deserialize, Debug)]
pub struct PdCat {
    pub icon: Option<String>,
    pub name: String,
    pub primary_id: u32,
    pub secondary_id: u32,
    pub tertiary_id: u32,
    pub sort: i32,
    pub is_del: u8,
}

#[derive(Serialize, Deserialize)]
pub struct ProductCatInfo {
    id: u32,
    icon: Option<String>,
    name: String,
    sort: i32,
    children: Vec<ProductCatInfo>,
}
/// 获取产品分类列表
#[get("/manage/mall/cat/list")]
pub async fn manage_mall_cat_list(_mana: AuthMana) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let sql = myfind!("spu_cat", {
        p0: ["is_del", "=", 0],
        r: "p0",
        order_by: "-sort",
    });
    let list: Vec<PdCat> = my_run_vec(&mut conn, sql)?;
    let mut pd_list: Vec<ProductCatInfo> = vec![];
    // 一级
    for i in 0..list.len() {
        if list[i].primary_id > 0 && list[i].secondary_id == 0 && list[i].tertiary_id == 0 {
            pd_list.push(ProductCatInfo {
                id: list[i].primary_id,
                icon: get_file_url(list[i].icon.clone()),
                name: list[i].name.clone(),
                sort: list[i].sort,
                children: vec![],
            });
        }
    }
    // 二级
    for i in 0..list.len() {
        if list[i].secondary_id > 0 && list[i].tertiary_id == 0 {
            for j in 0..pd_list.len() {
                if pd_list[j].id == list[i].primary_id {
                    pd_list[j].children.push(ProductCatInfo {
                        id: list[i].secondary_id,
                        icon: get_file_url(list[i].icon.clone()),
                        name: list[i].name.clone(),
                        sort: list[i].sort,
                        children: vec![],
                    });
                }
            }
        }
    }
    // 三级
    for i in 0..list.len() {
        if list[i].tertiary_id > 0 {
            for j in 0..pd_list.len() {
                if pd_list[j].id == list[i].primary_id {
                    for k in 0..pd_list[j].children.len() {
                        if pd_list[j].children[k].id == list[i].secondary_id {
                            pd_list[j].children[k].children.push(ProductCatInfo {
                                id: list[i].tertiary_id,
                                icon: get_file_url(list[i].icon.clone()),
                                name: list[i].name.clone(),
                                sort: list[i].sort,
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

#[derive(Serialize, Deserialize, Debug)]
pub enum ProductCatAddType {
    Primary,
    Secondary,
    Tertiary,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ProductCatAdd {
    primary_id: i32,
    secondary_id: i32,
    tertiary_id: i32,
    product_cat_type: ProductCatAddType,
    name: String,
    sort: Option<i32>,
    icon: Option<String>,
}
/// 新增修改产品分类
#[post("/manage/mall/cat/add")]
pub async fn manage_mall_cat_add(
    _: AuthMana,
    params: web::Json<ProductCatAdd>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let max_sql = "SELECT MAX(primary_id) as p_max,MAX(secondary_id)  as s_max,MAX(tertiary_id) as t_max FROM spu_cat";

    // myfind!("spu_cat", {
    //     select: "MAX(primary_id) as p_max,MAX(secondary_id) as s_max,MAX(tertiary_id) as t_max",
    // });

    let sort = if let Some(s) = params.sort { s } else { 0 };
    #[derive(Deserialize)]
    struct Max {
        p_max: Option<u32>,
        s_max: Option<u32>,
        t_max: Option<u32>,
    }
    let max: Vec<Max> = my_run_vec(&mut conn, max_sql.to_string())?;

    let icon_path = if let Some(p) = &params.icon {
        get_path_from_url(p, &OssBucket::EobFiles)
    } else {
        "".to_owned()
    };

    let pd_cat: Vec<PdCat> = my_run_vec(
        &mut conn,
        myfind!("spu_cat", {
            p0: ["name", "=", &params.name],
            r: "p0",
        }),
    )?;

    let sql;
    if pd_cat.len() > 0 {
        // 修改
        sql = match params.product_cat_type {
            ProductCatAddType::Primary => {
                if params.primary_id == 0 {
                    return Ok(web::Json(Res::fail("分类id不能为0")));
                }
                if pd_cat[0].is_del == 1 {
                    myupdate!("spu_cat", {"primary_id": pd_cat[0].primary_id}, {
                        "icon": &icon_path,
                        "sort": sort,
                        "is_del": 0,
                    })
                } else {
                    myupdate!("spu_cat", {"primary_id": pd_cat[0].primary_id}, {
                        "name": &params.name,
                        "sort": sort,
                        "icon": &icon_path,
                    })
                }
            }
            ProductCatAddType::Secondary => {
                if params.secondary_id == 0 {
                    return Ok(web::Json(Res::fail("分类id不能为0")));
                }
                if pd_cat[0].is_del == 1 {
                    myupdate!("spu_cat", {"secondary_id": pd_cat[0].secondary_id}, {
                        "icon": &icon_path,
                        "sort": sort,
                        "is_del": 0,
                    })
                } else {
                    myupdate!("spu_cat", {"secondary_id": pd_cat[0].secondary_id}, {
                        "name": &params.name,
                        "sort": sort,
                        "icon": &icon_path,
                    })
                }
            }
            ProductCatAddType::Tertiary => {
                if params.tertiary_id == 0 {
                    return Ok(web::Json(Res::fail("分类id不能为0")));
                }
                if pd_cat[0].is_del == 1 {
                    myupdate!("spu_cat", {"tertiary_id": pd_cat[0].tertiary_id}, {
                        "icon": &icon_path,
                        "sort": sort,
                        "is_del": 0,
                    })
                } else {
                    myupdate!("spu_cat", {"tertiary_id": pd_cat[0].tertiary_id}, {
                        "name": &params.name,
                        "sort": sort,
                        "icon": &icon_path,
                    })
                }
            }
        };
    } else {
        // 新增
        sql = match params.product_cat_type {
            ProductCatAddType::Primary => myset!("spu_cat", {
                "primary_id": if let Some(n) = max[0].p_max { n + 1} else { 1 },
                "name": &params.name,
                "icon": &icon_path,
                "sort": sort,
            }),
            ProductCatAddType::Secondary => {
                if params.primary_id == 0 {
                    return Ok(web::Json(Res::fail("一级分类不能为空")));
                }
                myset!("spu_cat", {
                    "primary_id": params.primary_id,
                    "secondary_id": if let Some(n) = max[0].s_max { n + 1} else { 1 },
                    "name": &params.name,
                    "icon": &icon_path,
                    "sort": sort,
                })
            }
            ProductCatAddType::Tertiary => {
                if params.primary_id == 0 {
                    return Ok(web::Json(Res::fail("一级分类不能为空")));
                }
                if params.secondary_id == 0 {
                    return Ok(web::Json(Res::fail("二级分类不能为空")));
                }
                myset!("spu_cat", {
                    "primary_id": params.primary_id,
                    "secondary_id": params.secondary_id,
                    "tertiary_id": if let Some(n) = max[0].t_max { n + 1} else { 1 },
                    "name": &params.name,
                    "icon": &icon_path,
                    "sort": sort,
                })
            }
        };
    }
    my_run_drop(&mut conn, sql)?;
    Ok(web::Json(Res::success("")))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProductCatDel {
    name: String,
}
/// 删除产品分类
#[put("/manage/mall/cat/del")]
pub async fn manage_mall_cat_del(
    _: AuthMana,
    params: web::Json<ProductCatDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    #[derive(Deserialize, Serialize)]
    struct CatGet {
        id: i32,
        name: String,
        is_del: i8,
    }
    #[derive(Deserialize)]
    struct ProductCatDelGet {
        primary_id: i32,
        secondary_id: i32,
        tertiary_id: i32,
    }
    let data: Vec<ProductCatDelGet> = my_run_vec(
        &mut conn,
        myget!("spu_cat", {"name": &params.name}, "primary_id,secondary_id,tertiary_id"),
    )?;
    if data.is_empty() {
        return Ok(web::Json(Res::success("")));
    }
    if data[0].tertiary_id > 0 {
        let change_name = format!("{}_{}", params.name, rand_string(18));
        my_run_drop(
            &mut conn,
            myupdate!("spu_cat", { "tertiary_id": data[0].tertiary_id }, {"is_del": 1, "name": change_name}),
        )?;
    } else if data[0].secondary_id > 0 {
        let cat: Vec<CatGet> = my_run_vec(
            &mut conn,
            myfind!("spu_cat", {
                p0: ["secondary_id", "=", data[0].secondary_id],
                p1: ["is_del", "=", 0],
                r: "p0 && p1",
            }),
        )?;
        let cat = cat
            .iter()
            .map(|item| CatGet {
                id: item.id,
                name: format!("{}_{}", item.name, rand_string(18)),
                is_del: 1,
            })
            .collect::<Vec<CatGet>>();
        my_run_drop(&mut conn, myupdatemany!("spu_cat", "id", cat))?;
    } else if data[0].primary_id > 0 {
        let cat: Vec<CatGet> = my_run_vec(
            &mut conn,
            myfind!("spu_cat", {
                p0: ["primary_id", "=", data[0].primary_id],
                p1: ["is_del", "=", 0],
                r: "p0 && p1",
            }),
        )?;
        let cat = cat
            .iter()
            .map(|item| CatGet {
                id: item.id,
                name: format!("{}_{}", item.name, rand_string(18)),
                is_del: 1,
            })
            .collect::<Vec<CatGet>>();
        my_run_drop(&mut conn, myupdatemany!("spu_cat", "id", cat))?;
    }

    Ok(web::Json(Res::success("")))
}
