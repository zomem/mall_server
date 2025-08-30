use serde::{Deserialize, Serialize};
use strum_macros::Display;
use utoipa::ToSchema;

use super::{FILE_STORAGE_TYPE, PROJECT_NAME, STATIC_FILE_URL};

pub enum OssBucket {
    EobFiles,
}
impl OssBucket {
    pub fn get_name(&self) -> &str {
        match &self {
            OssBucket::EobFiles => {
                if FILE_STORAGE_TYPE == 1 {
                    "local-files"
                } else {
                    "files"
                }
            }
        }
    }
    pub fn get_base_url(&self) -> String {
        match &self {
            OssBucket::EobFiles => {
                if FILE_STORAGE_TYPE == 1 {
                    STATIC_FILE_URL.to_string()
                } else {
                    "https://oshou.aliyuncs.com".to_string()
                }
            }
        }
    }
}

/// oss 或 本地 存储的 目录类型名,分类型和目录，在此配置
#[derive(PartialEq, Eq)]
pub enum FileDir {
    Avatar,
    /// 文章图片
    Article,
    CommonBanner,
    ProductCat,
    Product,
    /// 产品资料文件
    ProductFile,
    Unit,
    UnitAttr,
    /// 用户角色认证的资料图片等
    Credential,
    Brand,
    /// 用户提交表单里的图片文件
    QuestionForm,
    Empty,
}
impl FileDir {
    pub fn get_dir(&self) -> String {
        match &self {
            FileDir::Avatar => format!("{}/avatar", PROJECT_NAME),
            FileDir::Article => format!("{}/article", PROJECT_NAME),
            FileDir::CommonBanner => format!("{}/common/banner", PROJECT_NAME),
            FileDir::ProductCat => format!("{}/system/product_cat", PROJECT_NAME),
            FileDir::Product => format!("{}/product", PROJECT_NAME),
            FileDir::ProductFile => format!("{}/product_file", PROJECT_NAME),
            FileDir::Unit => format!("{}/unit", PROJECT_NAME),
            FileDir::UnitAttr => format!("{}/unit_attr", PROJECT_NAME),
            FileDir::Credential => format!("{}/credential", PROJECT_NAME),
            FileDir::Brand => format!("{}/brand", PROJECT_NAME),
            FileDir::QuestionForm => format!("{}/question_form", PROJECT_NAME),
            FileDir::Empty => "".to_string(),
        }
    }
}
impl From<String> for FileDir {
    fn from(value: String) -> Self {
        match value.as_str() {
            "avatar" => FileDir::Avatar,
            "article" => FileDir::Article,
            "banner" => FileDir::CommonBanner,
            "product_cat" => FileDir::ProductCat,
            "product" => FileDir::Product,
            "product_file" => FileDir::ProductFile,
            "unit" => FileDir::Unit,
            "unit_attr" => FileDir::UnitAttr,
            "credential" => FileDir::Credential,
            "brand" => FileDir::Brand,
            "question_form" => FileDir::QuestionForm,
            _ => FileDir::Empty,
        }
    }
}

/// 产品交易类型 delivery_type
#[allow(unused)]
#[derive(Serialize, Deserialize, Display, PartialEq, Debug, ToSchema, Clone)]
pub enum DeliveryType {
    /// 无需物流
    #[serde(rename = "NO_DELIVERY")]
    #[strum(to_string = "NO_DELIVERY")]
    NoDelivery,
    /// 手动发货
    #[serde(rename = "DO_DELIVERY")]
    #[strum(to_string = "DO_DELIVERY")]
    DoDelivery,
    /// 微信物流助手
    #[serde(rename = "WX_DELIVERY")]
    #[strum(to_string = "WX_DELIVERY")]
    WxDelivery,
    /// 微信即时配送
    #[serde(rename = "WX_INSTANT")]
    #[strum(to_string = "WX_INSTANT")]
    WxInstant,
    /// 到店自提
    #[serde(rename = "DOOR_PICKUP")]
    #[strum(to_string = "DOOR_PICKUP")]
    DoorPickup,
    /// 到店核销
    #[serde(rename = "STORE_WRITE_OFF")]
    #[strum(to_string = "STORE_WRITE_OFF")]
    StoreWriteOff,
}
impl<T> From<T> for DeliveryType
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        match value.as_ref() {
            "NO_DELIVERY" => DeliveryType::NoDelivery,
            "DO_DELIVERY" => DeliveryType::DoDelivery,
            "WX_DELIVERY" => DeliveryType::WxDelivery,
            "WX_INSTANT" => DeliveryType::WxInstant,
            "DOOR_PICKUP" => DeliveryType::DoorPickup,
            "STORE_WRITE_OFF" => DeliveryType::StoreWriteOff,
            _ => DeliveryType::NoDelivery,
        }
    }
}

