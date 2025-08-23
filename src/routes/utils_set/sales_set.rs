use actix_web::{Error, error};
use mysql_quick::{MysqlQuickCount, Transaction, mycount, myfind, myget, myset, myupdate};
use serde::{Deserialize, Serialize};
use serde_aux::prelude::{deserialize_number_from_string, deserialize_option_number_from_string};

use crate::common::types::{NormalStatus, OrderPayStatus, PayType, Role, TranType};
use crate::db::{my_run_tran_drop, my_run_tran_vec};
use crate::routes::utils_set::pocket_set::pocket_money_add;

/// 直接添加用户为总销售
pub fn main_sale_add(tran: &mut Transaction, uid: u64) -> Result<(), Error> {
    let mut m_roles = get_user_roles(tran, uid)?;
    if !m_roles.contains(&(Role::MainSale as u16)) {
        // 如果用户不是总销售，则添加销售角色
        m_roles.push(Role::MainSale as u16);
    }
    update_user_role(tran, uid, m_roles)?;
    sale_or_main_self_bind(tran, uid, 0)?;
    sale_and_main_set(tran, uid, uid)?;
    // 同时，还要将，用户添加为销售，因为，是总销售，必定是自己的销售
    sale_add(tran, uid)?;
    Ok(())
}
/// 直接添加用户为销售
pub fn sale_add(tran: &mut Transaction, uid: u64) -> Result<(), Error> {
    let mut s_roles = get_user_roles(tran, uid)?;
    if !s_roles.contains(&(Role::Sale as u16)) {
        // 如果用户不是销售，则添加销售角色
        s_roles.push(Role::Sale as u16);
    }
    update_user_role(tran, uid, s_roles)?;
    sale_or_main_self_bind(tran, 0, uid)?;
    user_and_sale_set(tran, uid, uid)?;
    Ok(())
}

/// 总销售，邀请用户成为销售，关系建立
pub fn main_sale_invite_sale(tran: &mut Transaction, m_uid: u64, s_uid: u64) -> Result<(), Error> {
    let m_roles = get_user_roles(tran, m_uid)?;
    let mut s_roles = get_user_roles(tran, s_uid)?;

    if !m_roles.contains(&(Role::MainSale as u16)) {
        return Err(error::ErrorBadRequest("总销售身份错误"));
    }
    if !s_roles.contains(&(Role::Sale as u16)) {
        // 如果用户不是销售，则添加销售角色
        s_roles.push(Role::Sale as u16);
    }
    let is_have = is_sale_have_main(tran, s_uid)?;
    if is_have {
        // 如果用户已经是总销售的销售，则返回错误
        return Err(error::ErrorBadRequest("用户已经绑定了总销售"));
    }
    update_user_role(tran, s_uid, s_roles)?;
    sale_or_main_self_bind(tran, 0, s_uid)?;
    sale_and_main_set(tran, m_uid, s_uid)?;
    Ok(())
}

/// 销售，邀请用户，关系建立
pub fn sale_invite_user(tran: &mut Transaction, s_uid: u64, uid: u64) -> Result<(), Error> {
    let s_roles = get_user_roles(tran, s_uid)?;

    if !s_roles.contains(&(Role::Sale as u16)) {
        return Err(error::ErrorBadRequest("销售身份错误"));
    }

    let is_have = is_user_have_sale(tran, uid)?;
    if is_have {
        // 如果用户已经 绑定了 销售，则返回错误
        return Err(error::ErrorBadRequest("用户已经绑定了销售"));
    }
    user_and_sale_set(tran, s_uid, uid)?;
    Ok(())
}

