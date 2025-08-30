use actix_web::{Responder, Result, error, get, post, put, web};
use mysql_quick::{TxOpts, myfind, myget};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};
use wx_pay::{Amount, Jsapi, Payer, WxPayData};

use crate::common::UNIT_START_SN;
use crate::common::types::{
    DeliveryType, OrderItemStatus, OrderPayStatus, PayType, ShopCartStatus, TranType,
    WriteOffStatus,
};
use crate::control::app_data::AppData;
use crate::control::wx_info::wx_pay_init;
use crate::db::{my_run_tran_vec, my_run_vec, mysql_conn};
use crate::middleware::AuthUser;
use crate::routes::Res;
use crate::routes::utils_set::mall_set::*;
use crate::routes::utils_set::pocket_set::pocket_money_sub;
use crate::routes::utils_set::sales_set::do_order_sale_split;
use crate::routes::utils_set::write_off_item::add_write_off;
use crate::utils::files::get_file_url;
use crate::utils::utils::{keep_uint, log_err};

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct AddShopCart {
    /// 商品编号
    unit_sn: u32,
    /// 购买数量
    buy_quantity: u32,
}
/// 【订单】添加到购物车
#[utoipa::path(
    request_body = AddShopCart,
    responses((status = 200, description = "【请求：AddShopCart】【返回：String】", body = String)),
)]
#[post("/mall/order/add/shop_cart")]
pub async fn mall_order_add_shop_cart(
    user: AuthUser,
    params: web::Json<AddShopCart>,
) -> Result<impl Responder> {
    let uid = user.id;

    if params.unit_sn < UNIT_START_SN {
        return Ok(web::Json(Res::fail("商品编号错误")));
    }
    if params.buy_quantity <= 0 {
        return Ok(web::Json(Res::fail("购买数量不能小于1")));
    }

    let mut conn = mysql_conn()?;
    // ---- 事务开始 ----
    let mut tran = conn.start_transaction(TxOpts::default()).unwrap();
    let add_info = match add_unit_to_shop_cart(
        &mut tran,
        uid,
        params.unit_sn,
        params.buy_quantity,
        ShopCartStatus::PendingPayment,
    ) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    if add_info.status == 1 {
        tran.commit().unwrap();
    } else {
        tran.rollback().unwrap();
        return Ok(web::Json(Res::fail(&add_info.message)));
    }
    // ---- 事务结束 ----
    Ok(web::Json(Res::success("添加购物车成功")))
}

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct BuyNow {
    /// 商品编号
    unit_sn: u32,
    /// 购买数量
    buy_quantity: u32,
}
/// 【订单】添加立即购买
#[utoipa::path(
    request_body = BuyNow,
    responses((status = 200, description = "【请求：BuyNow】【返回：String】", body = String)),
)]
#[post("/mall/order/add/buy_now")]
pub async fn mall_order_add_buy_now(
    user: AuthUser,
    params: web::Json<BuyNow>,
) -> Result<impl Responder> {
    let uid = user.id;

    if params.unit_sn < UNIT_START_SN {
        return Ok(web::Json(Res::fail("商品编号错误")));
    }
    if params.buy_quantity <= 0 {
        return Ok(web::Json(Res::fail("购买数量不能小于1")));
    }

    let mut conn = mysql_conn()?;
    // ---- 事务开始 ----
    let mut tran = conn.start_transaction(TxOpts::default()).unwrap();
    let add_info = match add_unit_to_shop_cart(
        &mut tran,
        uid,
        params.unit_sn,
        params.buy_quantity,
        ShopCartStatus::BuyNow,
    ) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    if add_info.status == 1 {
        tran.commit().unwrap();
    } else {
        tran.rollback().unwrap();
        return Ok(web::Json(Res::fail(&add_info.message)));
    }
    // ---- 事务结束 ----
    Ok(web::Json(Res::success("添加立即购买成功")))
}

