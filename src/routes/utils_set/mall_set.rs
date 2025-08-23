use actix_web::{Error, error, web::Data};
use mysql_quick::{
    MY_EXCLUSIVE_LOCK, PooledConn, Transaction, myfind, myget, myset, mysetmany, myupdate,
    myupdatemany,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::MakePay;
use crate::common::types::{
    DeliveryType, NormalStatus, OrderItemStatus, OrderPayStatus, OssBucket, PayType,
    ShopCartStatus, UserCouponStatus, WriteOffStatus,
};
use crate::control::app_data::{AppData, SlownWorker};
use crate::db::{my_run_tran_drop, my_run_tran_vec, my_run_vec};
use crate::utils::utils::log_err;
use crate::{
    routes::{Res, UnitAttrInfo},
    utils::{
        files::{get_file_url, get_path_from_url},
        time::{NowTimeType, get_now_time},
        utils::keep_decimal,
    },
};

/// 添加商品到购物车
pub fn add_unit_to_shop_cart(
    tran: &mut Transaction,
    uid: u64,
    unit_sn: u32,
    buy_quantity: u32,
    shop_cart_status: ShopCartStatus,
) -> Result<Res<String>, Error> {
    #[derive(Deserialize)]
    struct ShopCartGet {
        id: u64,
    }
    #[derive(Serialize, Deserialize)]
    struct UnitAttrGet {
        primary_name: String,
        secondary_name: String,
    }
    #[derive(Deserialize)]
    struct UnitInfoGet {
        unit_name: String,
        unit_cover: Option<String>,
        quantity: i64,
        product_name: String,
        status: u8,
        product_status: u8,
    }
    // 查找，购物车里有没有已经有的同一个编号的商品，有就直接更新数量
    // 立即购买，则不用查找
    let have_unit: Vec<ShopCartGet> = if shop_cart_status == ShopCartStatus::PendingPayment {
        my_run_tran_vec(
            tran,
            myfind!("ord_shop_cart", {
                p0: ["uid", "=", uid],
                p1: ["unit_sn", "=", unit_sn],
                p2: ["is_del", "=", 0],
                p3: ["status", "=", ShopCartStatus::PendingPayment as u8],
                r: "p0 && p1 && p2 && p3",
                select: "id",
            }) + MY_EXCLUSIVE_LOCK,
        )?
    } else {
        vec![]
    };

    // 查寻当前商品的属性
    let unit_attrs: Vec<UnitAttrGet> = my_run_tran_vec(
        tran,
        myfind!("sku_unit_attr", {
            p0: ["unit_sn", "=", unit_sn],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "primary_name,secondary_name",
        }),
    )?;

    // 获取当前商品 和 产品 信息，判断有没有下架，有没有库存不够
    let unit_info: Vec<UnitInfoGet> = my_run_tran_vec(
        tran,
        myfind!("sku_unit", {
            j0: ["product_sn", "inner", "spu_product.product_sn"],
            p0: ["unit_sn", "=", unit_sn],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "unit_cover,unit_name,spu_product.product_name,quantity,status,spu_product.status as product_status",
        }) + MY_EXCLUSIVE_LOCK,
    )?;
    if unit_info.len() == 0 {
        return Ok(Res::fail("商品不存在"));
    }
    if unit_info[0].product_status != 2 {
        return Ok(Res::fail("产品已下架"));
    }
    if unit_info[0].status != 2 {
        return Ok(Res::fail("商品已下架"));
    }
    if unit_info[0].quantity < 0 {
        return Ok(Res::fail("库存不足"));
    }
    if unit_info[0].quantity < buy_quantity as i64 {
        return Ok(Res::fail("库存不足"));
    }

    // 将库存减去
    let sub_quantity: i64 = -(buy_quantity as i64);
    my_run_tran_drop(
        tran,
        myupdate!("sku_unit", {"unit_sn": unit_sn}, {
            "quantity": ["incr", sub_quantity],
        }),
    )?;

    // 添加，或更新购物车数量
    let sql;
    if have_unit.len() > 0 {
        // 有同一个商品，直接更新数量
        sql =
            myupdate!("ord_shop_cart", have_unit[0].id, { "buy_quantity": ["incr", buy_quantity] });
    } else {
        let attr_json_str = match serde_json::to_string(&unit_attrs) {
            Ok(d) => d,
            Err(e) => {
                return Err(error::ErrorInternalServerError(log_err(&e, "")));
            }
        };
        // 没有，则新增
        sql = myset!("ord_shop_cart", {
            "uid": uid,
            "unit_sn": unit_sn,
            "unit_name": &unit_info[0].unit_name,
            "buy_quantity": buy_quantity,
            "unit_cover": if let Some(c) = &unit_info[0].unit_cover { c } else { "null" },
            "product_name": &unit_info[0].product_name,
            "unit_attr_info": &attr_json_str,
            "status": shop_cart_status as u8,
        });
    }
    my_run_tran_drop(tran, sql)?;

    Ok(Res::success("添加购物车成功".to_string()))
}

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct UserBuy {
    /// 购物车的id
    pub id: u64,
    /// 商品编号
    pub unit_sn: u32,
    /// 商品封面图
    pub unit_cover: String,
    /// 价格
    pub price: f64,
    /// 商品名
    pub unit_name: Option<String>,
    /// 产品编号
    pub product_sn: u32,
    /// 产品名
    pub product_name: String,
    /// 购买数量
    pub buy_quantity: u32,
    /// 店铺id
    pub store_code: Option<u32>,
    /// 品牌id
    pub brand_code: Option<u32>,
    /// 商品属性信息
    pub unit_attr_info: Vec<UnitAttrInfo>,
    /// 当前产品，支持的物流方式
    pub support_delivery: Vec<DeliveryType>,
}
#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct PrePareRes {
    /// 合计多少件
    pub total_quantity: u32,
    /// 合计多少钱,(元)
    pub total_amount: f64,
    /// 优惠多少钱,(元)
    pub reduce_amount: f64,
    /// 优惠描述信息
    pub reduce_des: Vec<String>,
    /// 实际多少钱,(元)
    pub pay_amount: f64,
    /// 用户购买的哪些商品
    pub user_buy: Vec<UserBuy>,
    /// 优惠券，是否已使用
    pub is_coupon_used: bool,
    /// 用户的优惠券编号id
    pub usr_coupon_id: Option<u64>,
}
/// 获取预览订单
pub fn get_order_prepare(
    tran: &mut Transaction,
    uid: u64,
    unit_sns: &Vec<u32>,
    shop_cart_status: &ShopCartStatus,
    coupon_id: Option<u32>,
    is_lock: bool,
) -> Result<PrePareRes, Error> {
    let lock = if is_lock { MY_EXCLUSIVE_LOCK } else { "" };

    let unit_info = unit_sns
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(",");

    #[derive(Serialize, Debug, Deserialize, Clone)]
    struct UserBuyGet {
        id: u64,
        unit_sn: u32,
        unit_cover: Option<String>,
        price: String,
        unit_name: Option<String>,
        product_sn: u32,
        product_name: String,
        store_code: Option<u32>,
        brand_code: Option<u32>,
        buy_quantity: u32,
        unit_attr_info: Option<String>,
        delivery_type: String,
    }
    let user_shop_unit: Vec<UserBuyGet> = my_run_tran_vec(
        tran,
        myfind!("ord_shop_cart", {
            j0: ["unit_sn", "inner", "sku_unit.unit_sn"],
            j1: ["sku_unit.product_sn", "inner", "spu_product.product_sn"],
            p0: ["uid", "=", uid],
            p1: ["status", "=", shop_cart_status.clone() as i8],
            p2: ["unit_sn", "in", unit_info],
            p3: ["unit_sn", "=", unit_sns[0]],
            p4: ["is_del", "=", 0],
            r: if shop_cart_status == &ShopCartStatus::BuyNow {
                "p0 && p1 && p3 && p4"
            } else {
                "p0 && p1 && p2 && p4"
            },
            order_by: "-created_at",
            select: "id, unit_sn, unit_cover, unit_name, sku_unit.price,
                    spu_product.store_code,spu_product.brand_code, spu_product.delivery_type,
                    sku_unit.product_sn, product_name, buy_quantity, unit_attr_info",
        }) + lock,
    )?;
    // 购物车里没有相关信息
    if user_shop_unit.len() == 0 {
        return Err(error::ErrorBadRequest("没有可支付的商品"));
    }

    let calc_user_shop_unit;
    if shop_cart_status == &ShopCartStatus::BuyNow {
        // 如果是立即购买，则只取最新的一条数据
        calc_user_shop_unit = vec![user_shop_unit[0].clone()];
    } else {
        calc_user_shop_unit = user_shop_unit;
    }

    let user_shop_unit: Vec<UserBuy> = calc_user_shop_unit
        .into_iter()
        .map(|x| UserBuy {
            id: x.id,
            unit_sn: x.unit_sn,
            unit_cover: get_file_url(x.unit_cover).unwrap_or("".to_string()),
            price: x.price.parse::<f64>().unwrap(),
            product_name: x.product_name,
            product_sn: x.product_sn,
            unit_name: x.unit_name,
            buy_quantity: x.buy_quantity,
            store_code: x.store_code,
            brand_code: x.brand_code,
            unit_attr_info: if let Some(u) = x.unit_attr_info {
                serde_json::from_str::<Vec<UnitAttrInfo>>(&u).unwrap()
            } else {
                vec![]
            },
            support_delivery: x
                .delivery_type
                .split(",")
                .map(|x| x.into())
                .collect::<Vec<DeliveryType>>(),
        })
        .collect();

    // 计算总件数，和总价格
    let mut total_price = 0.;
    let mut total_count = 0;

    for i in 0..user_shop_unit.len() {
        total_count = total_count + user_shop_unit[i].buy_quantity;
        total_price =
            total_price + ((user_shop_unit[i].buy_quantity as f64) * user_shop_unit[i].price);
    }

    let mut pay_price = total_price;
    let mut reduce_price = 0.;
    let mut reduce_des = vec![];
    let mut coupon_used = false;
    let mut usr_coupon_id = None;

    // 获取用户的优惠券信息
    if let Some(cou_id) = coupon_id {
        let coupon_reduce_info =
            calc_user_coupon_data(tran, uid, &user_shop_unit, cou_id, is_lock)?;

        usr_coupon_id = coupon_reduce_info.usr_coupon_id;

        if coupon_reduce_info.total_for_reduce > 0. {
            // 有用于优惠的金额
            if let Some(d) = coupon_reduce_info.c_reduce_amount {
                let reduce_am = d.parse::<f64>().unwrap();
                if coupon_reduce_info.total_for_reduce - reduce_am > 0. {
                    reduce_price = reduce_am;
                    pay_price = total_price - reduce_am;
                    reduce_des = coupon_reduce_info.reduce_des.clone();
                    coupon_used = true;
                }
            }
            if let Some(d) = coupon_reduce_info.c_discount {
                let discount = d.parse::<f64>().unwrap();
                if discount > 0. && discount < 1. {
                    reduce_price = coupon_reduce_info.total_for_reduce
                        - coupon_reduce_info.total_for_reduce * discount;
                    pay_price = total_price - reduce_price;
                    reduce_des = coupon_reduce_info.reduce_des;
                    coupon_used = true;
                }
            }
        }
    }

    Ok(PrePareRes {
        total_amount: keep_decimal(total_price),
        total_quantity: total_count,
        reduce_des,
        reduce_amount: keep_decimal(reduce_price),
        pay_amount: keep_decimal(pay_price),
        user_buy: user_shop_unit,
        is_coupon_used: coupon_used,
        usr_coupon_id,
    })
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct UserCouponData {
    /// 用户的优惠券编号id
    pub usr_coupon_id: Option<u64>,
    /// 总共可用于减的金额
    pub total_for_reduce: f64,
    /// 优惠券的描述
    pub reduce_des: Vec<String>,
    /// 优惠券，可减的金额
    pub c_reduce_amount: Option<String>,
    /// 优惠券，可打折的
    pub c_discount: Option<String>,
}
/// 获取用户的优惠结果
pub fn calc_user_coupon_data(
    tran: &mut Transaction,
    uid: u64,
    user_buy: &Vec<UserBuy>,
    coupon_id: u32,
    is_lock: bool,
) -> Result<UserCouponData, Error> {
    let lock = if is_lock { MY_EXCLUSIVE_LOCK } else { "" };

    // 优惠券，信息
    let mut reduce_info = UserCouponData {
        usr_coupon_id: None,
        total_for_reduce: 0.,
        reduce_des: vec![],
        c_reduce_amount: None,
        c_discount: None,
    };

    #[derive(Serialize, Debug, Deserialize, Clone)]
    struct UserCouponGet {
        id: u64,
        coupon_id: u32,
        coupon_name: String,
        reduce_amount: Option<String>,
        discount: Option<String>,
        expire_time: Option<String>,
        status: i8,
        is_del: i8,
        cc_id: u32,
        cc_title: String,
        cc_full_amount: Option<String>,
        cc_store_code: Option<u32>,
        cc_brand_code: Option<u32>,
        cc_product_cat: Option<String>,
        cc_product_sn: Option<u32>,
        cc_unit_sn: Option<u32>,
    }
    let user_coupons: Vec<UserCouponGet> = my_run_tran_vec(
        tran,
        myfind!("usr_coupon", {
            j0: ["coupon_id", "inner", "pmt_coupon.id"],
            j1: ["pmt_coupon.coupon_condition_id", "inner", "pmt_coupon_condition.id"],
            p0: ["uid", "=", uid],
            p1: ["status", "=", UserCouponStatus::NotUsed as i8],
            p2: ["coupon_id", "=", coupon_id],
            r: "p0 && p1 && p2",
            select: r#"
                id,pmt_coupon.id as coupon_id,pmt_coupon.coupon_name,pmt_coupon.reduce_amount,pmt_coupon.discount,
                pmt_coupon.expire_time,pmt_coupon.status,pmt_coupon.is_del,pmt_coupon_condition.id as cc_id,
                pmt_coupon_condition.title as cc_title,pmt_coupon_condition.full_amount as cc_full_amount,
                pmt_coupon_condition.store_code as cc_store_code,pmt_coupon_condition.brand_code as cc_brand_code,
                pmt_coupon_condition.product_cat as cc_product_cat,pmt_coupon_condition.product_sn as cc_product_sn,
                pmt_coupon_condition.unit_sn as cc_unit_sn
            "#,
        }) + lock,
    )?;

    if user_coupons.len() == 0 {
        return Err(error::ErrorBadRequest("没有找到对应可用优惠券"));
    }
    if user_coupons[0].is_del == 1 {
        return Err(error::ErrorBadRequest("没有找到对应可用优惠券"));
    }
    if let Some(time) = user_coupons[0].expire_time.clone() {
        if time <= get_now_time(NowTimeType::DateTime) {
            return Err(error::ErrorBadRequest("优惠券已过期"));
        }
    }
    if user_coupons[0].status != NormalStatus::Online as i8 {
        return Err(error::ErrorBadRequest("优惠券已下架"));
    }

    reduce_info.usr_coupon_id = Some(user_coupons[0].id);
    reduce_info.c_reduce_amount = user_coupons[0].reduce_amount.clone();
    reduce_info.c_discount = user_coupons[0].discount.clone();
    // 是否有满减条件，没有，则为0
    let full_amount: f64;
    if let Some(f) = &user_coupons[0].cc_full_amount {
        full_amount = f.parse().unwrap();
    } else {
        full_amount = 0.;
    }

    // 是否有指定店铺，没有则为0
    let store_code: u32;
    if let Some(c) = &user_coupons[0].cc_store_code {
        store_code = c.to_owned();
    } else {
        store_code = 0;
    }

    // 是否有指定品牌，没有则为0
    let brand_code: u32;
    if let Some(c) = &user_coupons[0].cc_brand_code {
        brand_code = c.to_owned();
    } else {
        brand_code = 0;
    }

    // 是指定商品优惠券, 则就不用看 product,cat,brand 了
    if let Some(unit_sn) = user_coupons[0].cc_unit_sn {
        let mut am: f64 = 0.; //可用于优惠券的，合计价格。
        for i in 0..user_buy.len() {
            if user_buy[i].unit_sn == unit_sn {
                if store_code > 0 {
                    // 指定了店铺，则要该店铺的
                    if user_buy[i].store_code == user_coupons[0].cc_store_code {
                        am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                    }
                } else {
                    am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                }
            }
        }
        if am >= full_amount {
            // 满足满减条件
            reduce_info.total_for_reduce = am;
        }
        reduce_info.reduce_des.push(format!(
            "{}{}",
            user_coupons[0].cc_title, user_coupons[0].coupon_name
        ));
        return Ok(reduce_info);
    }

    // 是指定产品优惠券，则不用看  ,cat,brand
    if let Some(product_sn) = user_coupons[0].cc_product_sn {
        let mut am: f64 = 0.; // 的合计价格。
        for i in 0..user_buy.len() {
            if user_buy[i].product_sn == product_sn {
                if store_code > 0 {
                    // 指定了店铺，则要该店铺的
                    if user_buy[i].store_code == user_coupons[0].cc_store_code {
                        am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                    }
                } else {
                    am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                }
            }
        }
        if am >= full_amount {
            // 满足满减条件
            reduce_info.total_for_reduce = am;
        }
        reduce_info.reduce_des.push(format!(
            "{}{}",
            user_coupons[0].cc_title, user_coupons[0].coupon_name
        ));
        return Ok(reduce_info);
    }

    // 是指定产品类别优惠券，
    if let Some(product_cat) = user_coupons[0].cc_product_cat.clone() {
        // 查寻所有产品的类别
        #[allow(unused)]
        #[derive(Deserialize, Debug)]
        struct ProductCatItem {
            product_sn: u32,
            primary_id: u32,
            secondary_id: u32,
            tertiary_id: u32,
        }
        let p_sns = user_buy
            .iter()
            .map(|x| x.product_sn.to_string())
            .collect::<Vec<String>>();
        let cat_list: Vec<ProductCatItem> = my_run_tran_vec(
            tran,
            myfind!("spu_product_cat", {
                p0: ["product_sn", "in", p_sns.join(",")],
                p1: ["is_del", "=", 0],
                r: "p0 && p1",
            }),
        )?;

        // 优惠券指定的产品类别
        let cp_p_cat = product_cat
            .split(",")
            .map(|x| x.parse::<u32>().unwrap())
            .collect::<Vec<u32>>();
        // 的合计价格。
        let mut am: f64 = 0.;
        for i in 0..user_buy.len() {
            // 当前商品，对应的产品类别
            let by_cat = cat_list
                .iter()
                .find(|x| x.product_sn == user_buy[i].product_sn);
            if let Some(cat) = by_cat {
                // 当前商品的类别存在 且为3级
                if cp_p_cat.len() == 3 {
                    if cp_p_cat[0] == cat.primary_id
                        && cp_p_cat[1] == cat.secondary_id
                        && cp_p_cat[2] == cat.tertiary_id
                    {
                        if store_code > 0 && brand_code > 0 {
                            // 指定了店铺 和 品牌，则要该店铺 同时和品牌的
                            if user_buy[i].store_code == user_coupons[0].cc_store_code
                                && user_buy[i].brand_code == user_coupons[0].cc_brand_code
                            {
                                am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                            }
                        } else if store_code > 0 {
                            if user_buy[i].store_code == user_coupons[0].cc_store_code {
                                am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                            }
                        } else if brand_code > 0 {
                            if user_buy[i].brand_code == user_coupons[0].cc_brand_code {
                                am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                            }
                        } else {
                            am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                        }
                    }
                }
                // 当前商品的类别存在 且为2级
                if cp_p_cat.len() == 2 {
                    if cp_p_cat[0] == cat.primary_id && cp_p_cat[1] == cat.secondary_id {
                        if store_code > 0 && brand_code > 0 {
                            // 指定了店铺 和 品牌，则要该店铺 同时和品牌的
                            if user_buy[i].store_code == user_coupons[0].cc_store_code
                                && user_buy[i].brand_code == user_coupons[0].cc_brand_code
                            {
                                am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                            }
                        } else if store_code > 0 {
                            if user_buy[i].store_code == user_coupons[0].cc_store_code {
                                am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                            }
                        } else if brand_code > 0 {
                            if user_buy[i].brand_code == user_coupons[0].cc_brand_code {
                                am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                            }
                        } else {
                            am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                        }
                    }
                }
                // 当前商品的类别存在 且为1级
                if cp_p_cat.len() == 1 {
                    if cp_p_cat[0] == cat.primary_id {
                        if store_code > 0 && brand_code > 0 {
                            // 指定了店铺 和 品牌，则要该店铺 同时和品牌的
                            if user_buy[i].store_code == user_coupons[0].cc_store_code
                                && user_buy[i].brand_code == user_coupons[0].cc_brand_code
                            {
                                am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                            }
                        } else if store_code > 0 {
                            if user_buy[i].store_code == user_coupons[0].cc_store_code {
                                am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                            }
                        } else if brand_code > 0 {
                            if user_buy[i].brand_code == user_coupons[0].cc_brand_code {
                                am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                            }
                        } else {
                            am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                        }
                    }
                }
            }
        }
        if am >= full_amount {
            // 满足满减条件
            reduce_info.total_for_reduce = am;
        }
        reduce_info.reduce_des.push(format!(
            "{}{}",
            user_coupons[0].cc_title, user_coupons[0].coupon_name
        ));
        return Ok(reduce_info);
    }

    // 是店铺 或 品牌 的优惠券
    if store_code > 0 || brand_code > 0 {
        // 的合计价格。
        let mut am: f64 = 0.;
        for i in 0..user_buy.len() {
            if store_code > 0 && brand_code > 0 {
                // 指定了店铺 和 品牌，则要该店铺 同时和品牌的
                if user_buy[i].store_code == user_coupons[0].cc_store_code
                    && user_buy[i].brand_code == user_coupons[0].cc_brand_code
                {
                    am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                }
            } else if store_code > 0 {
                if user_buy[i].store_code == user_coupons[0].cc_store_code {
                    am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                }
            } else if brand_code > 0 {
                if user_buy[i].brand_code == user_coupons[0].cc_brand_code {
                    am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
                }
            } else {
                am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
            }
        }
        if am >= full_amount {
            // 满足满减条件
            reduce_info.total_for_reduce = am;
        }
        reduce_info.reduce_des.push(format!(
            "{}{}",
            user_coupons[0].cc_title, user_coupons[0].coupon_name
        ));

        return Ok(reduce_info);
    }

    // 单纯的满减条件
    if full_amount > 0. {
        // 的合计价格。
        let mut am: f64 = 0.;
        for i in 0..user_buy.len() {
            am += user_buy[i].price * (user_buy[i].buy_quantity as f64);
        }
        if am >= full_amount {
            // 满足满减条件
            reduce_info.total_for_reduce = am;
        }
        reduce_info.reduce_des.push(format!(
            "{}{}",
            user_coupons[0].cc_title, user_coupons[0].coupon_name
        ));
        return Ok(reduce_info);
    }

    Ok(reduce_info)
}

/// 修改主订单的支付状态：2 已支付，1 待支付，0 取消支付,
pub fn upd_order_status(
    tran: &mut Transaction,
    order_sn: &String,
    status: OrderPayStatus,
    tran_id: Option<String>,
) -> Result<(), Error> {
    my_run_tran_drop(
        tran,
        myupdate!("ord_order", {"order_sn": order_sn}, {
            "status": status as u8,
            "transaction_id": &tran_id,
        }),
    )?;
    Ok(())
}

/// 增加产品和商品的销量计数
pub fn upd_product_unit_sell_total(tran: &mut Transaction, order_sn: &String) -> Result<(), Error> {
    // 查询，当前订单下面的，所有商品
    #[derive(Deserialize, Serialize)]
    struct OrderItemGet {
        order_sn: String,
        unit_sn: u64,
        product_sn: u64,
        buy_quantity: u32,
    }
    let item_list: Vec<OrderItemGet> = my_run_tran_vec(
        tran,
        myfind!("ord_order_item", {
            j0: ["unit_sn", "inner", "sku_unit.unit_sn"],
            p0: ["order_sn", "=", order_sn],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "order_sn, unit_sn, sku_unit.product_sn, buy_quantity",
        }),
    )?;
    for item in item_list {
        my_run_tran_drop(
            tran,
            myupdate!("sku_unit", {"unit_sn": &item.unit_sn}, {
                "sell_total": ["incr", item.buy_quantity],
            }),
        )?;
        my_run_tran_drop(
            tran,
            myupdate!("spu_product", {"product_sn": &item.product_sn}, {
                "sell_total": ["incr", item.buy_quantity],
            }),
        )?;
    }
    Ok(())
}

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct OrderChangeItems {
    /// 商品单号
    pub order_item_id: String,
    /// 订单状态
    pub status: u8,
}
#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct OrderChange {
    /// 订单号
    pub order_sn: String,
    /// 商品订单状态
    pub order_items: Vec<OrderChangeItems>,
}
/// 修改子订单的物流状态
/// 0 待发货，1 待收货, 2 已完成, 3 已评价，4 申请退货，5 已退货
pub fn upd_order_item_status(
    tran: &mut Transaction,
    order_item_id: &str,
    status: OrderItemStatus,
) -> Result<(), Error> {
    my_run_tran_drop(
        tran,
        myupdate!("ord_order_item", { "order_item_id": order_item_id }, {"status": status.clone() as u8}),
    )?;

    if status == OrderItemStatus::Apply || status == OrderItemStatus::Returned {
        // 如果是，待核销的商品，，如果是退货状态，则也要修改状态
        my_run_tran_drop(
            tran,
            myupdate!("ord_write_off_item", { "order_item_id": order_item_id }, {
                "write_off_status": WriteOffStatus::Cancel as u8,
            }),
        )?;
    }

    Ok(())
}

