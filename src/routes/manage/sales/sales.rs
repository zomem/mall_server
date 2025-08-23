use actix_web::{Responder, Result, get, put, web};
use mysql_quick::{MysqlQuickCount, mycount, myfind, myupdate};
use serde::{Deserialize, Serialize};

use crate::PageData;
use crate::routes::Res;
use crate::utils::files::get_file_url;
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::AuthMana,
};

#[derive(Serialize, Deserialize)]
pub struct MainSaleInfo {
    id: u32,
    main_sale_uid: u64,
    main_sale_name: String,
    main_sale_avatar_url: String,
    sale_uid: u64,
    sale_avatar_url: String,
    sale_name: String,
    created_at: String,
    status: i8,
}

/// 总销售的销售列表
#[get("/manage/sales/main_sale_sub/list/{main_sale_uid}/{page}/{limit}")]
pub async fn manage_sales_main_sale_sub_list(
    _mana: AuthMana,
    query: web::Path<(String, String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (muid, page, limit) = query.to_owned();
    let muid: u64 = muid.to_owned().parse().unwrap();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("sal_main_sale", {
            p0: ["is_del", "=", 0],
            p1: ["main_sale_uid", "=", muid],
            r: "p0 && p1",
        }),
    )?;

    #[derive(Serialize, Deserialize, Debug)]
    struct MainSaleGet {
        id: u32,
        main_sale_uid: u64,
        main_sale_name: String,
        main_sale_avatar_url: Option<String>,
        sale_uid: u64,
        sale_avatar_url: Option<String>,
        sale_name: String,
        created_at: String,
        status: i8,
    }

    let list: Vec<MainSaleGet> = my_run_vec(
        &mut conn,
        myfind!("sal_main_sale", {
            j0: ["main_sale_uid", "inner", "usr_silent.id as u1"],
            j1: ["sale_uid", "inner", "usr_silent.id as u2"],
            p0: ["is_del", "=", 0],
            p1: ["main_sale_uid", "=", muid],
            r: "p0 && p1",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id,main_sale_uid,u1.nickname as main_sale_name,u1.avatar_url as main_sale_avatar_url,sale_uid,u2.avatar_url as sale_avatar_url,u2.nickname as sale_name,status,created_at",
        }),
    )?;

    let list: Vec<MainSaleInfo> = list
        .into_iter()
        .map(|x| {
            return MainSaleInfo {
                id: x.id,
                main_sale_uid: x.main_sale_uid,
                main_sale_avatar_url: get_file_url(x.main_sale_avatar_url)
                    .unwrap_or("".to_string()),
                main_sale_name: x.main_sale_name,
                created_at: x.created_at,
                status: x.status,
                sale_uid: x.sale_uid,
                sale_avatar_url: get_file_url(x.sale_avatar_url).unwrap_or("".to_string()),
                sale_name: x.sale_name,
            };
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize)]
pub struct SaleInfo {
    id: u32,
    uid: u64,
    name: String,
    avatar_url: String,
    sale_uid: u64,
    sale_avatar_url: String,
    sale_name: String,
    created_at: String,
    status: i8,
}
/// 销售的用户列表
#[get("/manage/sales/sale_sub/list/{sale_uid}/{page}/{limit}")]
pub async fn manage_sales_sale_sub_list(
    _mana: AuthMana,
    query: web::Path<(String, String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (suid, page, limit) = query.to_owned();
    let suid: u64 = suid.to_owned().parse().unwrap();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("sal_sale_user", {
            p0: ["is_del", "=", 0],
            p1: ["sale_uid", "=", suid],
            r: "p0 && p1",
        }),
    )?;

    #[derive(Serialize, Deserialize, Debug)]
    struct MainSaleGet {
        id: u32,
        uid: u64,
        name: String,
        avatar_url: Option<String>,
        sale_uid: u64,
        sale_avatar_url: Option<String>,
        sale_name: String,
        created_at: String,
        status: i8,
    }

    let list: Vec<MainSaleGet> = my_run_vec(
        &mut conn,
        myfind!("sal_sale_user", {
            j0: ["uid", "inner", "usr_silent.id as u1"],
            j1: ["sale_uid", "inner", "usr_silent.id as u2"],
            p0: ["is_del", "=", 0],
            p1: ["sale_uid", "=", suid],
            r: "p0 && p1",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id,uid,u1.nickname as name,u1.avatar_url as avatar_url,sale_uid,u2.avatar_url as sale_avatar_url,u2.nickname as sale_name,status,created_at",
        }),
    )?;

    let list: Vec<SaleInfo> = list
        .into_iter()
        .map(|x| {
            return SaleInfo {
                id: x.id,
                uid: x.uid,
                avatar_url: get_file_url(x.avatar_url).unwrap_or("".to_string()),
                name: x.name,
                created_at: x.created_at,
                status: x.status,
                sale_uid: x.sale_uid,
                sale_avatar_url: get_file_url(x.sale_avatar_url).unwrap_or("".to_string()),
                sale_name: x.sale_name,
            };
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaleStatus {
    id: u32,
    status: i8,
}
#[put("/manage/sales/main_sale/status")]
pub async fn manage_sales_main_sale_status(
    _mana: AuthMana,
    params: web::Json<SaleStatus>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("sal_main_sale", {"id": params.id}, {
            "status": &params.status
        }),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[put("/manage/sales/sale_user/status")]
pub async fn manage_sales_sale_user_status(
    _mana: AuthMana,
    params: web::Json<SaleStatus>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    my_run_drop(
        &mut conn,
        myupdate!("sal_sale_user", {"id": params.id}, {
            "status": &params.status
        }),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaleDel {
    id: u32,
}
#[put("/manage/sales/main_sale/del")]
pub async fn manage_sales_main_sale_del(
    _mana: AuthMana,
    params: web::Json<SaleDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    #[derive(Serialize, Deserialize, Debug)]
    struct Get {
        id: u32,
        main_sale_uid: u64,
        sale_uid: u64,
    }
    let list: Vec<Get> = my_run_vec(
        &mut conn,
        myfind!("sal_main_sale", {
            p0: ["id", "=", params.id],
            r: "p0",
        }),
    )?;
    if list.is_empty() {
        return Ok(web::Json(Res::fail("不存在")));
    }
    if list[0].main_sale_uid == list[0].sale_uid {
        return Ok(web::Json(Res::fail("不能删除与自己的绑定关系")));
    }
    my_run_drop(
        &mut conn,
        myupdate!("sal_main_sale", {"id": params.id}, {"is_del": 1}),
    )?;
    Ok(web::Json(Res::success("成功")))
}

#[put("/manage/sales/sale_user/del")]
pub async fn manage_sales_sale_user_del(
    _mana: AuthMana,
    params: web::Json<SaleDel>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    #[derive(Serialize, Deserialize, Debug)]
    struct Get {
        id: u32,
        uid: u64,
        sale_uid: u64,
    }
    let list: Vec<Get> = my_run_vec(
        &mut conn,
        myfind!("sal_sale_user", {
            p0: ["id", "=", params.id],
            r: "p0",
        }),
    )?;
    if list.is_empty() {
        return Ok(web::Json(Res::fail("不存在")));
    }
    if list[0].uid == list[0].sale_uid {
        return Ok(web::Json(Res::fail("不能删除与自己的绑定关系")));
    }
    my_run_drop(
        &mut conn,
        myupdate!("sal_sale_user", {"id": params.id}, {"is_del": 1}),
    )?;
    Ok(web::Json(Res::success("成功")))
}
