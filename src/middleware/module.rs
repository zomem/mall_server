use crate::db::my_run_vec;
use crate::db::mysql_conn;
use actix_web::{dev::Payload, error, Error, FromRequest, HttpRequest};
use mysql_quick::{myfind, PooledConn};
use serde::Deserialize;
use std::future::{ready, Ready};

// 功能块 code
const COUPON: &str = "COUPON";
const DISTRIBUTION: &str = "DISTRIBUTION";
const POCKET_MONEY: &str = "POCKET_MONEY";
const AGENT: &str = "AGENT";
const JOIN: &str = "JOIN";
const SHOPPING_CART: &str = "SHOPPING_CART";
const SECKILL: &str = "SECKILL";
const GROUP_BUY: &str = "GROUP_BUY";
const ARTICLE: &str = "ARTICLE";
// 不是任何功能块
const NOT_MODULE: &str = "NOT_MODULE";

/// 判断当前功能块，是否开启了
#[derive(PartialEq, Eq)]
pub enum Module {
    Coupon,
    Distribution,
    PocketMoney,
    Agent,
    Join,
    ShoppingCart,
    Seckill,
    GroupBuy,
    Article,
    NotModule,
}
impl Module {
    fn to_string(&self) -> String {
        match &self {
            Module::Coupon => COUPON.to_string(),
            Module::Distribution => DISTRIBUTION.to_string(),
            Module::PocketMoney => POCKET_MONEY.to_string(),
            Module::Agent => AGENT.to_string(),
            Module::Join => JOIN.to_string(),
            Module::ShoppingCart => SHOPPING_CART.to_string(),
            Module::Seckill => SECKILL.to_string(),
            Module::GroupBuy => GROUP_BUY.to_string(),
            Module::Article => ARTICLE.to_string(),
            Module::NotModule => NOT_MODULE.to_string(),
        }
    }
}
impl From<String> for Module {
    fn from(value: String) -> Self {
        match value.as_str() {
            COUPON => Self::Coupon,
            DISTRIBUTION => Self::Distribution,
            POCKET_MONEY => Self::PocketMoney,
            AGENT => Self::Agent,
            JOIN => Self::Join,
            SHOPPING_CART => Self::ShoppingCart,
            SECKILL => Self::Seckill,
            GROUP_BUY => Self::GroupBuy,
            ARTICLE => Self::Article,
            _ => Self::NotModule,
        }
    }
}
fn get_module_by_code(conn: &mut PooledConn, md: Module) -> Result<Module, Error> {
    #[derive(Deserialize)]
    struct ModuleGet {
        code: String,
    }
    let code = md.to_string();
    let m: Vec<ModuleGet> = my_run_vec(
        conn,
        myfind!("sys_module_switch", {
            p0: ["is_on", "=", 1],
            p1: ["is_del", "=", 0],
            p2: ["code", "=", &code],
            r: "p0 && p1 && p2",
        }),
    )?;
    if m.len() > 0 {
        let module: Module = m[0].code.clone().into();
        if module == Module::NotModule {
            return Err(error::ErrorForbidden(format!(
                "没有开通[{}]功能模块",
                &code
            )));
        }
        Ok(module)
    } else {
        Err(error::ErrorForbidden(format!(
            "没有开通[{}]功能模块",
            &code
        )))
    }
}

