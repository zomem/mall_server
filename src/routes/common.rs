use std::collections::HashMap;

use actix_web::{Responder, Result, get, post, web};
use base64::{Engine as _, engine::general_purpose};
use mysql_quick::{Queryable, myfind, myget, myset, myupdate};
use serde::{Deserialize, Serialize};

use serde_json::json;
use utoipa::ToSchema;

use crate::control::frequency::freq_user_day;
use crate::control::sms::sms_send_code;
use crate::control::wx_info::{get_wx_mini_access_token, sign_wx_gzh_jssdk};
use crate::db::{my_run_drop, my_run_vec, mysql_conn};
use crate::middleware::AuthUser;
use crate::routes::{BaseInfo, BaseStrInfo, PdCat, Res};
use crate::utils::files::get_file_urls;

#[derive(Serialize, Clone, Debug, ToSchema)]
pub struct AreaItem {
    aid: String,
    name: String,
    code: u64,
}
#[derive(Serialize, Clone, Debug, ToSchema)]
pub struct CityItem {
    cid: String,
    name: String,
    code: u64,
    children: Vec<AreaItem>,
}
#[derive(Serialize, Clone, Debug, ToSchema)]
pub struct ProvItem {
    id: u64,
    name: String,
    code: u64,
    pid: String,
    children: Vec<CityItem>,
}
/// 【通用】省市区列表
#[utoipa::path(
    responses((status = 200, description = "【返回：ProvItem[]】", body = Vec<ProvItem>)),
    params(("grade", description="1 为省列表，2 为省市列表， 3 为省市区列表"))
)]
#[get("/common/province/list/{grade}")]
pub async fn common_province_list(grade: web::Path<String>) -> Result<impl Responder> {
    let grade_num = grade.parse::<u8>().unwrap();
    let mut conn = mysql_conn()?;

    // #[derive(Serialize, Clone, Copy)]
    struct ProvinceItem {
        id: u64,
        code: u64,
        name: String,
        province: String,
        city: String,
        area: String,
        town: String,
    }

    let temp_list = conn
        .query_map(
            "select * from cmn_province",
            |(id, code, name, province, city, area, town)| ProvinceItem {
                id,
                code,
                name,
                province,
                city,
                area,
                town,
            },
        )
        .unwrap();

    let mut list: Vec<ProvItem> = vec![];
    for item in &temp_list {
        if item.city == "0".to_string() {
            list.push(ProvItem {
                id: item.id,
                name: item.name.clone(),
                code: item.code,
                pid: item.province.clone(),
                children: vec![],
            });
        }
    }
    if grade_num == 2 || grade_num == 3 {
        let all_prov = list.clone();
        for item in &temp_list {
            if item.city != "0".to_string() && item.area == "0".to_string() {
                for (i, v) in all_prov.iter().enumerate() {
                    if v.pid == item.province {
                        list[i].children.push(CityItem {
                            cid: item.city.clone(),
                            name: item.name.clone(),
                            code: item.code,
                            children: vec![],
                        });
                    }
                }
            }
        }
    }
    let all_prov_city = list.clone();
    let mut new_all_prov_city = list.clone();
    if grade_num == 2 || grade_num == 3 {
        for (i, v) in all_prov_city.iter().enumerate() {
            if v.children.len() == 0 {
                new_all_prov_city[i].children.push(CityItem {
                    cid: "01".to_string(),
                    name: v.name.clone(),
                    code: v.code,
                    children: vec![],
                });
                list[i].children = new_all_prov_city[i].children.clone();
            }
        }
    }
    if grade_num == 3 {
        for item in &temp_list {
            if item.city != "0".to_string()
                && item.area != "0".to_string()
                && item.town == "0".to_string()
            {
                for (i, v) in new_all_prov_city.iter().enumerate() {
                    if v.pid == item.province {
                        for (i2, v2) in v.children.iter().enumerate() {
                            if v2.cid == item.city {
                                list[i].children[i2].children.push(AreaItem {
                                    aid: item.area.clone(),
                                    name: item.name.clone(),
                                    code: item.code,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct BaseData {
    /// 产品类别列表
    product_cat: Vec<BaseInfo>,
    /// 产品的物流类型
    delivery_type: Vec<BaseStrInfo>,
    /// 支付方式
    pay_type: Vec<BaseStrInfo>,
    /// 交易记录类型
    tran_type: Vec<BaseStrInfo>,
    /// 产品显示的布局方式
    product_layout: Vec<BaseStrInfo>,
}
/// 【通用】基本公用信息
#[utoipa::path(
    responses((status = 200, description = "【返回：BaseData】", body = BaseData)),
)]
#[get("/common/base/info")]
pub async fn common_base_info() -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let mut pd_list = BaseData {
        product_cat: vec![],
        delivery_type: vec![],
        pay_type: vec![],
        tran_type: vec![],
        product_layout: vec![],
    };

    let sql = myfind!("spu_cat", {
        p0: ["is_del", "=", 0],
        r: "p0",
    });
    let list: Vec<PdCat> = my_run_vec(&mut conn, sql).unwrap();
    // 一级
    for i in 0..list.len() {
        if list[i].primary_id > 0 && list[i].secondary_id == 0 && list[i].tertiary_id == 0 {
            pd_list.product_cat.push(BaseInfo {
                value: list[i].primary_id,
                label: list[i].name.clone(),
                children: vec![],
            });
        }
    }
    // 二级
    for i in 0..list.len() {
        if list[i].primary_id != 0 && list[i].secondary_id > 0 && list[i].tertiary_id == 0 {
            for j in 0..pd_list.product_cat.len() {
                if pd_list.product_cat[j].value == list[i].primary_id {
                    pd_list.product_cat[j].children.push(BaseInfo {
                        value: list[i].secondary_id,
                        label: list[i].name.clone(),
                        children: vec![],
                    });
                }
            }
        }
    }
    // 三级
    for i in 0..list.len() {
        if list[i].primary_id != 0 && list[i].secondary_id != 0 && list[i].tertiary_id > 0 {
            for j in 0..pd_list.product_cat.len() {
                if pd_list.product_cat[j].value == list[i].primary_id {
                    for k in 0..pd_list.product_cat[j].children.len() {
                        if pd_list.product_cat[j].children[k].value == list[i].secondary_id {
                            pd_list.product_cat[j].children[k].children.push(BaseInfo {
                                value: list[i].tertiary_id,
                                label: list[i].name.clone(),
                                children: vec![],
                            });
                        }
                    }
                }
            }
        }
    }

    // 获取产品的物流类别
    let sql = myfind!("sys_constants", {
        p0: ["key", "=", "delivery_type"],
        p1: ["is_del", "=", 0],
        r: "p0 && p1",
    });
    let list: Vec<BaseStrInfo> = my_run_vec(&mut conn, sql)?;
    pd_list.delivery_type = list;

    // 获取产品的支付类别
    let sql = myfind!("sys_constants", {
        p0: ["key", "=", "pay_type"],
        p1: ["is_del", "=", 0],
        r: "p0 && p1",
    });
    let list: Vec<BaseStrInfo> = my_run_vec(&mut conn, sql)?;
    pd_list.pay_type = list;

    // 获取产品的交易类别
    let sql = myfind!("sys_constants", {
        p0: ["key", "=", "tran_type"],
        p1: ["is_del", "=", 0],
        r: "p0 && p1",
    });
    let list: Vec<BaseStrInfo> = my_run_vec(&mut conn, sql)?;
    pd_list.tran_type = list;

    // 获取产品布局方式
    let sql = myfind!("sys_constants", {
        p0: ["key", "=", "product_layout"],
        p1: ["is_del", "=", 0],
        r: "p0 && p1",
    });
    let list: Vec<BaseStrInfo> = my_run_vec(&mut conn, sql)?;
    pd_list.product_layout = list;

    Ok(web::Json(Res::success(pd_list)))
}

#[derive(Serialize, Deserialize, Debug)]
struct CheckResult {
    label: u16,
    suggest: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CheckContent {
    content: String,
}
#[post("/common/check/content")]
pub async fn common_check_content(
    user: AuthUser,
    params: web::Json<CheckContent>,
) -> Result<impl Responder> {
    #[derive(Serialize, Deserialize, Debug)]
    struct CheckRes {
        result: CheckResult,
    }

    if params.content.clone() == String::from("") {
        return Ok(web::Json(Res::<u8>::info(1, "检测通过")));
    }

    let mut conn = mysql_conn()?;
    let res_user: Vec<UserOpenId> =
        my_run_vec(&mut conn, myget!("usr_silent", user.id, "id,openid"))?;
    let user_temp_info: Option<String> = if res_user.len() > 0 {
        Some(res_user[0].openid.clone())
    } else {
        None
    };
    if let Some(user_openid) = user_temp_info {
        let at_v = get_wx_mini_access_token().await?;
        let client = reqwest::Client::new();
        let data = json!({
            "content": params.content,
            "version": 2,
            "scene": 1,
            "openid": user_openid
        });
        let check_res: CheckRes = client
            .post(
                "https://api.weixin.qq.com/wxa/msg_sec_check?access_token=".to_string()
                    + at_v.as_str(),
            )
            .json(&data)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        if check_res.result.suggest == String::from("risky") {
            match check_res.result.label {
                100 => return Ok(web::Json(Res::<u8>::info(1, "检测通过"))),
                10001 => return Ok(web::Json(Res::<u8>::info(0, "文本包含广告内容"))),
                20001 => return Ok(web::Json(Res::<u8>::info(0, "文本包含时政内容"))),
                20002 => return Ok(web::Json(Res::<u8>::info(0, "文本包含色情内容"))),
                20003 => return Ok(web::Json(Res::<u8>::info(0, "文本包含辱骂内容"))),
                20006 => return Ok(web::Json(Res::<u8>::info(0, "文本包含违法犯罪内容"))),
                20008 => return Ok(web::Json(Res::<u8>::info(0, "文本包含欺诈内容"))),
                20012 => return Ok(web::Json(Res::<u8>::info(0, "文本包含低俗内容"))),
                20013 => return Ok(web::Json(Res::<u8>::info(0, "文本包含版权内容"))),
                21000 => return Ok(web::Json(Res::<u8>::info(0, "文本包含违规内容"))),
                _ => return Ok(web::Json(Res::<u8>::info(1, "检测通过"))),
            }
        } else {
            return Ok(web::Json(Res::<u8>::info(1, "检测通过")));
        }
    } else {
        return Ok(web::Json(Res::<u8>::info(1, "检测通过")));
    };
}

#[derive(Serialize, Deserialize, Clone)]
struct CheckListItem {
    id: u64,
    uid: u64,
    media_name: String,
    media_type: u8,
    media_url: String,
    trace_id: String,
    suggest: String,
    label: u16,
    local_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct UserOpenId {
    id: u64,
    openid: String,
}

/// 检测文件数据库的缓存
#[derive(Serialize, Deserialize, Debug)]
pub struct UserMedia {
    media_name: String,
}
#[post("/common/check/one")]
pub async fn common_check_one(
    user: AuthUser,
    params: web::Json<UserMedia>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let media_list: Vec<CheckListItem> = my_run_vec(
        &mut conn,
        myfind!("check_list", {
            p0: ["uid", "=", user.id],
            p1: ["media_name", "=", params.media_name.clone()],
            r: "p0 && p1",
            select: "id,uid,media_name,media_type,media_url,trace_id,suggest,label,local_path",
        }),
    )?;
    // 命中标签枚举值，100 正常；20001 时政；20002 色情；20006 违法犯罪；21000 其他
    let list = media_list.get(0);
    if let Some(item) = list {
        // 当前用户的这个文件，已经缓存，直接返回成功
        let path = item.local_path.clone();
        let sug = item.suggest.clone();
        let label = item.label;
        if sug == String::from("risky") {
            match label {
                100 => {
                    return Ok(web::Json(Res {
                        status: 1,
                        message: "检测通过".to_string(),
                        objects: Some(path),
                    }));
                }
                20001 => {
                    return Ok(web::Json(Res {
                        status: 0,
                        message: "包含时政内容".to_string(),
                        objects: Some(path),
                    }));
                }
                20002 => {
                    return Ok(web::Json(Res {
                        status: 0,
                        message: "包含色情内容".to_string(),
                        objects: Some(path),
                    }));
                }
                20006 => {
                    return Ok(web::Json(Res {
                        status: 0,
                        message: "包含违法犯罪内容".to_string(),
                        objects: Some(path),
                    }));
                }
                21000 => {
                    return Ok(web::Json(Res {
                        status: 0,
                        message: "包含违规内容".to_string(),
                        objects: Some(path),
                    }));
                }
                _ => {
                    return Ok(web::Json(Res {
                        status: 1,
                        message: "检测通过".to_string(),
                        objects: Some(path),
                    }));
                }
            }
        } else {
            return Ok(web::Json(Res {
                status: 1,
                message: "检测通过".to_string(),
                objects: Some(path),
            }));
        }
    } else {
        // 数据库没有，则要上传后，检测
        return Ok(web::Json(Res {
            status: 1,
            message: "未检测".to_string(),
            objects: Some("".to_string()),
        }));
    }
}

/// 微信小程序，一张的，图片音频安全检测
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckMedia {
    media_type: u8, // 1:音频;2:图片
    media_url: String,
    media_name: String,
    local_path: String,
}
#[post("/common/check/media")]
pub async fn common_check_media(
    user: AuthUser,
    params: web::Json<CheckMedia>,
) -> Result<impl Responder> {
    #[derive(Serialize, Deserialize, Debug)]
    struct CheckRes {
        errcode: u16,
        errmsg: String,
        trace_id: String,
    }

    if params.media_url.clone() == String::from("") {
        return Ok(web::Json(Res::<u8>::info(1, "检测通过")));
    }

    let mut conn = mysql_conn()?;
    let res_user: Vec<UserOpenId> =
        my_run_vec(&mut conn, myget!("usr_silent", user.id, "id,openid"))?;
    let user_temp_info: Option<String> = if res_user.len() > 0 {
        Some(res_user[0].openid.clone())
    } else {
        None
    };
    if let Some(user_openid) = user_temp_info {
        let at_v = get_wx_mini_access_token().await?;
        let client = reqwest::Client::new();
        let data = json!({
            "media_url": params.media_url.clone(),
            "version": 2,
            "scene": 1,
            "media_type": params.media_type.clone(),
            "openid": user_openid
        });

        let check_res: CheckRes = client
            .post(
                "https://api.weixin.qq.com/wxa/media_check_async?access_token=".to_string()
                    + at_v.as_str(),
            )
            .json(&data)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        // 将该条记录，保存到数据库
        my_run_drop(
            &mut conn,
            myset!("check_list", {
                "uid": user.id,
                "media_name": params.media_name.clone(),
                "media_type": params.media_type,
                "media_url": params.media_url.clone(),
                "local_path": params.local_path.clone(),
                "trace_id": check_res.trace_id.clone(),
            }),
        )?;
        return Ok(web::Json(Res::<u8>::info(1, "检测通过")));
    } else {
        return Ok(web::Json(Res::<u8>::info(1, "检测通过")));
    };
}

#[derive(Serialize, Deserialize, Debug)]
struct WxMessage {
    signature: String,
    timestamp: u64,
    nonce: u64,
    echostr: String,
}
// 微信接口 服务器验证 （消息推送，公众号开发等）
// #[get("/common/wx/message/verify")]
// pub async fn common_wx_message_verify(query: web::Query<WxMessage>) -> String {
//     println!("微信验证————————  {:#?}", query);
//     println!("微信验证—echostr———————  {:#?}", query.0.echostr);

//     query.0.echostr
// }

#[derive(Serialize, Deserialize, Debug)]
struct WxGzhCode {
    code: String,
    state: String,
}
// 微信接口 服务器验证 （消息推送，公众号开发等）  ?code=CODE&state=STATE。
#[get("/common/wx/message/verify")]
pub async fn common_wx_message_verify(query: web::Query<WxGzhCode>) -> String {
    println!("微信验证————————  {:#?}", query);
    println!("微信验证—code———————  {:#?}", query.0.code);

    String::from("success")
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WxCheckMsg {
    result: CheckResult,
    trace_id: String,
}
// 接收微信消息推送
#[post("/common/wx/message")]
pub async fn common_wx_message(params: web::Json<WxCheckMsg>) -> String {
    // println!("微信消息————————  {:#?}", params);
    // 将微信的结果，更新
    let mut conn = mysql_conn().unwrap();
    my_run_drop(
        &mut conn,
        myupdate!("check_list", { "trace_id": params.trace_id.clone() }, {
            "suggest": params.result.suggest.clone(),
            "label": params.result.label,
        }),
    )
    .unwrap();

    String::from("success")
}

#[derive(Serialize, Deserialize, Clone)]
struct BannerInfo {
    id: u32,
    img_urls: String,
    path_urls: Option<String>,
    name: String,
    page: Option<String>,
    color: Option<String>,
}
#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct BannerRes {
    id: u32,
    /// 图片链接 数组
    imgs: Vec<String>,
    /// 点击图片后的跳转 路径。和图片数组对应
    path_urls: Vec<String>,
    /// 名字
    name: String,
    /// 显示 图片的页面
    page: Option<String>,
    /// 主色
    color: Option<String>,
}
/// 【通用】Banner列表
#[utoipa::path(
    responses((status = 200, description = "【返回：BannerRes】", body = Vec<BannerRes>)),
)]
#[get("/common/banner/list")]
pub async fn common_banner_list() -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    let list: Vec<BannerInfo> = my_run_vec(
        &mut conn,
        myfind!("cmn_banner", {
            p0: ["status", "=", 2],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "id,img_urls, path_urls, name, page, color",
        }),
    )?;
    let list: Vec<BannerRes> = list
        .into_iter()
        .map(|x| BannerRes {
            id: x.id,
            name: x.name,
            imgs: get_file_urls(Some(&x.img_urls)),
            page: x.page,
            path_urls: if let Some(p) = x.path_urls {
                if p == String::default() {
                    vec![]
                } else {
                    p.split(",").map(|i| i.to_string()).collect::<Vec<String>>()
                }
            } else {
                vec![]
            },
            color: x.color,
        })
        .collect();

    Ok(web::Json(Res::success(Some(list))))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WxQRCode {
    id: u64,
}
/// 获取小程序码
#[post("/common/wx_qrcode")]
pub async fn common_wx_qrcode(params: web::Json<WxQRCode>) -> Result<impl Responder> {
    let at_v = get_wx_mini_access_token().await?;
    let client = reqwest::Client::new();
    let data = json!({
        "page": "pages/wifi/wifi",
        "scene": params.id,
    });

    let qrcode_res = client
        .post(
            "https://api.weixin.qq.com/wxa/getwxacodeunlimit?access_token=".to_string()
                + at_v.as_str(),
        )
        .json(&data)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let b64 = general_purpose::STANDARD.encode(qrcode_res);

    Ok(web::Json(Res::success(b64)))
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct SmsCodePhone {
    phone: String,
}
///【登录】短信验证码
#[utoipa::path(
    request_body = SmsCodePhone,
    responses((status = 200, description = "【请求：SmsCodePhone】【返回：String】", body = String)),
)]
#[post("/common/sms_code")]
pub async fn common_sms_code(
    user: AuthUser,
    params: web::Json<SmsCodePhone>,
) -> Result<impl Responder> {
    let uid = user.id;

    // 对发送短信的接口，限制每天，同一用户为5次
    freq_user_day(uid, "common_sms_code", 5)?;

    let send_sms = sms_send_code(&params.phone).await;
    if send_sms.is_ok() {
        Ok(web::Json(Res::success("验证码已发送")))
    } else {
        Ok(web::Json(Res::fail("验证码发送失败")))
    }
}

/// 【通用】公众号sdk签名
#[utoipa::path(
    responses((status = 200, description = "【返回：WxJsSdkSign】", body = WxJsSdkSign)),
)]
#[get("/common/wx/js_sdk/sign")]
pub async fn common_wx_js_sdk_sign() -> Result<impl Responder> {
    let sign_res = sign_wx_gzh_jssdk().await?;
    Ok(web::Json(Res::success(sign_res)))
}

/// 【通用】功能模块状态
#[utoipa::path(
    responses((status = 200, description = "【返回：编号-是否启用】如：COUPON: true，表示优惠券功能启用", body = Res<u8>)),
)]
#[get("/common/module/switch_list")]
pub async fn common_module_switch_list() -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    #[derive(Serialize, Clone, Deserialize)]
    struct ModuleSwitchGet {
        id: u32,
        name: String,
        code: String,
        is_on: u8,
    }
    let list: Vec<ModuleSwitchGet> = my_run_vec(
        &mut conn,
        myfind!("sys_module_switch", {
            p0: ["is_del", "=", 0],
            r: "p0",
            select: "id,name,is_on,code",
        }),
    )?;
    let mut hash = HashMap::new();

    for x in list.into_iter() {
        let temp_on = if x.is_on == 1 { true } else { false };
        hash.insert(x.code, temp_on);
    }
    Ok(web::Json(Res::success(hash)))
}

/// 测试用
#[get("/common/test/{id}")]
pub async fn common_test(id: web::Path<String>) -> Result<impl Responder> {
    println!("%%%%%%%%%%%%%%%%%%%%%%%% {}", id);
    Ok(web::Json(CommonTest {
        name: id.to_string(),
        age: 88,
        avatar: Some("avatthhhhhh".to_owned()),
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommonTest {
    name: String,
    age: u8,
    avatar: Option<String>,
}
/// 测试post
#[post("/common/test/post")]
pub async fn common_test_post(data: web::Json<CommonTest>) -> Result<impl Responder> {
    println!("%%%ZZZZZZZZZZZZZ  {:?}", data);

    Ok(web::Json({}))
}
