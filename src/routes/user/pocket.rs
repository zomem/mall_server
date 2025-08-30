use actix_web::{Responder, Result, error, get, post, web};
use mysql_quick::{TxOpts, myfind, myset, myupdate};
use serde::{Deserialize, Serialize};
use serde_aux::prelude::deserialize_number_from_string;
use serde_json::Value;
use utoipa::ToSchema;
use wx_pay::{Transfer, TransferSceneReportInfo};

use crate::common::types::{NormalStatus, PayType, TranType, WithdrawalReqStatus};
use crate::common::{WECHAT_MINI_APP_ID, WECHAT_PAY_MCH_ID, WECHAT_PAY_TRANSFER_NOTIFY_URL};
use crate::control::wx_info::wx_pay_init;
use crate::routes::Res;
use crate::routes::utils_set::hash_set::{
    hash_user_verify, hash_user_withdrawal_money, hash_user_withdrawal_money_verify,
};
use crate::routes::utils_set::pocket_set::{get_user_pocket_money, pocket_money_sub};
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

/// 【用户】提现申请
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

    let s = [
        WithdrawalReqStatus::Approved as u8,
        WithdrawalReqStatus::UnderReview as u8,
        WithdrawalReqStatus::Ing as u8,
    ]
    .iter()
    .map(|x| x.to_string())
    .collect::<Vec<String>>();
    // 检查是否有正在审核中的提现申请
    let pending_requests: Vec<serde_json::Value> = my_run_vec(
        &mut conn,
        myfind!("usr_withdrawal_request", {
            p0: ["uid", "=", uid],
            p1: ["status", "in", s.join(",")],
            p2: ["is_del", "=", 0],
            r: "p0 && p1 && p2",
            select: "id",
        }),
    )?;
    if !pending_requests.is_empty() {
        return Err(error::ErrorBadRequest("您有未完成的提现申请，请先完成。"));
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

    // 零钱减，同时有交易记录添加
    match pocket_money_sub(
        &mut tran,
        uid,
        req_amount,
        TranType::Withdraw,
        PayType::WxPay,
        Some("用户提现"),
    ) {
        Ok(_) => (),
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };

    let hash =
        hash_user_withdrawal_money(uid, req_amount, "", WithdrawalReqStatus::UnderReview as u8)?;
    // 插入提现申请记录
    if let Err(e) = my_run_tran_drop(
        &mut tran,
        myset!("usr_withdrawal_request", {
            "uid": uid,
            "req_amount": req_amount,
            "status": WithdrawalReqStatus::UnderReview as u8,
            "transfer_hash": hash,
        }),
    ) {
        tran.rollback().unwrap();
        return Err(e);
    }

    // 提交事务
    tran.commit().unwrap();

    Ok(web::Json(Res::success("提现申请提交成功")))
}

#[derive(Serialize, Deserialize, Debug, ToSchema, Clone)]
pub struct UserPendingWithdraw {
    /// 零钱，单位元
    #[serde(deserialize_with = "deserialize_number_from_string")]
    req_amount: f64,
    /// 提现状态
    status: Option<WithdrawalReqStatus>,
}
/// 【用户】待提现金额
#[utoipa::path(
    responses((status = 200, description = "【返回：UserPendingWithdraw】", body = UserPendingWithdraw))
)]
#[get("/user/pocket/pending_withdraw")]
pub async fn user_pocket_pending_withdraw(user: AuthUser) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;
    let mut info = UserPendingWithdraw {
        req_amount: 0.,
        status: None,
    };
    let list: Vec<UserPendingWithdraw> = my_run_vec(
        &mut conn,
        myfind!("usr_withdrawal_request", {
            p0: ["uid", "=", uid],
            p1: ["is_del", "=", 0],
            p2: ["status", "=", WithdrawalReqStatus::UnderReview as u8],
            p3: ["status", "=", WithdrawalReqStatus::Approved as u8],
            r: "p0 && p1 && (p2 || p3)",
            select: "req_amount, status",
        }),
    )?;
    if !list.is_empty() {
        info.req_amount = list[0].req_amount;
        info.status = list[0].clone().status;
    }

    Ok(web::Json(Res::success(info)))
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

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct WithdrawalRequestItem {
    id: u64,
    /// 申请提现金额，单位元
    #[serde(deserialize_with = "deserialize_number_from_string")]
    req_amount: f64,
    /// 提现状态
    status: WithdrawalReqStatus,
    /// 申请时间
    created_at: String,
    /// 转账单号
    out_bill_no: Option<String>,
    /// 微信转账单号
    transfer_bill_no: Option<String>,
}

