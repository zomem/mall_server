use actix_web::{Error, error};
use anyhow::Result;
use redis::Commands;

use crate::{common::PROJECT_NAME, db::redis_conn};

/// 用户，方法的使用频率限制，24小时内
pub fn freq_user_day(uid: u64, method: &str, max_count: u32) -> Result<(), Error> {
    let key = format!("{}:freq_user_day:{}:{}", PROJECT_NAME, uid, method);
    let mut redis_con = redis_conn()?;
    let count: u32 = redis_con.get(&key).unwrap_or(0);
    if count < max_count {
        // 可以使用
        if count == 0 {
            let _: () = redis_con
                .set_ex(&key, 1, 24 * 3600)
                .map_err(|_e| error::ErrorInternalServerError("服务器开小差啦"))?;
        } else {
            let _: () = redis_con
                .incr(&key, 1)
                .map_err(|_e| error::ErrorInternalServerError("服务器开小差啦"))?;
        }
        return Ok(());
    } else {
        return Err(error::ErrorForbidden("请求次数超过每日限制啦"));
    }
}

#[cfg(test)]
mod test {
    use super::freq_user_day;
    // use crate::db::redis_conn;
    // use redis::Commands;

    #[test]
    fn test_freq() {
        // let mut redis_con = redis_conn()?;
        // let _: () = redis_con.set("freq_key", 10).unwrap();
        // let c: u32 = redis_con.get("freq_key").unwrap_or(0);
        // println!("cccc1, {:?}", c);
        // let _: () = redis_con.incr("freq_key", 1).unwrap();
        // let c: String = redis_con.get("freq_key").unwrap_or("".to_string());
        // println!("cccc2, {:?}", c);
        // println!("cccc3, {:?}", c.is_empty());
        let a = freq_user_day(1, "fre_test", 3);

        println!("AAAA  {:?}", a);
    }
}
