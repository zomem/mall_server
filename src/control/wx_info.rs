use crate::common::{
    PROJECT_NAME, WECHAT_GZH_APP_ID, WECHAT_GZH_APP_SECRET, WECHAT_GZH_JS_SDK_URL,
    WECHAT_MINI_APP_ID, WECHAT_MINI_APP_SECRET, WECHAT_PAY_APIV3, WECHAT_PAY_MCH_ID,
    WECHAT_PAY_NOTIFY_URL, WECHAT_PAY_SERIAL, WECHAT_PRIVATE_KEY,
};
use crate::db::redis_conn;
use crate::utils::random::rand_unique;
use crate::utils::utils::log_err;
use actix_web::{Error, error};
use redis::Commands;
use serde::{Deserialize, Serialize};
use sha1::Digest;
use utoipa::ToSchema;
use wx_pay::WxPay;

/// 微信支付 初始化
pub fn wx_pay_init<'a>() -> WxPay<'a> {
    WxPay {
        appid: WECHAT_MINI_APP_ID,
        mchid: WECHAT_PAY_MCH_ID,
        private_key: WECHAT_PRIVATE_KEY,
        serial_no: WECHAT_PAY_SERIAL,
        api_v3_private_key: WECHAT_PAY_APIV3,
        notify_url: WECHAT_PAY_NOTIFY_URL,
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct WxAccessToken {
    expires_in: Option<usize>,
    access_token: String,
}
/// 获取 微信小程序的 access_token
pub async fn get_wx_mini_access_token() -> Result<String, Error> {
    let mut redis_con = redis_conn()?;

    let wx_url = "https://api.weixin.qq.com/cgi-bin/token?appid=".to_string()
        + WECHAT_MINI_APP_ID
        + "&secret="
        + WECHAT_MINI_APP_SECRET
        + "&grant_type=client_credential";

    let at_key_name = format!("{}:{}:access_token", PROJECT_NAME, WECHAT_MINI_APP_ID);
    let mut at_v: String = redis_con.get(&at_key_name).unwrap_or("".to_string());
    if at_v == String::from("") {
        let access_token_res: WxAccessToken = reqwest::get(wx_url)
            .await
            .map_err(|e| error::ErrorGatewayTimeout(e))?
            .json()
            .await
            .map_err(|e| error::ErrorInternalServerError(log_err(&e, "wx_info")))?;

        let expires_ss = if let Some(t) = access_token_res.expires_in {
            t
        } else {
            3600
        };

        let at_value = access_token_res.access_token.clone();
        at_v = at_value.clone();
        let _: () = redis_con
            .set_ex(&at_key_name, at_value, (expires_ss - 10) as u64)
            .map_err(|e| error::ErrorInternalServerError(log_err(&e, "wx_info")))?;
    }
    Ok(at_v)
}

/// 获取 微信公众号的 access_token
pub async fn get_wx_gzh_access_token() -> Result<String, Error> {
    let mut redis_con = redis_conn()?;

    let wx_url = "https://api.weixin.qq.com/cgi-bin/token?appid=".to_string()
        + WECHAT_GZH_APP_ID
        + "&secret="
        + WECHAT_GZH_APP_SECRET
        + "&grant_type=client_credential";

    let at_key_name = format!("{}:{}:access_token", PROJECT_NAME, WECHAT_GZH_APP_ID);
    let mut at_v: String = redis_con.get(&at_key_name).unwrap_or("".to_string());
    if at_v == String::from("") {
        let access_token_res: WxAccessToken = reqwest::get(wx_url)
            .await
            .map_err(|e| error::ErrorGatewayTimeout(e))?
            .json()
            .await
            .map_err(|e| error::ErrorInternalServerError(log_err(&e, "wx_info")))?;

        let expires_ss = if let Some(t) = access_token_res.expires_in {
            t
        } else {
            3600
        };

        let at_value = access_token_res.access_token.clone();
        at_v = at_value.clone();
        let _: () = redis_con
            .set_ex(&at_key_name, at_value, (expires_ss - 10) as u64)
            .map_err(|e| error::ErrorInternalServerError(log_err(&e, "wx_info")))?;
    }
    Ok(at_v)
}

#[derive(Serialize, Deserialize, Debug)]
struct WxJsapiTicket {
    expires_in: Option<usize>,
    ticket: String,
}
/// 获取 微信公众号的 jsapi_ticket
pub async fn get_wx_gzh_jsapi_ticket() -> Result<String, Error> {
    let mut redis_con = redis_conn()?;
    let access_token = get_wx_gzh_access_token().await?;
    let wx_url = "https://api.weixin.qq.com/cgi-bin/ticket/getticket?access_token=".to_string()
        + &access_token
        + "&type=jsapi";

    let at_key_name = format!("{}:{}:jsapi_ticket", PROJECT_NAME, WECHAT_GZH_APP_ID);
    let mut at_v: String = redis_con.get(&at_key_name).unwrap_or("".to_string());
    if at_v == String::from("") {
        let res: WxJsapiTicket = reqwest::get(wx_url)
            .await
            .map_err(|e| error::ErrorGatewayTimeout(e))?
            .json()
            .await
            .map_err(|e| error::ErrorInternalServerError(log_err(&e, "wx_info")))?;

        let expires_ss = if let Some(t) = res.expires_in {
            t
        } else {
            3600
        };

        let at_value = res.ticket.clone();
        at_v = at_value.clone();
        let _: () = redis_con
            .set_ex(&at_key_name, at_value, (expires_ss - 10) as u64)
            .map_err(|e| error::ErrorInternalServerError(log_err(&e, "wx_info")))?;
    }
    Ok(at_v)
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct WxJsSdkSign {
    pub app_id: String,
    pub signature: String,
    pub noncestr: String,
    pub timestamp: i64,
}
/// JS-SDK使用权限签名
pub async fn sign_wx_gzh_jssdk() -> Result<WxJsSdkSign, Error> {
    let noncestr = rand_unique();
    let timestamp = chrono::Local::now().timestamp();
    let ticket = get_wx_gzh_jsapi_ticket().await?;

    let content = format!(
        "jsapi_ticket={}&noncestr={}&timestamp={}&url={}",
        ticket, noncestr, timestamp, WECHAT_GZH_JS_SDK_URL
    );

    let mut hasher = sha1::Sha1::new();
    hasher.update(content);
    let result = hasher.finalize().to_vec();
    let signature: String = result
        .iter()
        .map(|c| format!("{:02x}", c))
        .collect::<Vec<_>>()
        .join("");

    Ok(WxJsSdkSign {
        app_id: WECHAT_GZH_APP_ID.to_string(),
        signature,
        noncestr,
        timestamp,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WXGzhWebAccessToken {
    pub access_token: String,
    pub expires_in: Option<usize>,
    pub refresh_token: String,
    pub openid: String,
    pub scope: Option<String>,
    pub unionid: Option<String>,
}
/// 微信公众号，web 静默登录，并存储 access_token
pub async fn get_wx_gzh_web_silent(code: &str) -> Result<WXGzhWebAccessToken, Error> {
    let mut redis_con = redis_conn()?;
    let wx_url = "https://api.weixin.qq.com/sns/oauth2/access_token?appid=".to_string()
        + WECHAT_GZH_APP_ID
        + "&secret="
        + WECHAT_GZH_APP_SECRET
        + "&code="
        + code
        + "&grant_type=authorization_code";

    let res: WXGzhWebAccessToken = reqwest::get(wx_url)
        .await
        .map_err(|e| error::ErrorGatewayTimeout(e))?
        .json()
        .await
        .map_err(|e| error::ErrorInternalServerError(log_err(&e, "wx_info")))?;

    let expires_ss = if let Some(t) = res.expires_in {
        t
    } else {
        3600
    };
    let at_key_name = format!(
        "{}:{}:web_access_token:{}",
        PROJECT_NAME, WECHAT_GZH_APP_ID, res.openid
    );
    let at_value = res.access_token.clone();

    let _: () = redis_con
        .set_ex(&at_key_name, &at_value, (expires_ss - 10) as u64)
        .map_err(|e| error::ErrorInternalServerError(log_err(&e, "wx_info")))?;
    Ok(res)
}

/// 获取 微信公众号，web 静默登录 的 access_token
pub fn get_wx_gzh_web_access_token(openid: &str) -> Result<String, Error> {
    let at_key_name = format!(
        "{}:{}:web_access_token:{}",
        PROJECT_NAME, WECHAT_GZH_APP_ID, openid
    );
    let mut redis_con = redis_conn()?;
    let at_v: String = redis_con.get(&at_key_name).unwrap_or("".to_string());
    if at_v.is_empty() {
        return Err(error::ErrorBadRequest("未获取到 access_token"));
    }
    Ok(at_v)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WXGzhWebUserInfo {
    pub openid: String,
    pub headimgurl: String,
    pub nickname: String,
    pub sex: i8,
    pub province: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
}
/// 获取 公众号登录的用户信息
pub async fn get_wx_gzh_web_user_info(openid: &str) -> Result<WXGzhWebUserInfo, Error> {
    let access_token = get_wx_gzh_web_access_token(openid)?;
    let wx_url = "https://api.weixin.qq.com/sns/userinfo?access_token=".to_string()
        + &access_token
        + "&openid="
        + openid
        + "&lang=zh_CN";

    let res: WXGzhWebUserInfo = reqwest::get(wx_url)
        .await
        .map_err(|e| error::ErrorGatewayTimeout(e))?
        .json()
        .await
        .map_err(|e| error::ErrorInternalServerError(log_err(&e, "wx_info")))?;

    Ok(res)
}

#[cfg(test)]
mod test {
    use sha1::{Digest, Sha1};

    #[test]
    fn test_sha1() {
        let s = "jsapi_ticket=sM4AOVdWfPE4DxkXGEs8VMCPGGVi4C3VM0P37wVUCFvkVAy_90u5h9nbSlYy3-Sl-HhTdfl2fzFy1AOcHKP7qg&noncestr=Wm3WZYTPz0wzccnW&timestamp=1414587457&url=http://mp.weixin.qq.com?params=value";

        let mut hasher = Sha1::new();
        hasher.update(s);
        let result: Vec<u8> = hasher.finalize().to_vec();
        let hex_string: String = result
            .iter()
            .map(|c| format!("{:02x}", c))
            .collect::<Vec<_>>()
            .join("");

        assert_eq!(hex_string, "0f9de62fce790f9a083d5c99e95740ceb90c27ed")
    }
}