#[derive(Serialize, Debug, Deserialize, IntoParams, ToSchema, Clone)]
pub struct MakePrePare {
    /// 商品编号列表
    unit_sns: String,
    /// 购买类型：pending 为购物车的待结算，buy_now 为立即购买方式
    buy_type: String,
    /// 优惠券id
    coupon_id: Option<u32>,
}
/// 【订单】生成预览订单
#[utoipa::path(
    responses((status = 200, description = "【请求：MakePrePare】【返回：PrePareRes】", body = PrePareRes)),
    params(MakePrePare),
)]
#[get("/mall/order/make/prepare")]
pub async fn mall_order_make_prepare(
    user: AuthUser,
    params: web::Query<MakePrePare>,
) -> Result<impl Responder> {
    let uid = user.id;
    if params.unit_sns.is_empty() {
        return Ok(web::Json(Res::fail("商品编号不能为空")));
    }
    let unit_sn_list = params
        .unit_sns
        .split(",")
        .map(|x| x.parse::<u32>().unwrap())
        .collect::<Vec<u32>>();
    if unit_sn_list.iter().find(|x| x < &&UNIT_START_SN).is_some() {
        return Ok(web::Json(Res::fail("商品编号错误")));
    }
    let buy_type: ShopCartStatus = params.buy_type.clone().into();
    if buy_type == ShopCartStatus::Wrong
        || buy_type == ShopCartStatus::Paid
        || buy_type == ShopCartStatus::BuyNowPaid
    {
        return Ok(web::Json(Res::fail("购买状态错误")));
    }
    let mut conn = mysql_conn()?;
    // ---- 事务开始 ----
    let mut tran = conn
        .start_transaction(TxOpts::default())
        .map_err(|e| error::ErrorInternalServerError(log_err(&e, &params)))?;
    let prepare = match get_order_prepare(
        &mut tran,
        uid,
        &unit_sn_list,
        &buy_type,
        params.coupon_id,
        false,
    ) {
        Ok(p) => p,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    tran.commit().unwrap();
    // ---- 事务结束 ----
    Ok(web::Json(Res::success(prepare)))
}

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct MakePay {
    /// 商品编号
    pub unit_sns: Vec<u32>,
    /// 购买类型：pending 为购物车的待结算，buy_now 为立即购买方式
    pub buy_type: String,
    /// 优惠券id
    pub coupon_id: Option<u32>,
    /// 用户备注
    pub notes: Option<String>,
    /// 用户地址id
    pub usr_address_id: Option<u64>,
    /// 支付类型，通用接口 /common/base/info 有返回
    pub pay_type: PayType,
    /// 用户选择的物流方式
    pub delivery_type: DeliveryType,
    /// 预约时间，快递的送达时间。如果为自提，则为自提的时间
    pub appointment_time: Option<String>,
}
/// 客户端发起支付返回的参数信息
#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct WxPayInfo {
    app_id: Option<String>,
    sign_type: String,
    pay_sign: String,
    package: String,
    nonce_str: String,
    time_stamp: String,
}
/// 客户端发起支付返回的参数信息
#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct MakePayRes {
    /// 用户零钱支付 POCKET_PAY
    /// 微信支付 WX_PAY
    /// 如果是零钱支付，则会直接返回支付成功；如果是微信支付，则返回微信支付参数
    pay_type: PayType,
    /// 微信支付，支付参数
    wx_pay: Option<WxPayInfo>,
}
/// 【订单】去支付
#[utoipa::path(
    request_body = MakePay,
    responses((status = 200, description = "【请求：MakePay】【返回：MakePayRes】", body = MakePayRes)),
)]
#[post("/mall/order/make/pay")]
pub async fn mall_order_make_pay(
    user: AuthUser,
    params: web::Json<MakePay>,
    app_data: web::Data<AppData>,
) -> Result<impl Responder> {
    let data = &app_data;
    let uid = user.id;
    if params.unit_sns.len() < 1 {
        return Ok(web::Json(Res::fail("商品编号不能为空")));
    }
    if params
        .unit_sns
        .iter()
        .find(|x| x < &&UNIT_START_SN)
        .is_some()
    {
        return Ok(web::Json(Res::fail("商品编号错误")));
    }
    let buy_type: ShopCartStatus = params.buy_type.clone().into();
    if buy_type == ShopCartStatus::Wrong
        || buy_type == ShopCartStatus::Paid
        || buy_type == ShopCartStatus::BuyNowPaid
    {
        return Ok(web::Json(Res::fail("购买状态错误")));
    }
    let mut conn = mysql_conn()?;
    // 因为是微信小程序或公众号，所以，这里的openid必定存在。
    let openid = get_user_openid(&mut conn, uid)?;

    let pay_type: PayType = params.pay_type.clone().into();
    if pay_type == PayType::UnknownPay {
        return Ok(web::Json(Res::fail("支付方式错误")));
    }

    // ---- 事务开始 ----
    let mut tran = conn.start_transaction(TxOpts::default()).unwrap();
    let prepare = match get_order_prepare(
        &mut tran,
        uid,
        &params.unit_sns,
        &buy_type,
        params.coupon_id,
        true,
    ) {
        Ok(p) => p,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    if prepare.user_buy.len() == 0 {
        tran.rollback().unwrap();
        return Ok(web::Json(Res::fail("没有待结算")));
    }
    // 查找用户地址
    let user_addr =
        match get_user_address_or_none(&mut tran, params.usr_address_id, &params.delivery_type) {
            Ok(d) => d,
            Err(e) => {
                tran.rollback().unwrap();
                return Err(e);
            }
        };

    // 生成一个总订单
    let (order_sn, pay_des) = match create_order(
        &mut tran, data, uid, &prepare, &params, &user_addr, &pay_type,
    ) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };

    // 更新 购物车状态  prepare.user_buy
    match upd_shop_cart_status(&mut tran, &prepare.user_buy, &buy_type) {
        Ok(_) => (),
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };

    // 如果用户使用了优惠券，则修改为已使用
    if prepare.is_coupon_used {
        if let Some(usr_c_id) = prepare.usr_coupon_id {
            match upd_coupon_status(&mut tran, usr_c_id) {
                Ok(_) => (),
                Err(e) => {
                    tran.rollback().unwrap();
                    return Err(e);
                }
            }
        } else {
            // 对不上，就回滚
            tran.rollback().unwrap();
            return Ok(web::Json(Res::fail("未找到相关优惠券")));
        }
    }

    let mut wxinfo = WxPayData {
        app_id: None,
        sign_type: String::new(),
        pay_sign: String::new(),
        package: String::new(),
        nonce_str: String::new(),
        time_stamp: String::new(),
    };
    // 如果是零钱支付，则进行零钱支付操作
    match pay_type {
        PayType::PocketPay => {
            let info = json!({
                "order_sn": &order_sn,
                "prepare": serde_json::to_string(&prepare).unwrap(),
                "params": serde_json::to_string(&params).unwrap(),
            });
            // 用户零钱支付
            if prepare.pay_amount < 0. {
                return Err(error::ErrorBadRequest("购买金额错误"));
            }
            // 零钱增减，同时有交易记录添加
            match pocket_money_sub(
                &mut tran,
                uid,
                prepare.pay_amount,
                TranType::Purchase,
                PayType::PocketPay,
                Some(&serde_json::to_string(&info).unwrap()),
            ) {
                Ok(_) => (),
                Err(e) => {
                    tran.rollback().unwrap();
                    return Err(e);
                }
            };
            // 进行销售分成处理
            match do_order_sale_split(&mut tran, &order_sn, uid, pay_type.clone()) {
                Ok(_) => (),
                Err(e) => {
                    tran.rollback().unwrap();
                    return Err(e);
                }
            };
            // 修改订单状态 为 已支付
            match upd_order_status(&mut tran, &order_sn, OrderPayStatus::Paid, None, None) {
                Ok(d) => d,
                Err(e) => {
                    tran.rollback().unwrap();
                    return Err(e);
                }
            };
            // 修改商品和产品的销售量计数
            match upd_product_unit_sell_total(&mut tran, &order_sn) {
                Ok(d) => d,
                Err(e) => {
                    tran.rollback().unwrap();
                    return Err(e);
                }
            };
            // 如果有核销类型的产品，那新增待核销记录
            match add_write_off(&mut tran, &order_sn) {
                Ok(d) => d,
                Err(e) => {
                    tran.rollback().unwrap();
                    return Err(e);
                }
            };
        }
        PayType::WxPay => {
            // 如果是微信支付，则进行微信支付操作
            // 发起微信支付
            let wxpay = wx_pay_init();
            wxinfo = match wxpay
                .jsapi(&Jsapi {
                    description: pay_des,
                    out_trade_no: order_sn.clone(),
                    amount: Amount {
                        total: keep_uint(prepare.pay_amount * 100.),
                        ..Default::default()
                    },
                    payer: Payer { openid },
                    ..Default::default()
                })
                .await
                .map_err(|e| error::ErrorBadGateway(e))
            {
                Ok(data) => data,
                Err(e) => {
                    tran.rollback().unwrap();
                    return Err(error::ErrorBadGateway(e));
                }
            };
        }
        _ => {
            tran.rollback().unwrap();
            return Err(error::ErrorBadRequest("不支持的支付类型"));
        }
    };
    tran.commit().unwrap();

    // ---- 事务结束 ----
    if pay_type == PayType::PocketPay {
        // 用户零钱支付成功
        Ok(web::Json(Res::success(MakePayRes {
            pay_type: PayType::PocketPay,
            wx_pay: None,
        })))
    } else if pay_type == PayType::WxPay {
        Ok(web::Json(Res::success(MakePayRes {
            pay_type: PayType::WxPay,
            wx_pay: Some(WxPayInfo {
                app_id: wxinfo.app_id.clone(),
                sign_type: wxinfo.sign_type.clone(),
                pay_sign: wxinfo.pay_sign.clone(),
                package: wxinfo.package.clone(),
                nonce_str: wxinfo.nonce_str.clone(),
                time_stamp: wxinfo.time_stamp,
            }),
        })))
    } else {
        Err(error::ErrorBadRequest("不支持的支付类型"))
    }
}

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct UserOrderItem {
    id: u64,
    /// 子订单编号
    order_item_id: String,
    /// 商品编号
    unit_sn: u32,
    /// 商品名
    unit_name: String,
    /// 商品封面图
    unit_cover: Option<String>,
    /// 产品编号
    product_sn: u32,
    /// 产品名
    product_name: String,
    /// 子订单状态 0 待发货，1 待收货, 2 已完成, 3 已评价，4 申请退货，5 已退货
    status: u8,
}
#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct UserOrder {
    id: u64,
    /// 订单编号
    order_sn: String,
    /// 购买总数量
    total_quantity: u32,
    /// 实际付款金额
    pay_amount: f64,
    /// 订单状态 1待支付，2已支付，0取消支付
    order_status: u8,
    /// 创建时间
    created_at: String,
    /// 子订单项目
    items: Vec<UserOrderItem>,
}
#[derive(Serialize, Deserialize, Debug, IntoParams, ToSchema)]
pub struct OrderDeliveryType {
    /// 物流类型: /common/base/info 返回的 delivery_type。取value值
    /// 多个值用逗号分隔
    delivery_type: String,
}
/// 【订单】用户订单列表
#[utoipa::path(
    responses((status = 200, description = "【返回：UserOrder[]】", body = Vec<UserOrder>)),
    params(
        ("status", description="【商品状态】：-1 全部，0 待发货，1 待收货, 2 已完成，3 已评价，4 申请退货，5 已退货。【核销状态】：-1 全部，0 待核销, 2 已完成，3 已评价，4 申请退货，5 已退货"), ("page", description="第几页"),
        OrderDeliveryType
    )
)]
#[get("/mall/order/list/{status}/{page}")]
pub async fn mall_order_list(
    user: AuthUser,
    path: web::Path<(String, String)>,
    query: web::Query<OrderDeliveryType>,
) -> Result<impl Responder> {
    let dtypes = query.delivery_type.clone();
    let uid = user.id;
    // order 1待支付，2已支付，0取消支付
    // order_item 0 待发货，1 待收货, 2 已完成, 3 已评价，4 申请退货，5 已退货
    let status: i8 = path
        .0
        .to_owned()
        .parse()
        .map_err(|e| error::ErrorBadRequest(e))?;
    let page: u32 = path
        .1
        .to_owned()
        .parse()
        .map_err(|e| error::ErrorBadRequest(e))?;

    let mut conn = mysql_conn()?;

    // 查寻用户子订单
    #[derive(Deserialize, Debug)]
    struct OrderItemGet {
        id: u64,
        order_item_id: String,
        order_sn: String,
        unit_sn: u32,
        unit_name: String,
        unit_cover: Option<String>,
        product_sn: u32,
        product_name: String,
        status: u8,
        order_id: u64,
        order_status: u8,
        total_quantity: u32,
        pay_amount: String,
        created_at: String,
    }
    let sub_order: Vec<OrderItemGet> = my_run_vec(
        &mut conn,
        myfind!("ord_order_item", {
            j0: ["unit_sn", "inner", "sku_unit.unit_sn"],
            j1: ["order_sn", "inner", "ord_order.order_sn"],
            p0: ["ord_order.status", "=", 2], // 已支付订单
            p1: ["is_del", "=", 0],
            p2: ["status", "=", status],
            p3: ["uid", "=", uid],
            p4: ["ord_order.delivery_type", "in", &dtypes], // 订单类别
            r: if status == -1 {
                "p0 && p4 && p1 && p3"
            } else {
                "p0 && p4 && p1 && p2 && p3"
            },
            page: page,
            limit: 20,
            select: "id, order_item_id, order_sn, unit_sn, unit_name, unit_cover,
            sku_unit.product_sn, product_name, status, ord_order.total_quantity,
            ord_order.pay_amount, ord_order.created_at, ord_order.id as order_id,
            ord_order.status as order_status",
        }),
    )?;

    let mut order_list: Vec<UserOrder> = vec![];
    for item in sub_order {
        let index_op = order_list.iter().position(|x| x.id == item.order_id);
        if let Some(index) = index_op {
            let mut order = order_list[index].clone();
            order.items.push(UserOrderItem {
                id: item.id,
                order_item_id: item.order_item_id.clone(),
                unit_sn: item.unit_sn,
                unit_name: item.unit_name.clone(),
                unit_cover: get_file_url(item.unit_cover.clone()),
                product_sn: item.product_sn,
                product_name: item.product_name.clone(),
                status: item.status,
            });
            order_list[index] = order;
        } else {
            // 没有
            order_list.push(UserOrder {
                id: item.order_id,
                order_sn: item.order_sn.clone(),
                total_quantity: item.total_quantity,
                pay_amount: item.pay_amount.parse::<f64>().unwrap(),
                order_status: item.order_status,
                created_at: item.created_at.clone(),
                items: vec![UserOrderItem {
                    id: item.id,
                    order_item_id: item.order_item_id.clone(),
                    unit_sn: item.unit_sn,
                    unit_name: item.unit_name.clone(),
                    unit_cover: get_file_url(item.unit_cover.clone()),
                    product_sn: item.product_sn,
                    product_name: item.product_name.clone(),
                    status: item.status,
                }],
            });
        }
    }

    Ok(web::Json(Res::success(order_list)))
}