/// 去支付，查寻用户用户openid
pub fn get_user_openid(conn: &mut PooledConn, uid: u64) -> Result<String, Error> {
    // 获取用户的 openid
    #[derive(Deserialize)]
    struct OpenId {
        openid: Option<String>,
    }
    let openid;
    let res_user: Vec<OpenId> = my_run_vec(conn, myget!("usr_silent", uid, "openid"))?;
    if let Some(o) = res_user[0].openid.clone() {
        openid = o;
    } else {
        return Err(error::ErrorBadRequest("用户未授权登录"));
    }
    Ok(openid)
}

#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct AddressGet {
    pub province: Option<String>,
    pub city: Option<String>,
    pub area: Option<String>,
    pub addr_detail: Option<String>,
    pub contact_user: Option<String>,
    pub contact_phone: Option<String>,
}
/// 去支付，查找用户地址。根据物流类型，判断要不要用户地址
pub fn get_user_address_or_none(
    tran: &mut Transaction,
    id: Option<u64>,
    delivery_type: &DeliveryType,
) -> Result<AddressGet, Error> {
    let is_need_addr = match delivery_type {
        &DeliveryType::DoDelivery => true,
        &DeliveryType::WxDelivery => true,
        &DeliveryType::WxInstant => true,
        &DeliveryType::NoDelivery => false,
        &DeliveryType::DoorPickup => false,
        &DeliveryType::StoreWriteOff => false,
    };
    if is_need_addr {
        if let Some(addr_id) = id {
            let sql_addr = myfind!("usr_address", {
                p0: ["id", "=", addr_id],
                p1: ["is_del", "=", 0],
                r: "p0 && p1",
                select: "province,city,area,addr_detail,contact_user,contact_phone",
            });
            let user_addr: Vec<AddressGet> = my_run_tran_vec(tran, sql_addr)?;
            if user_addr.is_empty() {
                Err(error::ErrorNotFound("用户地址不存在"))
            } else {
                Ok(user_addr[0].clone())
            }
        } else {
            Err(error::ErrorBadGateway("用户地址id不能为空"))
        }
    } else {
        Ok(AddressGet {
            province: None,
            city: None,
            area: None,
            addr_detail: None,
            contact_user: None,
            contact_phone: None,
        })
    }
}

