use actix_web::error::{self, Error};
use reqwest;
use serde::{Deserialize, Serialize};

use super::wx_info::get_wx_mini_access_token;

/// 收件人信息
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryReceiver {
    /// 收件人姓名，不超过64字节
    pub name: Option<String>,
    /// 收件人座机号码，若不填写则必须填写 mobile，不超过32字节
    pub tel: Option<String>,
    /// 收件人手机号码，若不填写则必须填写 tel，不超过32字节
    pub mobile: Option<String>,
    /// 收件人公司名称，不超过64字节
    pub company: Option<String>,
    /// 收件人邮编，不超过10字节
    pub post_code: Option<String>,
    /// 收件人国家，不超过64字节
    pub country: Option<String>,
    /// 收件人省份，比如："广东省"，不超过64字节
    pub province: Option<String>,
    /// 收件人市/地区，比如："广州市"，不超过64字节
    pub city: Option<String>,
    /// 收件人区/县，比如："海珠区"，不超过64字节
    pub area: Option<String>,
    /// 收件人详细地址，比如："XX路XX号XX大厦XX"，不超过512字节
    pub address: Option<String>,
}
/// 发件人信息
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliverySender {
    /// 发件人姓名，不超过64字节
    pub name: Option<String>,
    /// 发件人座机号码，若不填写则必须填写 mobile，不超过32字节
    pub tel: Option<String>,
    /// 发件人手机号码，若不填写则必须填写 tel，不超过32字节
    pub mobile: Option<String>,
    /// 发件人公司名称，不超过64字节
    pub company: Option<String>,
    /// 发件人邮编，不超过10字节
    pub post_code: Option<String>,
    /// 发件人国家，不超过64字节
    pub country: Option<String>,
    /// 发件人省份，比如："广东省"，不超过64字节
    pub province: Option<String>,
    /// 发件人市/地区，比如："广州市"，不超过64字节
    pub city: Option<String>,
    /// 发件人区/县，比如："海珠区"，不超过64字节
    pub area: Option<String>,
    /// 发件人详细地址，比如："XX路XX号XX大厦XX"，不超过512字节
    pub address: Option<String>,
}
/// 包裹信息，将传递给快递公司
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryCargoDetail {
    /// 商品名，不超过128字节
    pub name: String,
    /// 商品数量
    pub count: u32,
}
/// 包裹信息，将传递给快递公司
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryCargo {
    /// 包裹数量, 默认为1
    pub count: u32,
    /// 货物总重量，比如1.2，单位是千克(kg)
    pub weight: f64,
    /// 货物长度，比如20.0，单位是厘米(cm)
    pub space_x: f64,
    /// 货物宽度，比如15.0，单位是厘米(cm)
    pub space_y: f64,
    /// 货物高度，比如10.0，单位是厘米(cm)
    pub space_z: f64,
    /// 货物详情
    pub detail_list: Vec<WxDeliveryCargoDetail>,
}
/// 商品信息详情
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryShopDetail {
    /// 商品缩略图
    pub img_url: Option<String>,
    /// 商品名称
    pub goods_name: Option<String>,
}
/// 商品信息，会展示到物流服务通知和电子面单中
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryShop {
    /// 商家小程序的路径，建议为订单页面
    pub wxa_path: Option<String>,
    /// 商品缩略图 url；shop.detail_list为空则必传，shop.detail_list非空可不传。
    pub img_url: Option<String>,
    /// 商品名称, 不超过128字节；shop.detail_list为空则必传，shop.detail_list非空可不传。
    pub goods_name: Option<String>,
    /// 商品数量；shop.detail_list为空则必传。shop.detail_list非空可不传，默认取shop.detail_list的size
    pub goods_count: u32,
    /// 商品详情列表，适配多商品场景，用以消息落地页展示。（新规范，新接入商家建议用此字段）
    pub detail_list: Vec<WxDeliveryShopDetail>,
}
/// 保价信息
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryInsured {
    /// 是否保价，0 表示不保价，1 表示保价
    pub use_insured: Option<u8>,
    /// 保价金额，单位是分，比如: 10000 表示 100 元
    pub insured_value: Option<u32>,
}
/// 服务类型
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryService {
    /// 服务类型ID，详见已经支持的快递公司基本信息
    pub service_type: u32,
    /// 服务名称，详见已经支持的快递公司基本信息
    pub service_name: String,
}
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryAdd {
    /// 订单ID，须保证全局唯一，不超过512字节
    pub order_id: String,
    /// 用户openid，当add_source=2时无需填写（不发送物流服务通知）
    pub openid: String,
    /// 快递公司ID，参见getAllDelivery
    pub delivery_id: String,
    /// 快递客户编码或者现付编码
    pub biz_id: String,
    /// 快递备注信息，比如"易碎物品"，不超过1024字节
    pub custom_remark: Option<String>,
    /// 订单标签id，用于平台型小程序区分平台上的入驻方，tagid须与入驻方账号一一对应，非平台型小程序无需填写该字段
    pub tagid: u64,
    /// 订单来源，0为小程序订单，2为App或H5订单，填2则不发送物流服务通知
    pub add_source: u8,
    /// App或H5的appid，add_source=2时必填，需和开通了物流助手的小程序绑定同一open帐号
    pub wx_appid: Option<String>,
    /// 收件人信息
    pub sender: WxDeliverySender,
    /// 发件人信息
    pub receiver: WxDeliveryReceiver,
    /// 包裹信息，将传递给快递公司
    pub cargo: WxDeliveryCargo,
    /// 商品信息，会展示到物流服务通知和电子面单中
    pub shop: WxDeliveryShop,
    /// 保价信息
    pub insured: WxDeliveryInsured,
    /// 服务类型
    pub service: WxDeliveryService,
    /// Unix 时间戳, 单位秒，顺丰必须传。 预期的上门揽件时间，0表示已事先约定取件时间；
    /// 否则请传预期揽件时间戳，需大于当前时间，收件员会在预期时间附近上门。
    /// 例如expect_time为“1557989929”，
    /// 表示希望收件员将在2019年05月16日14:58:49-15:58:49内上门取货。
    /// 说明：若选择 了预期揽件时间，请不要自己打单，由上门揽件的时候打印。
    /// 如果是下顺丰散单，则必传此字段，否则不会有收件员上门揽件。
    pub expect_time: u64,
    /// 分单策略，【0：线下网点签约，1：总部签约结算】，不传默认线下网点签约。目前支持圆通。
    pub take_mode: Option<u8>,
}
/// 运单信息，下单成功时返回
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryWaybillData {
    /// 运单信息 key
    pub key: String,
    ///	运单信息 value
    pub value: String,
}
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryInfo {
    /// 微信侧错误码，下单失败时返回
    pub errcode: Option<u32>,
    /// 微信侧错误信息，下单失败时返回
    pub errmsg: Option<String>,
    /// 订单ID，下单成功时返回
    pub order_id: String,
    /// 运单ID，下单成功时返回
    pub waybill_id: String,
    /// 快递侧错误码，下单失败时返回
    pub delivery_resultcode: Option<u32>,
    /// 快递侧错误信息，下单失败时返回
    pub delivery_resultmsg: Option<String>,
    /// 运单信息，下单成功时返回
    pub waybill_data: Vec<WxDeliveryWaybillData>,
}

