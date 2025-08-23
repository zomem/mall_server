use actix_web::{Error, error};
use redis::{Client, Connection};

use crate::{common::REDIS_URL, utils::utils::log_err};

pub fn redis_conn() -> anyhow::Result<Connection, Error> {
    let url = REDIS_URL;
    let client = Client::open(url)
        .map_err(|e| error::ErrorInternalServerError(log_err(&e, "Redis连接出错啦")))?;
    let con = client
        .get_connection()
        .map_err(|e| error::ErrorInternalServerError(log_err(&e, "Redis连接出错啦～")))?;
    Ok(con)
}
