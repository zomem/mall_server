use actix_web::{Responder, Result, error, get, post, web};
use mysql_quick::{TxOpts, myfind, myset};
use serde::{Deserialize, Serialize};
use serde_aux::prelude::deserialize_number_from_string;
use serde_json::Value;
use utoipa::ToSchema;

use crate::common::types::{NormalStatus, PayType, TranType};
use crate::routes::Res;
use crate::routes::utils_set::pocket_set::get_user_pocket_money;
use crate::utils::filter::deserialize_nested_json;
use crate::{
    db::{my_run_tran_drop, my_run_vec, mysql_conn},
    middleware::AuthUser,
};

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct UserPocket {
    id: u64,
    /// 零钱，单位元
    #[serde(deserialize_with = "deserialize_number_from_string")]
    amount: f64,
}
/// 【用户】获取用户零钱
#[utoipa::path(
    responses((status = 200, description = "【返回：UserPocket】", body = UserPocket))
)]
#[get("/user/pocket/money")]
pub async fn user_pocket_money(user: AuthUser) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;

    let list: Vec<UserPocket> = my_run_vec(
        &mut conn,
        myfind!("usr_pocket_money", {
            p0: ["uid", "=", uid],
            p1: ["status", "=", NormalStatus::Online as u8],
            p2: ["is_del", "=", 0],
            r: "p0 && p1 && p2",
            select: "id, amount",
        }),
    )?;
    if list.is_empty() {
        return Err(error::ErrorBadRequest("用户零钱未开通"));
    }

    Ok(web::Json(Res::success(list)))
}

/// 提现申请请求结构体
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct WithdrawRequest {
    /// 申请提现金额，单位元
    req_amount: f64,
}

/// 【用户】零钱提现申请
#[utoipa::path(
    request_body = WithdrawRequest,
    responses((status = 200, description = "提现申请提交成功", body = String))
)]
#[post("/user/pocket/withdraw_req")]
pub async fn user_pocket_withdraw_req(
    user: AuthUser,
    data: web::Json<WithdrawRequest>,
) -> Result<impl Responder> {
    let uid = user.id;
    let req_amount = data.req_amount;

    // 验证提现金额
    if req_amount <= 0.0 {
        return Err(error::ErrorBadRequest("提现金额必须大于0"));
    }

    let mut conn = mysql_conn()?;

    // 检查是否有正在审核中的提现申请
    let pending_requests: Vec<serde_json::Value> = my_run_vec(
        &mut conn,
        myfind!("usr_withdrawal_request", {
            p0: ["uid", "=", uid],
            p1: ["status", "=", 1],
            p2: ["is_del", "=", 0],
            r: "p0 && p1 && p2",
            select: "id",
        }),
    )?;
    if !pending_requests.is_empty() {
        return Err(error::ErrorBadRequest(
            "您有正在审核中的提现申请，请等待审核完成",
        ));
    }

    let mut tran = conn.start_transaction(TxOpts::default()).unwrap();

    // 检查用户零钱余额
    let pocket_info = match get_user_pocket_money(&mut tran, uid) {
        Ok(info) => info,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    if pocket_info.amount < req_amount {
        tran.rollback().unwrap();
        return Err(error::ErrorBadRequest("零钱余额不足"));
    }

    // 插入提现申请记录
    if let Err(e) = my_run_tran_drop(
        &mut tran,
        myset!("usr_withdrawal_request", {
            "uid": uid,
            "req_amount": req_amount,
        }),
    ) {
        tran.rollback().unwrap();
        return Err(e);
    }

    // 提交事务
    tran.commit().unwrap();

    Ok(web::Json(Res::success("提现申请提交成功")))
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct UserTran {
    id: u64,
    /// 交易金额，单位元。负数表示支出，正数表示收入。
    #[serde(deserialize_with = "deserialize_number_from_string")]
    tran_amount: f64,
    /// 交易类型
    tran_type: TranType,
    /// 支付方式
    pay_type: PayType,
    /// 详细内容
    #[serde(deserialize_with = "deserialize_nested_json")]
    info: Value,
    /// 交易时间
    created_at: String,
}
/// 【用户】交易记录
#[utoipa::path(
    responses((status = 200, description = "【返回：UserTran[]】", body = Vec<UserTran>)),
    params(("tran_types", description="/common/base/info 接口返回的 tran_type。可以传多个，如：PURCHASE,REFUND 购买和退款"),("page", description="页码"))
)]
#[get("/user/pocket/tran/{tran_types}/{page}")]
pub async fn user_pocket_tran(
    user: AuthUser,
    path: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let uid = user.id;
    let tran_types: String = path.0.clone();
    let page: u32 = path.1.parse().unwrap_or(1);

    let mut conn = mysql_conn()?;

    let list: Vec<UserTran> = my_run_vec(
        &mut conn,
        myfind!("usr_transaction_records", {
            p0: ["uid", "=", uid],
            p1: ["tran_type", "in", tran_types],
            p2: ["is_del", "=", 0],
            r: "p0 && p1 && p2",
            page: page,
            limit: 15,
            select: "id, tran_amount, tran_type, pay_type, info, created_at",
        }),
    )?;

    Ok(web::Json(Res::success(list)))
}
