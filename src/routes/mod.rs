use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

use crate::common::types::{DeliveryType, PayType, ProductLayout, QuestionFormType, TranType};
// use crate::routes::BaseData;
use crate::routes::utils_set::mall_set::{PrePareRes, UserBuy};

mod utils_set;

// 测试用的接口
mod test;
pub use test::*;

// 测试用的接口
// mod ws;
// pub use ws::*;

// 测试用的接口
mod user;
pub use user::*;

/// 文件上传， 接口
mod upload;
pub use upload::*;

/// 文章
mod article;
pub use article::*;

/// 问卷表单
mod que_form;
pub use que_form::*;

/// 通用接口
mod common;
pub use common::*;

/// 登录相关接口
mod login;
pub use login::*;

/// 支付接口
mod pay;
pub use pay::*;

/// 管理后台接口
mod manage;
pub use manage::*;

/// 商城相关接口
mod mall;
pub use mall::*;

/// 商城相关接口
mod sales;
pub use sales::*;

use crate::control::wx_info::WxJsSdkSign;

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct Res<T> {
    /// 请求状态，0为失败，1为成功
    status: i8,
    /// 请求结果描述
    message: String,
    /// 返回的数据
    objects: Option<T>,
}
impl<T> Res<T> {
    pub fn success(data: T) -> Self {
        Res {
            status: 1,
            message: String::from("操作成功"),
            objects: Some(data),
        }
    }
    pub fn fail(message: &str) -> Self {
        Res {
            status: 0,
            message: String::from(message),
            objects: None,
        }
    }
    pub fn info(status: i8, message: &str) -> Self {
        Res {
            status,
            message: String::from(message),
            objects: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct PageData<T> {
    list: T,
    total: u64,
}
impl<T> PageData<T> {
    pub fn new(total: u64, list: T) -> Self {
        PageData { list, total }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct BaseNumInfo {
    label: String,
    value: u32,
}
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct BaseStrInfo {
    label: String,
    value: String,
}
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct BaseInfo {
    pub label: String,
    pub value: u32,
    #[schema(no_recursion)]
    pub children: Vec<BaseInfo>,
}

#[derive(OpenApi)]
#[openapi(
    servers(
        (url = "https://xx.com/api", description = "服务器地址"),
        (url = "http://localhost:3060", description = "本机地址"),
    ),
    paths(
        login_silent_wechat_mini, login_wechat_mini_info, upload_file,
        user_feedback, user_collect_add, common_banner_list, common_province_list,
        common_base_info, mall_order_add_shop_cart, mall_order_make_prepare,
        mall_order_make_pay, mall_coupon_receive, mall_coupon_list, mall_product_list,
        mall_product_unit_list, mall_product_user_publish, user_credential_add, common_sms_code,
        login_sms_bind_phone, user_addr_add, user_addr_del, user_addr_list, mall_product_detail,
        login_silent_wechat_gzh, login_wechat_gzh_info, mall_order_list, mall_order_detail,
        mall_order_add_buy_now, mall_store_list, mall_store_detail, user_collect_list, user_addr_detail,
        test_jwt_token, user_coupon_list, user_credential_detail, common_wx_js_sdk_sign,
        pay_make_wx_test,mall_order_modify_status, common_module_switch_list,
        que_form_detail, que_form_submit, mall_brand_options, login_wechat_phone_mini,
        mall_brand_products, mall_brand_products_all, mall_cat_products_all, mall_product_file,
        mall_product_group_all, mall_product_file_send_email, mall_cat_list, mall_cat_tertiary_of,
        article_category_list, article_content_list, article_content_detail, article_stat_praise,
        mall_write_off_info, mall_write_off_do, user_pocket_tran, user_pocket_withdraw_req,
        sales_invite_sale_code, sales_invite_sale_bind, sales_invite_sale_del, sales_invite_user_code,
        sales_invite_user_bind, sales_invite_user_del, sales_list_sale, sales_list_user, user_pocket_money
    ),
    components(schemas(
        UserInfo, WechatLoginMiniInfo, Res<u8>, UploadFile, QuestionFormType,
        UploadRes, BannerRes, Feedback, AreaItem, CityItem, UserAddCredential, ProductLayout,
        ProvItem, AddShopCart, MakePrePare, MakePay, PrePareRes, UserBuy, CouponReceive, WechatPhone,
        ProductRes, UnitRes, CouponRes, AddCollect, UserAddressId, BaseNumInfo, BaseStrInfo,
        BaseInfo, BaseData, ProductAddrInfo, ProductAddCat, UserPubProduct,
        SmsCodePhone, BindPhone, WechatSilent, UserAddress, BaseNumInfo,
        UserOrder, UserOrderItem, UserOrderDetail, UserOrderItemDetail, BuyNow, UserPocket,
        StoreRes, StoreDetailRes, CollectRes, UserCouponRes, CredentialRes, TranType,
        WxJsSdkSign, TestPay, TestJwtToken, MakePayRes, ModifyOder, WxPayInfo, DeliveryType, PayType,
        QueFormItem, QueForm, QueFormItemSubmit, QueFormSubmit, BrandProductItem, BrandProduct,
        CatProductItem, CatProduct, Brand, ProductAttr, ProductDetail, ProductFile, ProductCatItem,
        ProductGroupItem, ProductGroupAll, ProductGroup, ProductGroupSearch, EmailProductFile,
        ArticleCat, Article, ArticleDetail, ArticleId, WriteOffInfo, DoWriteOff, Invite, SaleDelUid,
        SaleUserItem, UserTran, WithdrawRequest
    ))
)]
/// 小程序端接口文档
pub(crate) struct ApiDocMini;

#[derive(OpenApi)]
#[openapi(
    servers(
        (url = "https://xx.com/api", description = "服务器地址"),
        (url = "http://localhost:3060", description = "本机地址"),
    ),
    paths(
        test_jwt_token, manage_mall_store_add, manage_mall_store_list,
        manage_mall_product_add, manage_mall_product_list, manage_mall_product_search,
        manage_mall_brand_add, manage_mall_brand_list, manage_mall_brand_search, manage_mall_brand_del,
        manage_mall_brand_status, manage_mall_product_file_add, manage_mall_product_file_list,
        manage_mall_product_file_del, manage_mall_product_file_status, manage_que_form_que_list,
        manage_que_form_ans_list,
    ),
    components(schemas(
        Res<u8>, UploadFile,
        UploadRes, TestJwtToken,
        StoreAdd, StoreAddrInfo, StoreInfo, BaseNumInfo,
        ProductAdd, ProductAddAttr, ProductAddCat, ProductAddrInfo,
        ProductInfoRes, ProductAddAttrRes, ProductAddCatRes,
        BrandAdd, BrandInfo, BrandSearchInfo, BrandDel, BrandStatus, ProductFileAdd,
        ProductFileInfo, ProductFileDel, ProductFileStatus, QueFormItemRes, QueFormRes,
        AnsItemRes, AnsFormRes
    ))
)]
/// 管理端接口文档
pub(crate) struct ApiDocManage;