// /// 【订单】用户总订单
// #[utoipa::path(
//     responses((status = 200, description = "【返回：UserOrder[]】", body = Vec<UserOrder>)),
//     params(("status", description="2 已支付，1待支付，0取消支付"), ("page", description="第几页"))
// )]
// #[get("/mall/order/list/pay/{status}/{page}")]
// pub async fn mall_order_pay_list(
//     user: AuthUser,
//     query: web::Path<(String, String)>,
// ) -> Result<impl Responder> {
//     let uid = user.id;
//     // order 1待支付，2已支付，0取消支付
//     // order_item 0 待发货，1 待收货, 2 已完成, 3 已评价，4 申请退货，5 已退货
//     let status: u8 = query
//         .0
//         .to_owned()
//         .parse()
//         .map_err(|e| error::ErrorBadRequest(e))?;
//     let page: u32 = query
//         .1
//         .to_owned()
//         .parse()
//         .map_err(|e| error::ErrorBadRequest(e))?;

//     let r;
//     let r2;

//     let mut order_s = 0;
//     let mut order_item_s = 0;
//     if status == 20 {
//         r = "p0 && p1";
//         r2 = "p0 && p1";
//     } else if status >= 10 && status < 20 {
//         order_s = status - 10;
//         r = "p0 && p1 && p2";
//         r2 = "p0 && p1";
//     } else if status < 10 {
//         order_s = 2; // 必须要为已支付的
//         order_item_s = status;
//         r = "p0 && p1 && p2";
//         r2 = "p0 && p1 && p2";
//     } else {
//         return Err(error::ErrorBadRequest("请求参数 status 错误"));
//     };