/// 优惠券功能，是否开启的鉴权。后面类似
#[allow(unused)]
pub struct ModuleCoupon;
impl FromRequest for ModuleCoupon {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(_req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            match mysql_conn() {
                Ok(c) => {
                    let mut conn = c;
                    match get_module_by_code(&mut conn, Module::Coupon) {
                        Ok(_) => Ok(ModuleCoupon),
                        Err(e) => Err(e),
                    }
                }
                Err(_) => Err(error::ErrorInternalServerError("Module 数据库连接错误")),
            }
        })
    }
}
#[allow(unused)]
pub struct ModuleDistribution;
impl FromRequest for ModuleDistribution {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(_req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            match mysql_conn() {
                Ok(c) => {
                    let mut conn = c;
                    match get_module_by_code(&mut conn, Module::Coupon) {
                        Ok(_) => Ok(ModuleDistribution),
                        Err(e) => Err(e),
                    }
                }
                Err(_) => Err(error::ErrorInternalServerError("Module 数据库连接错误")),
            }
        })
    }
}
#[allow(unused)]
pub struct ModulePocketMoney;
impl FromRequest for ModulePocketMoney {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(_req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            match mysql_conn() {
                Ok(c) => {
                    let mut conn = c;
                    match get_module_by_code(&mut conn, Module::Coupon) {
                        Ok(_) => Ok(ModulePocketMoney),
                        Err(e) => Err(e),
                    }
                }
                Err(_) => Err(error::ErrorInternalServerError("Module 数据库连接错误")),
            }
        })
    }
}
#[allow(unused)]
pub struct ModuleAgent;
impl FromRequest for ModuleAgent {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(_req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            match mysql_conn() {
                Ok(c) => {
                    let mut conn = c;
                    match get_module_by_code(&mut conn, Module::Coupon) {
                        Ok(_) => Ok(ModuleAgent),
                        Err(e) => Err(e),
                    }
                }
                Err(_) => Err(error::ErrorInternalServerError("Module 数据库连接错误")),
            }
        })
    }
}
#[allow(unused)]
pub struct ModuleJoin;
impl FromRequest for ModuleJoin {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(_req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            match mysql_conn() {
                Ok(c) => {
                    let mut conn = c;
                    match get_module_by_code(&mut conn, Module::Coupon) {
                        Ok(_) => Ok(ModuleJoin),
                        Err(e) => Err(e),
                    }
                }
                Err(_) => Err(error::ErrorInternalServerError("Module 数据库连接错误")),
            }
        })
    }
}
#[allow(unused)]
pub struct ModuleShoppingCart;
impl FromRequest for ModuleShoppingCart {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(_req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            match mysql_conn() {
                Ok(c) => {
                    let mut conn = c;
                    match get_module_by_code(&mut conn, Module::Coupon) {
                        Ok(_) => Ok(ModuleShoppingCart),
                        Err(e) => Err(e),
                    }
                }
                Err(_) => Err(error::ErrorInternalServerError("Module 数据库连接错误")),
            }
        })
    }
}
#[allow(unused)]
pub struct ModuleSeckill;
impl FromRequest for ModuleSeckill {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(_req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            match mysql_conn() {
                Ok(c) => {
                    let mut conn = c;
                    match get_module_by_code(&mut conn, Module::Coupon) {
                        Ok(_) => Ok(ModuleSeckill),
                        Err(e) => Err(e),
                    }
                }
                Err(_) => Err(error::ErrorInternalServerError("Module 数据库连接错误")),
            }
        })
    }
}
#[allow(unused)]
pub struct ModuleGroupBuy;
impl FromRequest for ModuleGroupBuy {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(_req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            match mysql_conn() {
                Ok(c) => {
                    let mut conn = c;
                    match get_module_by_code(&mut conn, Module::Coupon) {
                        Ok(_) => Ok(ModuleGroupBuy),
                        Err(e) => Err(e),
                    }
                }
                Err(_) => Err(error::ErrorInternalServerError("Module 数据库连接错误")),
            }
        })
    }
}
#[allow(unused)]
pub struct ModuleArticle;
impl FromRequest for ModuleArticle {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;
    fn from_request(_req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready({
            match mysql_conn() {
                Ok(c) => {
                    let mut conn = c;
                    match get_module_by_code(&mut conn, Module::Coupon) {
                        Ok(_) => Ok(ModuleArticle),
                        Err(e) => Err(e),
                    }
                }
                Err(_) => Err(error::ErrorInternalServerError("Module 数据库连接错误")),
            }
        })
    }
}
