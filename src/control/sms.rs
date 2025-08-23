use anyhow::{Result, anyhow};
use rand::prelude::*;
use redis::Commands;
use sms::aliyun::Aliyun;

use crate::common::{
    PROJECT_NAME, SMS_ACCESS_KEY_ID, SMS_ACCESS_KEY_SECRET, SMS_SIGN_NAME, SMS_TEMPLATE_CODE,
};
use crate::db::redis_conn;
use crate::middleware::save_logs;

/// 发送验证码
pub async fn sms_send_code(phone: &str) -> Result<()> {
    let key_name = format!("{}:sms_code:{}", PROJECT_NAME, phone);
    let mut redis_con = redis_conn().unwrap();
    let mut rng = rand::rng();
    let code = format!("{}", rng.random_range(100000..=999999));
    let msg = format!(r#"{{"code":"{}","product":"byj"}}"#, code);

    let save_code: String = redis_con.get(&key_name).unwrap_or("".to_string());

    if !save_code.is_empty() {
        // 有code，则没有过期，
        return Ok(());
    }

    let aliyun = Aliyun::new(SMS_ACCESS_KEY_ID, SMS_ACCESS_KEY_SECRET);
    let resp = match aliyun
        .send_sms(phone, SMS_SIGN_NAME, SMS_TEMPLATE_CODE, &msg)
        .await
    {
        Ok(d) => d,
        Err(e) => {
            save_logs("logs/utils/sms.log", &format!("{:?}", &e));
            return Err(anyhow!("sms接口请求出错"));
        }
    };
    //  println!("{:?}", resp);
    //  {"BizId": "825818724906120482^0", "Code": "OK", "Message": "OK", "RequestId": "150D8CB2-AD94-55B3-9433-BDED9E3222E8"}
    let status = resp.get("Code");
    if let Some(s) = status {
        if s == &"OK".to_owned() {
            let _: () = redis_con.set_ex(&key_name, &code, 15 * 60)?;
            Ok(())
        } else {
            Err(anyhow!("sms发送失败"))
        }
    } else {
        Err(anyhow!("sms发送失败"))
    }
}

pub struct SmsVerify {
    pub status: u8,
    pub message: String,
}
/// 验证，验证码
pub fn sms_verify(phone: &str, code: &str) -> Result<SmsVerify> {
    let key_name = format!("{}:sms_code:{}", PROJECT_NAME, phone);
    let mut redis_con = redis_conn().unwrap();
    let save_code: String = redis_con.get(&key_name).unwrap_or("".to_string());

    if save_code.is_empty() {
        // 没有code, 则是没发送成功
        return Ok(SmsVerify {
            status: 0,
            message: "未发送验证码".to_owned(),
        });
    }
    if save_code.as_str() != code {
        return Ok(SmsVerify {
            status: 0,
            message: "验证码错误".to_owned(),
        });
    }
    Ok(SmsVerify {
        status: 1,
        message: "验证通过".to_owned(),
    })
}

#[cfg(test)]
mod test {
    use crate::db::redis_conn;
    use redis::Commands;

    #[tokio::test]
    async fn test_sms_send_code() {
        // sms_send_code("2").await;
        let mut redis_con = redis_conn().unwrap();
        // let c: String = match redis_con.get("iaiaia") {
        //     Ok(d) => d,
        //     Err(e) => {
        //         println!("eeeee, {:?}", e);
        //         return;
        //     }
        // };
        let c: String = redis_con.get("iaiaia").unwrap_or("".to_string());
        println!("cccc, {:?}", c);
        println!("cccc, {:?}", c.is_empty());
    }
}