//     let mut conn = mysql_conn()?;

//     // 用户订单
//     #[derive(Deserialize, Debug)]
//     struct OrderGet {
//         id: u64,
//         order_sn: String,
//         total_quantity: u32,
//         pay_amount: String,
//         status: u8,
//         created_at: String,
//     }
//     let order: Vec<OrderGet> = my_run_vec(
//         &mut conn,
//         myfind!("ord_order", {
//             p0: ["uid", "=", uid],
//             p1: ["is_del", "=", 0],
//             p2: ["status", "=", order_s],
//             r: r,
//             page: page,
//             limit: 12,
//             select: "id,order_sn,total_quantity,pay_amount,status,created_at",
//         }),
//     )?;

//     let order_sn_list: Vec<String> = order.iter().map(|x| x.order_sn.clone()).collect();

//     // 查寻用户子订单
//     #[derive(Deserialize, Debug)]
//     struct OrderItemGet {
//         id: u64,
//         order_item_id: String,
//         order_sn: String,
//         unit_sn: u32,
//         unit_name: String,
//         unit_cover: Option<String>,
//         product_sn: u32,
//         product_name: String,
//         status: u8,
//     }
//     let sub_order: Vec<OrderItemGet> = my_run_vec(
//         &mut conn,
//         myfind!("ord_order_item", {
//             j0: ["unit_sn", "inner", "sku_unit.unit_sn"],
//             p0: ["order_sn", "in", order_sn_list.join(",")],
//             p1: ["is_del", "=", 0],
//             p2: ["status", "=", order_item_s],
//             r: r2,
//             select: "id, order_item_id, order_sn, unit_sn, unit_name, unit_cover, sku_unit.product_sn, product_name, status",
//         }),
//     )?;