/// 去支付，生成一个总订单，和子订单项
/// 返回 (订单号, 产品描述)
pub fn create_order(
    tran: &mut Transaction,
    data: &Data<AppData>,
    uid: u64,
    prepare: &PrePareRes,
    params: &MakePay,
    user_addr: &AddressGet,
    pay_type: &PayType,
) -> Result<(String, String), Error> {
    let order_sn = data.rand_no(SlownWorker::OrderSn);
    let sql_all = myset!("ord_order", {
        "uid": uid,
        "order_sn": &order_sn,
        "total_amount": prepare.total_amount,
        "pay_amount": prepare.pay_amount,
        "total_quantity": prepare.total_quantity,
        "reduce_amount": prepare.reduce_amount,
        "reduce_des": &prepare.reduce_des.join(","),
        "delivery_type": &params.delivery_type.to_string(),
        "notes": &params.notes,
        "appointment_time": &params.appointment_time,
        "province": &user_addr.province,
        "city": &user_addr.city,
        "area": &user_addr.area,
        "addr_detail": &user_addr.addr_detail,
        "contact_user": &user_addr.contact_user,
        "contact_phone": &user_addr.contact_phone,
        "pay_type": pay_type.to_string(),
    });

    #[derive(Serialize, Debug, Deserialize)]
    struct OrderItem {
        uid: u64,
        order_sn: String,
        order_item_id: String,
        unit_sn: u32,
        unit_name: Option<String>,
        unit_attr_info: String,
        product_name: String,
        unit_cover: String,
        price: f64,
        buy_quantity: u32,
        amount: f64,
    }
    let mut pay_des: Vec<String> = vec![];
    let order_items: Vec<OrderItem> = prepare
        .user_buy
        .iter()
        .map(|x| {
            let u_name = x.unit_name.clone().unwrap_or("".to_string());
            pay_des.push(format!("{}-{}", &u_name, &x.product_name));
            OrderItem {
                uid,
                order_sn: order_sn.clone(),
                order_item_id: data.rand_id(SlownWorker::OrderItemId),
                unit_sn: x.unit_sn,
                unit_name: x.unit_name.clone(),
                unit_attr_info: if x.unit_attr_info.len() > 0 {
                    serde_json::to_string(&x.unit_attr_info).unwrap()
                } else {
                    "null".to_string()
                },
                product_name: x.product_name.clone(),
                unit_cover: get_path_from_url(&x.unit_cover, &OssBucket::EobFiles),
                price: x.price,
                buy_quantity: x.buy_quantity,
                amount: x.price * x.buy_quantity as f64,
            }
        })
        .collect();
    let sql_items = mysetmany!("ord_order_item", order_items);
    my_run_tran_drop(tran, sql_all)?;
    my_run_tran_drop(tran, sql_items)?;
    Ok((order_sn, pay_des.join("、")))
}

