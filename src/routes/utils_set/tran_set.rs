use actix_web::Error;
use mysql_quick::{Transaction, myset};
use serde_json::Value;

use crate::common::types::{PayType, TranType};
use crate::db::my_run_tran_drop;
use crate::routes::utils_set::hash_set::hash_user_tran;

/// 新增，用户的交易记录。可以是购买，充值，提现，分成等等
pub fn add_tran_record(
    tran: &mut Transaction,
    tran_type: TranType,
    pay_type: PayType,
    uid: u64,
    tran_amount: f64,
    info: Option<&Value>,
) -> Result<(), Error> {
    let time = chrono::Local::now().timestamp();
    let hash = hash_user_tran(
        uid,
        tran_amount,
        &tran_type.to_string(),
        &pay_type.to_string(),
        time,
    )?;
    // 新增记录
    let info_str = info.map(|v| serde_json::to_string(v).unwrap_or_default());
    my_run_tran_drop(
        tran,
        myset!("usr_transaction_records", {
            "uid": uid,
            "tran_amount": tran_amount,
            "tran_amount_hash": hash,
            "tran_type": tran_type.to_string(),
            "pay_type": pay_type.to_string(),
            "info": info_str.as_deref(),
        }),
    )?;

    Ok(())
}