/// 销售，总销售，自己与自己的绑定关系检测
fn sale_or_main_self_bind(tran: &mut Transaction, m_uid: u64, s_uid: u64) -> Result<(), Error> {
    if m_uid > 0 {
        let list: Vec<serde_json::Value> = my_run_tran_vec(
            tran,
            myfind!("sal_main_sale", {
                p0: ["main_sale_uid", "=", m_uid],
                p1: ["sale_uid", "=", m_uid],
                r: "p0 && p1",
            }),
        )?;
        if list.is_empty() {
            my_run_tran_drop(
                tran,
                myset!("sal_main_sale", {
                    "main_sale_uid": m_uid,
                    "sale_uid": m_uid,
                }),
            )?;
        }
    }
    if s_uid > 0 {
        let list: Vec<serde_json::Value> = my_run_tran_vec(
            tran,
            myfind!("sal_sale_user", {
                p0: ["sale_uid", "=", s_uid],
                p1: ["uid", "=", s_uid],
                r: "p0 && p1",
            }),
        )?;
        if list.is_empty() {
            my_run_tran_drop(
                tran,
                myset!("sal_sale_user", {
                    "sale_uid": s_uid,
                    "uid": s_uid,
                }),
            )?;
        }
    }

    Ok(())
}

/// 获取用户当前的所有角色，列表
fn get_user_roles(tran: &mut Transaction, uid: u64) -> Result<Vec<u16>, Error> {
    #[derive(Deserialize)]
    struct Get {
        role: String,
    }
    let list: Vec<Get> = my_run_tran_vec(
        tran,
        myfind!("usr_silent", {
            p0: ["id", "=", uid],
            r: "p0",
            select: "role",
        }),
    )?;

    let role = list[0]
        .role
        .split(",")
        .filter(|x| !x.is_empty())
        .map(|r| r.parse::<u16>().unwrap())
        .collect::<Vec<_>>();

    Ok(role)
}

/// 更新当前用户角色
fn update_user_role(tran: &mut Transaction, uid: u64, role: Vec<u16>) -> Result<(), Error> {
    let role_str = role
        .iter()
        .map(|r| r.to_string())
        .collect::<Vec<_>>()
        .join(",");
    my_run_tran_drop(
        tran,
        myupdate!("usr_silent", uid, {
            "role": role_str
        }),
    )?;
    Ok(())
}

/// 判断，当前销售，是否绑定了有效的总销售
fn is_sale_have_main(tran: &mut Transaction, sale_uid: u64) -> Result<bool, Error> {
    // 查询当前用户的销售总销售绑定关系
    let count: Vec<MysqlQuickCount> = my_run_tran_vec(
        tran,
        mycount!("sal_main_sale", {
            p0: ["is_del", "=", 0],
            p1: ["sale_uid", "=", sale_uid],
            r: "p0 && p1",
        }),
    )?;
    Ok(count[0].mysql_quick_count > 0)
}

/// 判断，当前用户，是否绑定了有效的销售
fn is_user_have_sale(tran: &mut Transaction, uid: u64) -> Result<bool, Error> {
    // 查询当前用户的销售用户绑定关系
    let count: Vec<MysqlQuickCount> = my_run_tran_vec(
        tran,
        mycount!("sal_sale_user", {
            p0: ["is_del", "=", 0],
            p1: ["uid", "=", uid],
            r: "p0 && p1",
        }),
    )?;
    Ok(count[0].mysql_quick_count > 0)
}

/// 建立，总销售-销售关系
fn sale_and_main_set(tran: &mut Transaction, main_uid: u64, sale_uid: u64) -> Result<(), Error> {
    // 查询当前用户的销售总销售绑定关系
    #[derive(Deserialize)]
    struct SalesGet {
        id: u32,
        main_sale_uid: u64,
        sale_uid: u64,
        is_del: u8,
    }
    let list: Vec<SalesGet> = my_run_tran_vec(
        tran,
        myfind!("sal_main_sale", {
            p1: ["main_sale_uid", "=", main_uid],
            p2: ["sale_uid", "=", sale_uid],
            r: "p1 && p2",
        }),
    )?;
    if list.len() > 0 {
        // 和当前总销售有过关系，则更新
        my_run_tran_drop(
            tran,
            myupdate!("sal_main_sale", list[0].id, {
                "is_del": 0,
                "status": NormalStatus::UnderReview as u8,
            }),
        )?;
    } else {
        // 新增
        my_run_tran_drop(
            tran,
            myset!("sal_main_sale", {
                "main_sale_uid": main_uid,
                "sale_uid": sale_uid,
            }),
        )?;
    }
    Ok(())
}