#[derive(Serialize, Debug, Deserialize)]
struct ShopUpd {
    id: u64,
    status: u8,
}
/// 去支付，更新购物车状态
pub fn upd_shop_cart_status(
    tran: &mut Transaction,
    user_buy: &Vec<UserBuy>,
    buy_type: &ShopCartStatus,
) -> Result<(), Error> {
    let upd_cart_type = match buy_type {
        &ShopCartStatus::BuyNow => ShopCartStatus::BuyNowPaid,
        &ShopCartStatus::PendingPayment => ShopCartStatus::Paid,
        _ => ShopCartStatus::Wrong,
    };
    if upd_cart_type == ShopCartStatus::Wrong {
        return Err(error::ErrorBadRequest("buy_type 参数错误"));
    }
    let shop_items: Vec<ShopUpd> = user_buy
        .iter()
        .map(|x| ShopUpd {
            id: x.id,
            status: upd_cart_type.clone() as u8,
        })
        .collect();
    let sql_shop = myupdatemany!("ord_shop_cart", "id", shop_items);
    my_run_tran_drop(tran, sql_shop)?;
    Ok(())
}

/// 去支付，如果用户使用了优惠券，则修改为已使用
pub fn upd_coupon_status(tran: &mut Transaction, coupon_id: u64) -> Result<(), Error> {
    my_run_tran_drop(
        tran,
        myupdate!("usr_coupon", coupon_id, { "status":  UserCouponStatus::Used as i8}),
    )?;
    Ok(())
}

