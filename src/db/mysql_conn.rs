use actix_web::{Error, error};
use mysql_quick::{
    MysqlQuick, PooledConn, Transaction, TxOpts, my_run_drop as run_drop,
    my_run_tran_drop as run_tran_drop, my_run_tran_vec as run_tran_vec, my_run_vec as run_vec,
};
use serde::de::DeserializeOwned;

use crate::{common::MYSQL_URL, utils::utils::log_aes_err};

pub fn mysql_conn() -> anyhow::Result<PooledConn, Error> {
    let conn = MysqlQuick::new(MYSQL_URL)
        .map_err(|e| error::ErrorInternalServerError(log_aes_err(&e, "数据库连接出错")))?
        .pool
        .get_conn()
        .map_err(|e| error::ErrorInternalServerError(log_aes_err(&e, "数据库连接出错2")))?;
    Ok(conn)
}

pub fn mysql_tran(conn: &mut PooledConn) -> anyhow::Result<Transaction, Error> {
    let tran = conn
        .start_transaction(TxOpts::default())
        .map_err(|e| error::ErrorInternalServerError(log_aes_err(&e, "数据库连接出错")))?;
    Ok(tran)
}

pub fn my_run_vec<U>(conn: &mut PooledConn, sql: String) -> anyhow::Result<Vec<U>, Error>
where
    U: DeserializeOwned,
{
    let data: Vec<U> = run_vec(conn, sql.clone())
        .map_err(|e| error::ErrorInternalServerError(log_aes_err(&e, &sql)))?;
    Ok(data)
}

pub fn my_run_drop(conn: &mut PooledConn, sql: String) -> anyhow::Result<u64, Error> {
    let data = run_drop(conn, sql.clone())
        .map_err(|e| error::ErrorInternalServerError(log_aes_err(&e, &sql)))?;
    Ok(data)
}

pub fn my_run_tran_vec<U>(tran: &mut Transaction, sql: String) -> anyhow::Result<Vec<U>, Error>
where
    U: DeserializeOwned,
{
    let data: Vec<U> = run_tran_vec(tran, sql.clone())
        .map_err(|e| error::ErrorInternalServerError(log_aes_err(&e, &sql)))?;
    Ok(data)
}

pub fn my_run_tran_drop(tran: &mut Transaction, sql: String) -> anyhow::Result<u64, Error> {
    let data = run_tran_drop(tran, sql.clone())
        .map_err(|e| error::ErrorInternalServerError(log_aes_err(&e, &sql)))?;
    Ok(data)
}
