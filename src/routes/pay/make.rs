use actix_web::{Responder, Result, post, web};
use mysql_quick::Queryable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use wx_pay::{
    Amount,
    Jsapi,
    Payer,
    WxPay,
    WxPayData,
    // decode::{WxPayNotify, decode_wx_pay},
};

use crate::common::{
    WECHAT_MINI_APP_ID, WECHAT_PAY_APIV3, WECHAT_PAY_MCH_ID, WECHAT_PAY_NOTIFY_URL,
    WECHAT_PAY_PUBKEY, WECHAT_PAY_SERIAL, WECHAT_PRIVATE_KEY,
};
use crate::control::app_data::{AppData, SlownWorker};
use crate::db::mysql_conn;
use crate::middleware::AuthUser;
use crate::routes::Res;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct TestPay {
    total: u32,
}
/// 【测试】微信支付测试
#[allow(unused)]
#[utoipa::path(
    request_body = TestPay,
    responses((status = 200, description = "【请求：TestPay】【返回：String】", body = String)),
)]
#[post("/pay/make/wx/test")]
pub async fn pay_make_wx_test(
    user: AuthUser,
    params: web::Json<TestPay>,
    app_data: web::Data<AppData>,
) -> Result<impl Responder> {
    let data = &app_data;
    // 查寻当前用户
    let mut conn = mysql_conn()?;
    let user_info: Option<(u64, Option<String>, Option<String>)> = conn
        .query_first(
            "select id,nickname,openid from usr_silent where id = ".to_string()
                + user.id.to_string().as_str(),
        )
        .unwrap();

    if let Some((_uid, _nickname, u_openid)) = user_info {
        if let Some(openid) = u_openid {
            println!("privkkkkkkkkkk, {}", openid);
            let wx_pay = WxPay {
                appid: WECHAT_MINI_APP_ID,
                mchid: WECHAT_PAY_MCH_ID,
                private_key: WECHAT_PRIVATE_KEY,
                serial_no: WECHAT_PAY_SERIAL,
                api_v3_private_key: WECHAT_PAY_APIV3,
                notify_url: WECHAT_PAY_NOTIFY_URL,
                wx_public_key: Some(WECHAT_PAY_PUBKEY),
                wx_public_key_id: None,
            };

            let data = wx_pay
                .jsapi(&Jsapi {
                    description: "测试122".to_string(),
                    out_trade_no: data.rand_no(SlownWorker::OutTradeNo),
                    amount: Amount {
                        total: 1,
                        ..Default::default()
                    },
                    payer: Payer { openid },
                    ..Default::default()
                })
                .await
                .unwrap();

            println!("jsapi 返回的 wx_data 为： {:#?}", data);

            return Ok(web::Json(Res::success(data)));
        }
    }

    Ok(web::Json(Res::success(WxPayData {
        app_id: None,
        sign_type: "".into(),
        pay_sign: "".into(),
        package: "".into(),
        nonce_str: "".into(),
        time_stamp: "".into(),
    })))
}