/// 支付类型
#[derive(Serialize, Deserialize, Display, PartialEq, Debug, ToSchema, Clone)]
pub enum PayType {
    /// 用户钱包余额支付
    #[serde(rename = "POCKET_PAY")]
    #[strum(to_string = "POCKET_PAY")]
    PocketPay,
    /// 微信支付
    #[serde(rename = "WX_PAY")]
    #[strum(to_string = "WX_PAY")]
    WxPay,
    /// 未知支付
    #[serde(rename = "UNKNOWN_PAY")]
    #[strum(to_string = "UNKNOWN_PAY")]
    UnknownPay,
}
impl<T> From<T> for PayType
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        match value.as_ref() {
            "POCKET_PAY" => PayType::PocketPay,
            "WX_PAY" => PayType::WxPay,
            _ => PayType::UnknownPay,
        }
    }
}

/// 交易类型，订单类型
#[allow(unused)]
#[derive(Serialize, Deserialize, Display, PartialEq, Debug, ToSchema, Clone)]
pub enum TranType {
    /// 用户购买商品 -
    #[serde(rename = "PURCHASE")]
    #[strum(to_string = "PURCHASE")]
    Purchase,
    /// 用户提现 -
    #[serde(rename = "WITHDRAW")]
    #[strum(to_string = "WITHDRAW")]
    Withdraw,
    /// 用户充值 +
    #[serde(rename = "RECHARGE")]
    #[strum(to_string = "RECHARGE")]
    Recharge,
    /// 用户退款 +
    #[serde(rename = "REFUND")]
    #[strum(to_string = "REFUND")]
    Refund,
    /// 总销售分账 +
    #[serde(rename = "MAIN_SALE_SPLIT")]
    #[strum(to_string = "MAIN_SALE_SPLIT")]
    MainSaleSplit,
    /// 销售分账 +
    #[serde(rename = "SALE_SPLIT")]
    #[strum(to_string = "SALE_SPLIT")]
    SaleSplit,
    /// 未知交易类型
    #[serde(rename = "UNKNOWN")]
    #[strum(to_string = "UNKNOWN")]
    Unknown,
}
impl From<String> for TranType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "PURCHASE" => TranType::Purchase,
            "WITHDRAW" => TranType::Withdraw,
            "RECHARGE" => TranType::Recharge,
            "REFUND" => TranType::Refund,
            "MAIN_SALE_SPLIT" => TranType::MainSaleSplit,
            "SALE_SPLIT" => TranType::SaleSplit,
            _ => TranType::Unknown,
        }
    }
}

// /// 产品布局方式
#[derive(Serialize, Deserialize, Display, PartialEq, Debug, ToSchema, Clone)]
pub enum ProductLayout {
    /// 整行左图右文
    #[serde(rename = "COVER_TXT_LR")]
    #[strum(to_string = "COVER_TXT_LR")]
    CoverTextLR,
    /// 半行上图下文
    #[serde(rename = "HALF_COVER_TXT_TB")]
    #[strum(to_string = "HALF_COVER_TXT_TB")]
    HalfCoverTextTB,
    /// 整行上图下文
    #[serde(rename = "COVER_TXT_TB")]
    #[strum(to_string = "COVER_TXT_TB")]
    CoverTextTB,
}
impl<T: AsRef<str>> From<T> for ProductLayout {
    fn from(value: T) -> Self {
        match value.as_ref() {
            "COVER_TXT_LR" => ProductLayout::CoverTextLR,
            "HALF_COVER_TXT_TB" => ProductLayout::HalfCoverTextTB,
            "COVER_TXT_TB" => ProductLayout::CoverTextTB,
            _ => ProductLayout::CoverTextLR,
        }
    }
}

/// 问题表单的题目类型
#[derive(Serialize, Deserialize, Display, PartialEq, Debug, ToSchema, Clone)]
pub enum QuestionFormType {
    #[serde(rename = "INPUT")]
    #[strum(to_string = "INPUT")]
    Input,
    /// 微信获取手机号
    #[serde(rename = "PHONE_NUMBER")]
    #[strum(to_string = "PHONE_NUMBER")]
    PhoneNumber,
    #[serde(rename = "TEXTAREA")]
    #[strum(to_string = "TEXTAREA")]
    Textarea,
    #[serde(rename = "SELECT")]
    #[strum(to_string = "SELECT")]
    Select,
    #[serde(rename = "RADIO")]
    #[strum(to_string = "RADIO")]
    Radio,
    #[serde(rename = "CHECK_BOX")]
    #[strum(to_string = "CHECK_BOX")]
    CheckBox,
    #[serde(rename = "IMAGE_SINGLE")]
    #[strum(to_string = "IMAGE_SINGLE")]
    ImageSingle,
    #[serde(rename = "IMAGE_MULTIPLE")]
    #[strum(to_string = "IMAGE_MULTIPLE")]
    ImageMultiple,
}
impl<T: AsRef<str>> From<T> for QuestionFormType {
    fn from(value: T) -> Self {
        match value.as_ref() {
            "INPUT" => QuestionFormType::Input,
            "PHONE_NUMBER" => QuestionFormType::PhoneNumber,
            "TEXTAREA" => QuestionFormType::Textarea,
            "SELECT" => QuestionFormType::Select,
            "RADIO" => QuestionFormType::Radio,
            "CHECK_BOX" => QuestionFormType::CheckBox,
            "IMAGE_SINGLE" => QuestionFormType::ImageSingle,
            "IMAGE_MULTIPLE" => QuestionFormType::ImageMultiple,
            _ => QuestionFormType::Input,
        }
    }
}

