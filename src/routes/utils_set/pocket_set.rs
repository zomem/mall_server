use actix_web::{Error, error};
use mysql_quick::{
    MY_EXCLUSIVE_LOCK, MysqlQuickCount, PooledConn, Transaction, mycount, myfind, myset, myupdate,
};
use serde::{Deserialize, Serialize};
use serde_aux::prelude::deserialize_number_from_string;

use crate::common::types::{PayType, TranType};
use crate::db::{my_run_drop, my_run_tran_drop, my_run_tran_vec, my_run_vec};
use crate::routes::utils_set::hash_set::{hash_user_pocket_money, hash_user_pocket_money_verify};
use crate::routes::utils_set::tran_set::add_tran_record;

/// 用户零钱信息
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct UserPocketMoney {
    pub id: u64,
    pub uid: u64,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub amount: f64,
    pub amount_hash: String,
    pub status: u8,
    pub is_del: u8,
}
// 用户零钱增加
pub fn pocket_money_add(
    tran: &mut Transaction,
    uid: u64,
    add_amount: f64,
    tran_type: TranType,
    pay_type: PayType,
    info: Option<&str>,
) -> Result<(), Error> {
    if add_amount < 0. {
        return Err(error::ErrorBadRequest("金额不能为负数"));
    }
    let user_pocket = get_user_pocket_money(tran, uid)?;
    let money_new = user_pocket.amount + add_amount;
    let hash = hash_user_pocket_money(uid, money_new);
    my_run_tran_drop(
        tran,
        myupdate!("usr_pocket_money", {"uid": uid}, {
            "amount": ["incr", add_amount],
            "amount_hash":["set", &hash],
        }),
    )?;
    // 新增交易记录
    let info_value =
        info.map(|s| serde_json::from_str(s).unwrap_or(serde_json::Value::String(s.to_string())));
    add_tran_record(
        tran,
        tran_type,
        pay_type,
        uid,
        add_amount,
        info_value.as_ref(),
    )?;

    Ok(())
}

// 用户零钱减少
pub fn pocket_money_sub(
    tran: &mut Transaction,
    uid: u64,
    sub_amount: f64,
    tran_type: TranType,
    pay_type: PayType,
    info: Option<&str>,
) -> Result<(), Error> {
    if sub_amount < 0. {
        return Err(error::ErrorBadRequest("金额不能为负数"));
    }
    let user_pocket = get_user_pocket_money(tran, uid)?;
    if user_pocket.amount < sub_amount {
        return Err(error::ErrorForbidden("用户零钱不足"));
    }
    let money_new = user_pocket.amount - sub_amount;
    let hash = hash_user_pocket_money(uid, money_new);
    my_run_tran_drop(
        tran,
        myupdate!("usr_pocket_money", {"uid": uid}, {
            "amount": ["incr", -sub_amount],
            "amount_hash":["set", &hash],
        }),
    )?;
    // 新增交易记录
    let info_value =
        info.map(|s| serde_json::from_str(s).unwrap_or(serde_json::Value::String(s.to_string())));
    add_tran_record(
        tran,
        tran_type,
        pay_type,
        uid,
        -sub_amount,
        info_value.as_ref(),
    )?;

    Ok(())
}

/// 获取用户零钱信息，同时加上检测
pub fn get_user_pocket_money(tran: &mut Transaction, uid: u64) -> Result<UserPocketMoney, Error> {
    let info: Vec<UserPocketMoney> = my_run_tran_vec(
        tran,
        myfind!("usr_pocket_money", {
            p0: ["uid", "=", uid],
            r: "p0",
        }) + MY_EXCLUSIVE_LOCK,
    )?;
    if info.is_empty() {
        return Err(error::ErrorNotFound("未找到用户的钱包信息，请联系客服"));
    }
    if info[0].is_del == 1 {
        return Err(error::ErrorNotFound("未找到用户的钱包信息"));
    }
    if info[0].status != 2 {
        return Err(error::ErrorForbidden("用户钱包未启用"));
    }
    let user_pocket = info[0].clone();
    let is_pass = hash_user_pocket_money_verify(&user_pocket.amount_hash, uid, user_pocket.amount)?;
    if !is_pass {
        return Err(error::ErrorForbidden("用户钱包异常，请联系客服"));
    }
    Ok(user_pocket)
}

/// 用户零钱数据初始化。一般只在用户静默登录时，运行一次
pub fn init_user_pocket_money(conn: &mut PooledConn, uid: u64) -> Result<(), Error> {
    let count: Vec<MysqlQuickCount> = my_run_vec(
        conn,
        mycount!("usr_pocket_money", {
            p0: ["uid", "=", uid],
            r: "p0",
        }),
    )?;
    if count[0].mysql_quick_count > 0 {
        // 已经有了，就不再新增
        return Ok(());
    }
    let hash = hash_user_pocket_money(uid, 0.);
    // 新增
    my_run_drop(
        conn,
        myset!("usr_pocket_money", {
            "uid": uid,
            "amount": 0.,
            "amount_hash": hash,
        }),
    )?;
    Ok(())
}