/// 建立，销售-用户关系
fn user_and_sale_set(tran: &mut Transaction, sale_uid: u64, uid: u64) -> Result<(), Error> {
    // 查询当前用户的销售总销售绑定关系
    #[derive(Deserialize)]
    struct SalesGet {
        id: u32,
        sale_uid: u64,
        uid: u64,
        is_del: u8,
    }
    let list: Vec<SalesGet> = my_run_tran_vec(
        tran,
        myfind!("sal_sale_user", {
            p1: ["sale_uid", "=", sale_uid],
            p2: ["uid", "=", uid],
            r: "p1 && p2",
        }),
    )?;
    if list.len() > 0 {
        // 和当前销售有过关系，则更新
        my_run_tran_drop(
            tran,
            myupdate!("sal_sale_user", list[0].id, {
                "is_del": 0,
                "status": NormalStatus::UnderReview as u8,
            }),
        )?;
    } else {
        // 新增
        my_run_tran_drop(
            tran,
            myset!("sal_sale_user", {
                "sale_uid": sale_uid,
                "uid": uid,
            }),
        )?;
    }
    Ok(())
}

/// 取消建立，总销售-销售关系
pub fn sale_and_main_del(
    tran: &mut Transaction,
    main_uid: u64,
    sale_uid: u64,
) -> Result<(), Error> {
    // 查询当前用户的销售总销售绑定关系
    #[derive(Deserialize)]
    struct SalesGet {
        id: u32,
        main_sale_uid: u64,
        sale_uid: u64,
        is_del: u8,
    }
    let list: Vec<SalesGet> = my_run_tran_vec(
        tran,
        myfind!("sal_main_sale", {
            p1: ["main_sale_uid", "=", main_uid],
            p2: ["sale_uid", "=", sale_uid],
            r: "p1 && p2",
        }),
    )?;
    if list.len() > 0 {
        // 和当前总销售有过关系，则更新取消
        my_run_tran_drop(
            tran,
            myupdate!("sal_main_sale", list[0].id, {
                "is_del": 1,
            }),
        )?;
    }
    Ok(())
}

/// 取消建立，销售-用户关系
pub fn user_and_sale_del(tran: &mut Transaction, sale_uid: u64, uid: u64) -> Result<(), Error> {
    // 查询当前用户的销售总销售绑定关系
    #[derive(Deserialize)]
    struct SalesGet {
        id: u32,
        sale_uid: u64,
        uid: u64,
        is_del: u8,
    }
    let list: Vec<SalesGet> = my_run_tran_vec(
        tran,
        myfind!("sal_sale_user", {
            p1: ["sale_uid", "=", sale_uid],
            p2: ["uid", "=", uid],
            r: "p1 && p2",
        }),
    )?;
    if list.len() > 0 {
        // 和当前销售有过关系，则更新
        my_run_tran_drop(
            tran,
            myupdate!("sal_sale_user", list[0].id, {
                "is_del": 1,
            }),
        )?;
    }
    Ok(())
}