/// 角色分类
#[derive(Serialize, Deserialize, PartialEq, Debug, ToSchema, Clone, Copy)]
pub enum Role {
    /// 总销售
    MainSale = 1000,
    /// 销售
    Sale = 1001,
    /// 销售
    Agent = 2000,
    /// 核销员
    WriteOff = 3000,
    /// 无角色
    NoRoles = 0,
}
impl From<u32> for Role {
    fn from(value: u32) -> Self {
        match value {
            1000 => Role::MainSale,
            1001 => Role::Sale,
            2000 => Role::Agent,
            _ => Role::NoRoles,
        }
    }
}

///  通用数据的状态 0为审核不通过 1为审核 2正常上线 3为下架
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub enum NormalStatus {
    /// 0为审核不通过
    NotPass,
    /// 1为审核
    UnderReview,
    /// 2正常上线
    Online,
    /// 3为下架
    OffShelf,
}

/// 用户购物车状态 1为待结算，2 为已结算，3为立即购买 4为立即购买已结算
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, PartialEq, Eq)]
pub enum ShopCartStatus {
    /// 1为待结算
    PendingPayment = 1,
    /// 2为已结算
    Paid,
    /// 3为立即购买
    BuyNow,
    /// 4为立即购买已结算
    BuyNowPaid,
    /// 5 x状态错误
    Wrong,
}
impl From<String> for ShopCartStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "pending" => Self::PendingPayment,
            "paid" => Self::Paid,
            "buy_now" => Self::BuyNow,
            "buy_now_paid" => Self::BuyNowPaid,
            _ => Self::Wrong,
        }
    }
}

/// 用户优惠券状态 1已过期，2未使用，3已使用
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub enum UserCouponStatus {
    /// 已过期
    Expired = 1,
    /// 未使用
    NotUsed,
    /// 已使用
    Used,
}

/// 用户订单支付状态 1 为待支付，2 为已支付，0 为取消支付  4 为申请退款  5 为已退款  6 为退款中
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, Eq, PartialEq)]
pub enum OrderPayStatus {
    /// 0 取消支付
    CancelPayment,
    /// 1 为待支付
    PendingPayment,
    /// 2 为已支付
    Paid,
    /// 4 为申请退款
    Apply = 4,
    /// 5 为已退款
    Refund = 5,
    /// 6 为退款中
    Refunding = 6,
    /// 7 为拒绝退款
    Refuse = 7,
}

/// 用户子订单物流等状态 0 待发货，1 待收货, 2 已完成, 3 已评价，4 申请退货，5 已退货，6 为退款中
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, Eq, PartialEq)]
pub enum OrderItemStatus {
    /// 0 为待发货
    WaitDeliverGoods,
    /// 1 为待收货
    WaitTakeDelivery,
    /// 2 为已完成
    Complete,
    /// 3 为已评价
    Evaluated,
    /// 4 为申请退货
    Apply,
    /// 5 为已退货
    Refund,
    /// 6 为退款中
    Refunding,
    /// 7 为拒绝退款
    Refuse = 7,
}
impl From<u8> for OrderItemStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::WaitDeliverGoods,
            1 => Self::WaitTakeDelivery,
            2 => Self::Complete,
            3 => Self::Evaluated,
            4 => Self::Apply,
            5 => Self::Refund,
            _ => Self::WaitDeliverGoods,
        }
    }
}

/// 核销单子的状态，0 为取消订单，1 为待核销，2 为已核销，3 已过期
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, Eq, PartialEq)]
pub enum WriteOffStatus {
    /// 0 为取消订单
    Cancel,
    /// 1 为待核销
    PendingWriteOff,
    /// 2 为已核销
    SuccessWriteOff,
    /// 3 已作废
    Invalidated,
}

/// 提现状态，2审核通过，1审核中，0未通过，3提现成功，4提现失败，5正在提现
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema, Eq, PartialEq)]
pub enum WithdrawalReqStatus {
    /// 未通过
    Refuse,
    /// 审核中
    UnderReview,
    /// 审核通过
    Approved,
    /// 提现成功
    Success,
    /// 提现失败
    Fail,
    /// 正在提现
    Ing,
}