/// 微信物流，[生成运单](https://developers.weixin.qq.com/miniprogram/dev/OpenApiDoc/express/express-by-business/addOrder.html)
#[allow(unused)]
pub async fn add_wx_delivery_order(body: &WxDeliveryAdd) -> anyhow::Result<WxDeliveryInfo, Error> {
    let access_token = get_wx_mini_access_token().await?;
    let url = "https://api.weixin.qq.com/cgi-bin/express/business/order/add?access_token="
        .to_string()
        + &access_token;

    let client = reqwest::Client::new();
    let res: WxDeliveryInfo = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?
        .json()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?;

    Ok(res)
}

/// 获取运单数据
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryBody {
    /// 订单 ID，需保证全局唯一
    pub order_id: String,
    ///	该参数仅在getOrder接口生效，batchGetOrder接口不生效。用户openid，当add_source=2时无需填写（不发送物流服务通知）
    pub openid: Option<String>,
    ///	快递公司ID，参见getAllDelivery, 必须和waybill_id对应
    pub delivery_id: String,
    ///	运单ID
    pub waybill_id: Option<String>,
    ///	该参数仅在getOrder接口生效，batchGetOrder接口不生效。获取打印面单类型【1：一联单，0：二联单】，默认获取二联单
    pub print_type: Option<u8>,
    pub custom_remark: Option<String>,
}
/// 运单数据
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryGetInfo {
    /// 微信侧错误码，下单失败时返回
    pub errcode: Option<u32>,
    /// 微信侧错误信息，下单失败时返回
    pub errmsg: Option<String>,
    /// 运单 html 的 BASE64 结果
    pub print_html: String,
    /// 运单信息，下单成功时返回
    pub waybill_data: Vec<WxDeliveryWaybillData>,
    /// 订单ID
    pub order_id: String,
    /// 快递公司ID
    pub delivery_id: String,
    /// 运单号
    pub waybill_id: String,
    /// 运单状态, 0正常，1取消
    pub order_status: u8,
}
/// [获取运单数据](https://developers.weixin.qq.com/miniprogram/dev/OpenApiDoc/express/express-by-business/getOrder.html)
#[allow(unused)]
pub async fn get_wx_delivery_order(
    body: &WxDeliveryBody,
) -> anyhow::Result<WxDeliveryGetInfo, Error> {
    let access_token = get_wx_mini_access_token().await?;
    let url = "https://api.weixin.qq.com/cgi-bin/express/business/order/get?access_token="
        .to_string()
        + &access_token;

    let client = reqwest::Client::new();
    let res: WxDeliveryGetInfo = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?
        .json()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?;

    Ok(res)
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryWaybillPath {
    ///	轨迹节点 Unix 时间戳
    pub action_time: u64,
    /// 轨迹节点类型
    pub action_type: u64,
    /// 轨迹节点详情
    pub action_msg: String,
}
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryWaybillInfo {
    pub openid: Option<String>,
    ///	快递公司ID，参见getAllDelivery
    pub delivery_id: String,
    /// 运单ID
    pub waybill_id: String,
    /// 轨迹节点数量
    pub path_item_num: u32,
    /// 轨迹节点列表
    pub path_item_list: Vec<WxDeliveryWaybillPath>,
}
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryWaybillBody {
    /// 用户openid，当add_source=2时无需填写（不发送物流服务通知）
    pub openid: Option<String>,
    ///	快递公司ID，参见getAllDelivery
    pub delivery_id: String,
    /// 运单ID
    pub waybill_id: String,
}
/// [查询运单轨迹](https://developers.weixin.qq.com/miniprogram/dev/OpenApiDoc/express/express-by-business/getPath.html)
#[allow(unused)]
pub async fn get_wx_delivery_waybill(
    body: &WxDeliveryWaybillBody,
) -> anyhow::Result<WxDeliveryWaybillInfo, Error> {
    let access_token = get_wx_mini_access_token().await?;
    let url = "https://api.weixin.qq.com/cgi-bin/express/business/path/get?access_token="
        .to_string()
        + &access_token;

    let client = reqwest::Client::new();
    let res: WxDeliveryWaybillInfo = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?
        .json()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?;

    Ok(res)
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryWaybillCancelInfo {
    /// 微信侧错误码，下单失败时返回
    pub errcode: Option<u32>,
    /// 微信侧错误信息，下单失败时返回
    pub errmsg: Option<String>,
    /// 快递侧错误码，下单失败时返回
    pub delivery_resultcode: Option<u32>,
    /// 快递侧错误信息，下单失败时返回
    pub delivery_resultmsg: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryWaybillCancelBody {
    /// 用户openid，当add_source=2时无需填写（不发送物流服务通知）
    pub openid: Option<String>,
    ///	快递公司ID，参见getAllDelivery
    pub delivery_id: String,
    /// 运单ID
    pub waybill_id: String,
    /// 订单 ID，需保证全局唯一
    pub order_id: String,
}
/// [取消运单](https://developers.weixin.qq.com/miniprogram/dev/OpenApiDoc/express/express-by-business/cancelOrder.html)
#[allow(unused)]
pub async fn cancel_wx_delivery(
    body: &WxDeliveryWaybillCancelBody,
) -> anyhow::Result<WxDeliveryWaybillCancelInfo, Error> {
    let access_token = get_wx_mini_access_token().await?;
    let url = "https://api.weixin.qq.com/cgi-bin/express/business/order/cancel?access_token="
        .to_string()
        + &access_token;

    let client = reqwest::Client::new();
    let res: WxDeliveryWaybillCancelInfo = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?
        .json()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?;

    // 返回示例
    // {
    //   "errcode": 0,
    //   "errmsg": "ok",
    //   "delivery_resultcode": 0,
    //   "delivery_resultmsg": ""
    // }
    Ok(res)
}

///快递公司信息列表
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDelivery {
    /// 快递公司 ID
    pub delivery_id: String,
    ///	快递公司名称
    pub delivery_name: String,
    /// 是否支持散单, 1表示支持
    pub can_use_cash: u8,
    /// 是否支持查询面单余额, 1表示支持
    pub can_get_quota: u8,
    /// 散单对应的bizid，当can_use_cash=1时有效
    pub cash_biz_id: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WxDeliveryAll {
    /// 快递公司数量
    pub count: u16,
    ///	快递公司信息列表
    pub data: Vec<WxDelivery>,
}
/// [获取支持的快递公司列表](https://developers.weixin.qq.com/miniprogram/dev/OpenApiDoc/express/express-by-business/getAllDelivery.html)
#[allow(unused)]
pub async fn get_all_wx_delivery() -> anyhow::Result<WxDeliveryAll, Error> {
    let access_token = get_wx_mini_access_token().await?;
    let url = "https://api.weixin.qq.com/cgi-bin/express/business/delivery/getall?access_token="
        .to_string()
        + &access_token;

    let client = reqwest::Client::new();
    let res: WxDeliveryAll = client
        .get(url)
        .send()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?
        .json()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?;

    Ok(res)
}