/// 【用户】提现申请记录
#[utoipa::path(
    responses((status = 200, description = "【返回：WithdrawalRequestItem[]】", body = Vec<WithdrawalRequestItem>)),
    params(("page", description = "页码，从1开始"))
)]
#[get("/user/pocket/transfer/list/{page}")]
pub async fn user_pocket_transfer_list(
    user: AuthUser,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let uid = user.id;
    let page: u32 = path.parse().unwrap_or(1);
    let mut conn = mysql_conn()?;

    let list: Vec<WithdrawalRequestItem> = my_run_vec(
        &mut conn,
        myfind!("usr_withdrawal_request", {
            p0: ["uid", "=", uid],
            p1: ["is_del", "=", 0],
            r: "p0 && p1",
            page: page,
            limit: 15,
            order_by: "-created_at",
            select: "id, req_amount, status, created_at, out_bill_no, transfer_bill_no",
        }),
    )?;

    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct WithdrawalRequestInfo {
    pub mchid: String,
    pub appid: String,
    pub package: String,
}
/// 【用户】发起提现
#[utoipa::path(
    responses((status = 200, description = "转账发起成功", body = String))
)]
#[get("/user/pocket/transfer")]
pub async fn user_pocket_transfer(user: AuthUser) -> Result<impl Responder> {
    let uid = user.id;
    let mut conn = mysql_conn()?;

    // 查找当前用户状态为已通过(status=2)的提现申请
    #[derive(Serialize, Deserialize)]
    struct WithdrawalRequest {
        id: u64,
        #[serde(deserialize_with = "deserialize_number_from_string")]
        req_amount: f64,
        out_bill_no: Option<String>,
        transfer_hash: String,
        status: u8,
    }

    let withdrawal_requests: Vec<WithdrawalRequest> = my_run_vec(
        &mut conn,
        myfind!("usr_withdrawal_request", {
            p0: ["uid", "=", uid],
            p1: ["status", "=", WithdrawalReqStatus::Approved as u8],
            p2: ["is_del", "=", 0],
            r: "p0 && p1 && p2",
            select: "id, req_amount, out_bill_no, transfer_hash, status",
        }),
    )?;

    if withdrawal_requests.is_empty() {
        return Err(error::ErrorBadRequest("没有找到已通过的提现申请"));
    }
    let withdrawal_req = &withdrawal_requests[0];

    let out_bill = withdrawal_req.out_bill_no.clone().unwrap_or("".to_string());
    let is_pass = hash_user_withdrawal_money_verify(
        &withdrawal_req.transfer_hash,
        uid,
        withdrawal_req.req_amount,
        &out_bill,
        withdrawal_req.status,
    )?;
    if !is_pass {
        return Err(error::ErrorBadRequest("数据一致性错误，请联系客服"));
    }

    // 获取用户openid
    #[derive(Serialize, Deserialize, Clone)]
    struct UserInfo {
        openid: Option<String>,
        hash: Option<String>,
    }
    let user_info: Vec<UserInfo> = my_run_vec(
        &mut conn,
        myfind!("usr_silent", {
            p0: ["id", "=", uid],
            r: "p0",
            select: "openid, hash",
        }),
    )?;
    if user_info.is_empty() || user_info[0].openid.is_none() {
        return Err(error::ErrorBadRequest("用户openid不存在"));
    }
    let openid = user_info[0].openid.as_ref().unwrap();

    if !hash_user_verify(
        &user_info[0].clone().hash.unwrap_or("".to_string()),
        uid,
        openid,
    )? {
        return Err(error::ErrorBadRequest("用户数据一致性错误，请联系客服"));
    }

    // 生成转账单号
    let out_batch_no = uuid::Uuid::new_v4().simple().to_string();

    // 开启事务
    let mut tran = conn
        .start_transaction(TxOpts::default())
        .map_err(|_| error::ErrorInternalServerError("事务错误"))?;

    // 初始化微信支付
    let wx_pay = wx_pay_init();

    // 构建转账场景报备信息
    let mut transfer_scene_report_infos = Vec::new();
    transfer_scene_report_infos.push(TransferSceneReportInfo {
        info_type: "岗位类型".to_string(),
        info_content: "销售".to_string(),
    });
    transfer_scene_report_infos.push(TransferSceneReportInfo {
        info_type: "报酬说明".to_string(),
        info_content: "销售分成申请提现".to_string(),
    });

    // 构建转账请求
    let transfer_req = Transfer {
        appid: wx_pay.appid.to_string(),
        out_bill_no: out_batch_no.clone(),
        transfer_scene_id: "1005".to_string(), // 转账场景ID
        openid: openid.clone(),
        user_name: None, // 收款用户姓名，可选
        transfer_amount: (withdrawal_req.req_amount * 100.0) as u64, // 转账金额，单位分
        transfer_remark: "销售金额提现".to_string(),
        notify_url: Some(WECHAT_PAY_TRANSFER_NOTIFY_URL.to_string()),
        user_recv_perception: Some("劳务报酬".to_string()),
        transfer_scene_report_infos,
    };

    // 调用微信转账接口
    match wx_pay.transfer(&transfer_req).await {
        Ok(transfer_result) => {
            let hash = hash_user_withdrawal_money(
                uid,
                withdrawal_req.req_amount,
                &out_batch_no,
                WithdrawalReqStatus::Ing as u8,
            )?;
            // 更新提现申请记录
            if let Err(e) = my_run_tran_drop(
                &mut tran,
                myupdate!("usr_withdrawal_request", withdrawal_req.id, {
                    "out_bill_no": &transfer_result.out_bill_no ,
                    "transfer_bill_no": &transfer_result.transfer_bill_no ,
                    "status": WithdrawalReqStatus::Ing as u8,
                    "transfer_hash": hash,
                }),
            ) {
                tran.rollback().unwrap();
                return Err(e);
            }
            // 提交事务
            tran.commit().unwrap();
            Ok(web::Json(Res::success(WithdrawalRequestInfo {
                appid: WECHAT_MINI_APP_ID.to_string(),
                mchid: WECHAT_PAY_MCH_ID.to_string(),
                package: transfer_result.package_info.unwrap_or("".to_string()),
            })))
        }
        Err(e) => {
            tran.rollback().unwrap();
            Err(error::ErrorInternalServerError(format!("转账失败：{}", e)))
        }
    }
}