//     // 将子订单合并到 订单
//     let order_all: Vec<UserOrder> = order
//         .iter()
//         .map(|x| {
//             let sub_order_all: Vec<UserOrderItem> = sub_order
//                 .iter()
//                 .map(|y| {
//                     if x.order_sn == y.order_sn {
//                         UserOrderItem {
//                             id: y.id,
//                             order_item_id: y.order_item_id.clone(),
//                             unit_sn: y.unit_sn,
//                             unit_name: y.unit_name.clone(),
//                             unit_cover: get_file_url(y.unit_cover.clone()),
//                             product_sn: y.product_sn,
//                             product_name: y.product_name.clone(),
//                             status: y.status,
//                         }
//                     } else {
//                         UserOrderItem {
//                             id: 0,
//                             order_item_id: "".to_string(),
//                             unit_sn: 0,
//                             unit_name: "".to_string(),
//                             unit_cover: None,
//                             product_sn: 0,
//                             product_name: "".to_string(),
//                             status: 0,
//                         }
//                     }
//                 })
//                 .filter(|d| d.id > 0)
//                 .collect();
//             UserOrder {
//                 id: x.id,
//                 order_sn: x.order_sn.clone(),
//                 total_quantity: x.total_quantity,
//                 pay_amount: x.pay_amount.parse::<f64>().unwrap(),
//                 order_status: x.status,
//                 created_at: x.created_at.clone(),
//                 items: sub_order_all,
//             }
//         })
//         .collect();

