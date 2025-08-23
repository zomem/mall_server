use actix_web::{Responder, Result, get, web};
use mysql_quick::{MysqlQuickCount, mycount, myfind};
use serde::{Deserialize, Serialize};
use serde_aux::prelude::deserialize_number_from_string;
use serde_json::Value;

use crate::PageData;
use crate::common::types::TranType;
use crate::routes::Res;
use crate::utils::filter::{deserialize_nested_json, deserialize_path_to_url};
use crate::{
    db::{my_run_vec, mysql_conn},
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

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct SaleRecord {
    id: u64,
    #[serde(deserialize_with = "deserialize_path_to_url")]
    avatar_url: String,
    nickname: Option<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    tran_amount: f64,
    tran_type: TranType,
    #[serde(deserialize_with = "deserialize_nested_json")]
    info: Value,
    created_at: String,
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct SearchSaleRecord {
    search_nickname: Option<String>,
}
/// 总销售、销售的分成列表
#[get("/manage/sales/records/list/{page}/{limit}")]
pub async fn manage_sales_records_list(
    _mana: AuthMana,
    path: web::Path<(String, String)>,
    query: web::Query<SearchSaleRecord>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (page, limit) = path.to_owned();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();
    let search_key = query.0.search_nickname.map_or(String::default(), |s| s);

    let r = if search_key.is_empty() {
        "p0 && (p2 || p3)"
    } else {
        "p0 && p1 && (p2 || p3)"
    };
    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("usr_transaction_records", {
            j0: ["uid", "inner", "usr_silent.id"],
            p0: ["is_del", "=", 0],
            p1: ["usr_silent.nickname", "like", format!("%{}%", search_key)],
            p2: ["tran_type", "=", TranType::MainSaleSplit.to_string()],
            p3: ["tran_type", "=", TranType::SaleSplit.to_string()],
            r: r,
        }),
    )?;

    let list: Vec<SaleRecord> = my_run_vec(
        &mut conn,
        myfind!("usr_transaction_records", {
            j0: ["uid", "inner", "usr_silent.id"],
            p0: ["is_del", "=", 0],
            p1: ["usr_silent.nickname", "like", format!("%{}%", search_key)],
            p2: ["tran_type", "=", TranType::MainSaleSplit.to_string()],
            p3: ["tran_type", "=", TranType::SaleSplit.to_string()],
            r: r,
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id, usr_silent.avatar_url, usr_silent.nickname,
                tran_amount, tran_type, info, created_at
            ",
        }),
    )?;

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}