/// 微信物流，发货
#[allow(unused)]
pub fn auto_add_wx_waybill(
    tran: &mut Transaction,
    data: &Data<AppData>,
    order_sn: &str,
) -> Result<(), Error> {
    // 获取当前订单信息
    #[derive(Deserialize)]
    struct OrderGet {
        order_sn: String,
        uid: u64,
        notes: Option<String>,
        appointment_time: Option<String>,
        province: Option<String>,
        city: Option<String>,
        area: Option<String>,
        addr_detail: Option<String>,
        contact_user: Option<String>,
        contact_phone: Option<String>,
        delivery_type: String,
    }
    let order_get: Vec<OrderGet> =
        my_run_tran_vec(tran, myget!("ord_order", {"order_sn": order_sn}))?;
    if order_get.len() == 0 {
        return Err(error::ErrorBadRequest("订单信息不存在"));
    }
    if order_get[0].province.is_none() || order_get[0].city.is_none() || order_get[0].area.is_none()
    {
        return Err(error::ErrorBadRequest("收件地址信息不完整"));
    }
    if order_get[0].contact_user.is_none() || order_get[0].contact_phone.is_none() {
        return Err(error::ErrorBadRequest("收件人信息不完整"));
    }
    if order_get[0].delivery_type != DeliveryType::WxDelivery.to_string() {
        return Err(error::ErrorBadRequest("订单物流不是微信物流"));
    }

    // 获取订单下面的所有子订单信息
    #[derive(Deserialize)]
    struct OrderItemGet {
        order_sn: String,
        order_item_id: String,
        unit_name: String,
        unit_cover: String,
        // 还要有产品信息，product 产品对应的寄件人
    }
    let order_items: Vec<OrderItemGet> = my_run_tran_vec(
        tran,
        myfind!("ord_order_item", {
            p0: ["order_sn", "=", order_sn],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
        }),
    )?;

    // 将 order_item_id 为同一个寄件人的，组合成一个 delivery_code。并记录他们的关系
    let delivery_code = data.rand_id(SlownWorker::DeliveryCode);

    // 根据每个 delivery_code  生成对应微信发货物流单
    // 这就是，同一个收件人，但可能有不同的寄件地址

    Ok(())
}