//     Ok(web::Json(Res::success(order_all)))
// }

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct UserOrderItemDetail {
    id: u64,
    /// 子订单编号
    order_item_id: String,
    /// 商品编号
    unit_sn: u32,
    /// 商品名
    unit_name: String,
    /// 商品封面图
    unit_cover: Option<String>,
    /// 商品价格
    price: f64,
    /// 购买的数量
    buy_quantity: u32,
    /// 产品编号
    product_sn: u32,
    /// 产品名
    product_name: String,
    /// 产品封面图
    product_cover: Option<String>,
    /// 子订单状态 0 待发货，1 待收货, 2 已完成, 3 已评价，4 申请退货，5 已退货
    status: u8,
}
#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct UserOrderDetail {
    id: u64,
    /// 订单编号
    order_sn: String,
    /// 购买总数量
    total_quantity: u32,
    /// 合计金额
    total_amount: f64,
    /// 优惠金额
    reduce_amount: Option<f64>,
    /// 优惠信息
    reduce_des: Option<String>,
    /// 实际付款金额
    pay_amount: f64,
    /// 用户备注
    notes: Option<String>,
    /// 预约时间
    appointment_time: Option<String>,
    /// 收货地址 省
    province: Option<String>,
    /// 收货地址 市
    city: Option<String>,
    /// 收货地址 区
    area: Option<String>,
    /// 收货地址 详细
    addr_detail: Option<String>,
    /// 收货 联系人
    contact_user: Option<String>,
    /// 收货 联系手机
    contact_phone: Option<String>,
    /// 订单状态 1待支付，2已支付，0取消支付
    order_status: u8,
    /// 创建时间
    created_at: String,
    /// 子订单项目
    items: Vec<UserOrderItemDetail>,
}
/// 【订单】用户订单详情
#[utoipa::path(
    responses((status = 200, description = "【返回：UserOrderDetail】", body = UserOrderDetail)),
    params(("order_sn", description="订单编号"))
)]
#[get("/mall/order/detail/{order_sn}")]
pub async fn mall_order_detail(
    _user: AuthUser,
    query: web::Path<String>,
) -> Result<impl Responder> {
    let order_sn = query.to_owned();
    let mut conn = mysql_conn()?;

    // 用户订单
    #[derive(Deserialize, Debug)]
    struct OrderGet {
        id: u64,
        order_sn: String,
        total_quantity: u32,
        total_amount: String,
        reduce_amount: Option<String>,
        reduce_des: Option<String>,
        pay_amount: String,
        notes: Option<String>,
        appointment_time: Option<String>,
        province: Option<String>,
        city: Option<String>,
        area: Option<String>,
        addr_detail: Option<String>,
        contact_user: Option<String>,
        contact_phone: Option<String>,
        status: u8,
        created_at: String,
    }
    let order: Vec<OrderGet> = my_run_vec(
        &mut conn,
        myfind!("ord_order", {
            p0: ["order_sn", "=", &order_sn],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "id,order_sn,total_quantity,total_amount,pay_amount,
                reduce_amount,reduce_des,status,notes,appointment_time,
                province,city,area,addr_detail,contact_user,contact_phone,created_at",
        }),
    )?;

    if order.len() == 0 {
        return Err(error::ErrorBadRequest(format!("订单 {} 不存在", order_sn)));
    }

    // 查寻用户子订单
    #[derive(Deserialize, Debug)]
    struct OrderItemGet {
        id: u64,
        order_item_id: String,
        unit_sn: u32,
        unit_name: String,
        unit_cover: Option<String>,
        price: String,
        buy_quantity: u32,
        product_sn: u32,
        product_name: String,
        product_cover: Option<String>,
        status: u8,
    }
    let sub_order: Vec<OrderItemGet> = my_run_vec(
        &mut conn,
        myfind!("ord_order_item", {
            j0: ["unit_sn", "inner", "sku_unit.unit_sn"],
            j1: ["sku_unit.product_sn", "inner", "spu_product.product_sn"],
            p0: ["order_sn", "=", &order_sn],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "id, order_item_id, unit_sn, unit_name,
                    unit_cover, sku_unit.product_sn, product_name, status,
                    price, buy_quantity, spu_product.product_cover_img as product_cover",
        }),
    )?;

    let sub_order_all: Vec<UserOrderItemDetail> = sub_order
        .iter()
        .map(|y| UserOrderItemDetail {
            id: y.id,
            order_item_id: y.order_item_id.clone(),
            unit_sn: y.unit_sn,
            unit_name: y.unit_name.clone(),
            unit_cover: get_file_url(y.unit_cover.clone()),
            price: y.price.parse::<f64>().unwrap(),
            buy_quantity: y.buy_quantity,
            product_sn: y.product_sn,
            product_name: y.product_name.clone(),
            product_cover: get_file_url(y.product_cover.clone()),
            status: y.status,
        })
        .collect();
    // 将子订单合并到 订单
    let order_info: Vec<UserOrderDetail> = order
        .iter()
        .map(|x| UserOrderDetail {
            id: x.id,
            order_sn: x.order_sn.clone(),
            total_quantity: x.total_quantity,
            total_amount: x.total_amount.parse::<f64>().unwrap(),
            reduce_amount: if let Some(r) = x.reduce_amount.clone() {
                match r.parse::<f64>() {
                    Ok(d) => Some(d),
                    Err(_) => None,
                }
            } else {
                None
            },
            reduce_des: x.reduce_des.clone(),
            pay_amount: x.pay_amount.parse::<f64>().unwrap(),
            notes: x.notes.clone(),
            appointment_time: x.appointment_time.clone(),
            province: x.province.clone(),
            city: x.city.clone(),
            area: x.area.clone(),
            addr_detail: x.addr_detail.clone(),
            contact_user: x.contact_user.clone(),
            contact_phone: x.contact_phone.clone(),
            order_status: x.status,
            created_at: x.created_at.clone(),
            items: sub_order_all.clone(),
        })
        .collect();

    Ok(web::Json(Res::success(order_info)))
}

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct ModifyOder {
    /// 订单号
    order_sn: String,
    ///  4 申请退款
    status: OrderPayStatus,
    /// 退款原因
    reason: Option<String>,
}
/// 【订单】用户申请退款
#[utoipa::path(
    request_body = ModifyOder,
    responses((status = 200, description = "【请求：ModifyOder】【返回：String】", body = String)),
)]
#[put("/mall/order/modify/status")]
pub async fn mall_order_modify_status(
    user: AuthUser,
    params: web::Json<ModifyOder>,
) -> Result<impl Responder> {
    let uid = user.id;

    // 只支持申请退款操作
    if params.status != OrderPayStatus::Apply {
        return Ok(web::Json(Res::fail("只支持申请退款操作")));
    }

    let mut conn = mysql_conn()?;

    // ---- 事务开始 ----
    let mut tran = conn.start_transaction(TxOpts::default()).unwrap();

    // 验证订单是否存在且属于该用户，同时检查当前状态
    #[derive(Deserialize)]
    struct OrderInfo {
        uid: u64,
        status: i8,
    }

    let order_info: Vec<OrderInfo> = match my_run_tran_vec(
        &mut tran,
        myget!("ord_order", { "order_sn": &params.order_sn }, "uid,status"),
    ) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };

    if order_info.is_empty() {
        tran.rollback().unwrap();
        return Ok(web::Json(Res::fail("订单不存在")));
    }

    if order_info[0].uid != uid {
        tran.rollback().unwrap();
        return Ok(web::Json(Res::fail("订单不属于当前用户")));
    }

    // 检查订单当前状态，只有已支付的订单才能申请退款
    if order_info[0].status != OrderPayStatus::Paid as i8 {
        tran.rollback().unwrap();
        return Ok(web::Json(Res::fail("只有已支付的订单才能申请退款")));
    }

    // 1. 修改主订单状态为申请退款
    match upd_order_status(
        &mut tran,
        &params.order_sn,
        OrderPayStatus::Apply,
        None,
        params.reason.clone(),
    ) {
        Ok(_) => {}
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    }

    // 2. 查询该订单下的所有子订单项
    #[derive(Deserialize)]
    struct OrderItemInfo {
        order_item_id: String,
    }

    let order_items: Vec<OrderItemInfo> = match my_run_tran_vec(
        &mut tran,
        myfind!("ord_order_item", {
            p0: ["order_sn", "=", &params.order_sn],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "order_item_id",
        }),
    ) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };

    // 3. 检查核销状态（如果存在核销记录）
    for item in &order_items {
        // 查询是否存在核销记录
        #[derive(Deserialize)]
        struct WriteOffInfo {
            write_off_status: i8,
        }

        let write_off_records: Vec<WriteOffInfo> = match my_run_tran_vec(
            &mut tran,
            myfind!("ord_write_off_item", {
                p0: ["order_item_id", "=", &item.order_item_id],
                p1: ["is_del", "=", 0],
                r: "p0 && p1",
                select: "write_off_status",
            }),
        ) {
            Ok(d) => d,
            Err(e) => {
                tran.rollback().unwrap();
                return Err(e);
            }
        };

        // 如果存在核销记录，检查核销状态
        if !write_off_records.is_empty() {
            let write_off_status = write_off_records[0].write_off_status;
            // 只有待核销状态才允许申请退款
            if write_off_status != WriteOffStatus::PendingWriteOff as i8 {
                tran.rollback().unwrap();
                return Ok(web::Json(Res::fail(
                    "该订单包含已核销或其他状态的商品，不允许申请退款",
                )));
            }
        }
    }

    // 4. 修改所有子订单项状态为申请退货
    for item in &order_items {
        match upd_order_item_status(&mut tran, &item.order_item_id, OrderItemStatus::Apply) {
            Ok(_) => {}
            Err(e) => {
                tran.rollback().unwrap();
                return Err(e);
            }
        }
    }

    // 提交事务
    tran.commit().unwrap();

    Ok(web::Json(Res::success("申请退款成功，等待管理员处理")))
}

#[cfg(test)]
mod test {
    use mysql_quick::mysetmany;
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_json() {
        let a: serde_json::Value = serde_json::from_str(
            r#"{
          "unit_sns": [
            1000003,2
          ]
        }"#,
        )
        .unwrap();
        println!("{:?}", a);

        #[derive(Serialize, Debug, Deserialize)]
        struct OrderItems {
            name: Option<String>,
        }
        let order_items = vec![
            OrderItems { name: None },
            OrderItems {
                name: Some("3234234234234".to_string()),
            },
        ];

        let sql_items = mysetmany!("ord_order_item", order_items);

        println!("sql_items  {}", sql_items);
        assert!(true)
    }
}