pub struct UserSaleMainSale {
    pub uid: u64,
    pub sale_uid: Option<u64>,
    pub main_sale_uid: Option<u64>,
}
/// 查找，用户关联的销售，和销售关联的总销售。
/// uid 为下单用户的
pub fn get_user_sale_main_sale(
    tran: &mut Transaction,
    uid: u64,
) -> Result<UserSaleMainSale, Error> {
    let mut user_sale_main_sale = UserSaleMainSale {
        uid,
        sale_uid: None,
        main_sale_uid: None,
    };
    #[derive(Deserialize)]
    struct SalesGet {
        id: u32,
        sale_uid: u64,
        uid: u64,
        is_del: u8,
    }
    let list: Vec<SalesGet> = my_run_tran_vec(
        tran,
        myfind!("sal_sale_user", {
            p0: ["status", "=", NormalStatus::Online as i8],
            p1: ["is_del", "=", 0],
            p2: ["uid", "=", uid],
            r: "p0 && p1 && p2",
        }),
    )?;
    if list.len() > 0 {
        user_sale_main_sale.sale_uid = Some(list[0].sale_uid);
        #[derive(Deserialize)]
        struct MainSalesGet {
            id: u32,
            main_sale_uid: u64,
            sale_uid: u64,
            is_del: u8,
        }
        let list_m: Vec<MainSalesGet> = my_run_tran_vec(
            tran,
            myfind!("sal_main_sale", {
                p0: ["status", "=", NormalStatus::Online as i8],
                p1: ["is_del", "=", 0],
                p2: ["sale_uid", "=", list[0].sale_uid],
                r: "p0 && p1 && p2",
            }),
        )?;
        if list_m.len() > 0 {
            user_sale_main_sale.main_sale_uid = Some(list_m[0].main_sale_uid);
        }
    }
    Ok(user_sale_main_sale)
}

/// 订单分成操作。通过 order_sn 和 下单用户 uid 进行
pub fn do_order_sale_split(
    tran: &mut Transaction,
    order_sn: &str,
    uid: u64,
    pay_type: PayType,
) -> Result<(), Error> {
    let user_sale_main_sale = get_user_sale_main_sale(tran, uid)?;
    // 查询当前订单，状态
    #[derive(Deserialize)]
    struct OrderGet {
        id: u32,
        status: u64,
        is_del: u8,
    }
    let order: Vec<OrderGet> = my_run_tran_vec(tran, myget!("ord_order", {"order_sn": order_sn}))?;
    if order.len() == 0 {
        return Err(error::ErrorBadRequest("订单信息不存在"));
    }
    if order[0].is_del == 1 {
        return Err(error::ErrorBadRequest("订单信息不存在"));
    }
    if order[0].status != OrderPayStatus::PendingPayment as u64 {
        return Err(error::ErrorBadRequest("订单信息不是待支付"));
    }
    // 查询，当前订单下面的，所有商品
    #[derive(Deserialize, Serialize)]
    struct OrderItemGet {
        order_sn: String,
        unit_sn: u64,
        unit_name: String,
        product_name: String,
        #[serde(deserialize_with = "deserialize_number_from_string")]
        price: f64,
        buy_quantity: u32,
        #[serde(deserialize_with = "deserialize_number_from_string")]
        amount: f64,
        #[serde(deserialize_with = "deserialize_option_number_from_string")]
        main_sale_split: Option<f64>,
        #[serde(deserialize_with = "deserialize_option_number_from_string")]
        sale_split: Option<f64>,
        is_split: u8,
    }
    let item_list: Vec<OrderItemGet> = my_run_tran_vec(
        tran,
        myfind!("ord_order_item", {
            j0: ["unit_sn", "inner", "sku_unit.unit_sn"],
            p0: ["order_sn", "=", order_sn],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            select: "order_sn, unit_sn, unit_name, product_name, price, buy_quantity, amount, sku_unit.main_sale_split, sku_unit.sale_split, sku_unit.is_split",
        }),
    )?;
    for item in item_list {
        if item.is_split == 0 {
            // 当前商品不分成
            continue;
        }
        if let Some(main_sale_uid) = user_sale_main_sale.main_sale_uid
            && let Some(main_sale_split) = item.main_sale_split
        {
            pocket_money_add(
                tran,
                main_sale_uid,
                main_sale_split * (item.buy_quantity as f64),
                TranType::MainSaleSplit,
                pay_type.clone(),
                Some(&serde_json::to_string(&item).unwrap()),
            )?;
        }
        if let Some(sale_uid) = user_sale_main_sale.sale_uid
            && let Some(sale_split) = item.sale_split
        {
            pocket_money_add(
                tran,
                sale_uid,
                sale_split * (item.buy_quantity as f64),
                TranType::SaleSplit,
                pay_type.clone(),
                Some(&serde_json::to_string(&item).unwrap()),
            )?;
        }
    }

    Ok(())
}