#[derive(Serialize, Debug, Deserialize)]
pub struct UserProductUpd {
    pub uid: u64,
    pub status: u8,
    pub is_del: u8,
}
/// 用户的产品状态的批量修改
#[allow(unused)]
pub fn upd_user_product_multiple_status(
    tran: &mut Transaction,
    user_product_list: &Vec<UserProductUpd>,
) -> Result<(), Error> {
    let sql = myupdatemany!("spu_product", "uid,is_del", user_product_list);
    my_run_tran_drop(tran, sql)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::{UnitAttrInfo, UserProductUpd};
    use mysql_quick::{myfind, myset, myupdatemany};

    #[test]
    fn test_myfind() {
        let _sql = myfind!("usr_coupon", {
            j0: ["coupon_id", "inner", "pmt_coupon.id"],
            j1: ["pmt_coupon.coupon_condition_id", "inner", "pmt_coupon_condition.id"],
            p0: ["uid", "=", 1],
            p1: ["status", "=", 2],
            r: "p0 && p1",
            select: r#"
            id,pmt_coupon.id as coupon_id,pmt_coupon.coupon_name,pmt_coupon.reduce_amount,pmt_coupon.discount,
            pmt_coupon.expire_time,pmt_coupon.status,pmt_coupon.is_del,pmt_coupon_condition.id as cc_id,
            pmt_coupon_condition.title as cc_title,pmt_coupon_condition.full_amount as cc_full_amount,
            pmt_coupon_condition.store_code as cc_store_code,pmt_coupon_condition.brand_code as cc_brand_code,
            pmt_coupon_condition.product_cat as cc_product_cat,pmt_coupon_condition.product_sn as cc_product_sn,
            pmt_coupon_condition.unit_sn as cc_unit_sn
            "#,
        });
        // println!("ccsss  {}", sql);
        assert!(true)
    }
    #[test]
    fn test_json() {
        let unit_attrs = vec![UnitAttrInfo {
            primary_name: "内在".to_string(),
            secondary_name: "载".to_string(),
        }];
        let s = serde_json::to_string(&unit_attrs).unwrap();
        println!("sss,,  {}", s);
        let d: Vec<UnitAttrInfo> = serde_json::from_str(&s).unwrap();
        println!("dd,,,  {:?}", d);

        let mut v_r = s.as_str().replace("\\", "\\\\");
        v_r = v_r.replace("\"", "\\\"");

        println!("v_r,,,  {}", v_r);

        let sql = myset!("talbe", {
           "name": s
        });
        println!("sql,,,  {}", sql);

        let sql = myset!("talbe", {
           "name": r#"m'y,,a#@!@$$^&^%&&#$,,adflll+_)"(\_)*)(32389)d(ŐдŐ๑)🍉 .',"#
        });
        println!("sql,,,  {}", sql);

        // let u = "[{\"primary_name\":\"内存\",\"secondary_name\":\"4G+256G\"},{\"primary_name\":\"颜色\",\"secondary_name\":\"红色\"}]";
        // println!("u,,,,  {}", u);
        // let uu = serde_json::to_string(u).unwrap();
        // println!("u,,,,  {}", uu);
        // let a = serde_json::from_str::<Vec<UnitAttrInfo>>(u).unwrap();
        // println!("aaa,,,,  {:?}", a);
    }
    #[test]
    fn test_user_upd() {
        let a = UserProductUpd {
            uid: 1,
            status: 3,
            is_del: 0,
        };
        let sql = myupdatemany!("spu_product", "uid,is_del", vec![&a]);
        println!("sql....  {}", sql)
    }
}
