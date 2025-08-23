use actix_web::{Responder, Result, error, get, post, web};
use mysql_quick::{myfind, myset, mysetmany};
use serde::{Deserialize, Serialize};
use serde_aux::prelude::{deserialize_bool_from_anything, deserialize_string_from_number};
use serde_json::Value;
use utoipa::ToSchema;

use crate::common::types::QuestionFormType;
use crate::control::frequency::freq_user_day;
use crate::routes::Res;
use crate::{
    db::{my_run_drop, my_run_vec, mysql_conn},
    middleware::AuthUser,
    utils::utils::log_err,
};

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct QueFormItem {
    /// id
    id: u32,
    /// alias name
    alias: String,
    /// 标题名
    title: String,
    /// 说明的内容
    note: Option<String>,
    /// 预提示
    placeholder: Option<String>,
    /// 用户没有填时，的提示语
    prompt: Option<String>,
    /// 是否必填： 1 表示必填，0 表示非必填
    required: bool,
    /// 类型
    que_type: QuestionFormType,
    /// 是否禁止编辑
    disable: bool,
    /// 监听其他组件id，值的变化
    listening_id: Option<u32>,
}
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct QueForm {
    /// id
    id: u32,
    /// 问卷表单标题
    title: String,
    /// 问卷表单唯一名
    form_name: String,
    /// 提示，描述
    tips: Option<String>,
    /// 题目列表
    que_items: Vec<QueFormItem>,
    /// 备注内容
    remark: Option<String>,
    /// 提交成功后提示内容
    submit_prompts: Option<String>,
    /// json 控制内容
    json: Option<Value>,
}
/// 【表单】获取单个表单
#[utoipa::path(
    responses((status = 200, description = "【返回：QueForm[]】", body = Res<QueForm>)),
)]
#[get("/que_form/detail/{id}")]
pub async fn que_form_detail(path: web::Path<String>) -> Result<impl Responder> {
    let id = path
        .parse::<i8>()
        .map_err(|_| error::ErrorBadRequest("id参数错误"))?;

    let mut conn = mysql_conn()?;

    #[derive(Serialize, Deserialize)]
    struct QueFormGet {
        id: u32,
        title: String,
        form_name: String,
        tips: Option<String>,
        que_item_ids: String,
        remark: Option<String>,
        submit_prompts: Option<String>,
        json: Option<String>,
    }
    let info: Vec<QueFormGet> = my_run_vec(
        &mut conn,
        myfind!("cqf_que_form", {
            p0: ["status", "=", 2],
            p1: ["is_del", "=", 0],
            p2: ["id", "=", id],
            r: "p0 && p1 && p2",
            select: "id,title,form_name,tips,que_item_ids,remarks,submit_prompts,json",
        }),
    )?;

    if info.is_empty() {
        return Err(error::ErrorNotFound("未找到相关问卷表单"));
    }

    // 通过 que_item_ids 查找所有item
    #[derive(Serialize, Deserialize)]
    struct QueFormItemGet {
        id: u32,
        alias: String,
        title: String,
        note: Option<String>,
        placeholder: Option<String>,
        prompt: Option<String>,
        listening_id: Option<u32>,
        #[serde(deserialize_with = "deserialize_bool_from_anything")]
        required: bool,
        que_type: String,
        #[serde(deserialize_with = "deserialize_bool_from_anything")]
        disable: bool,
    }
    let list: Vec<QueFormItemGet> = my_run_vec(
        &mut conn,
        myfind!("cqf_que_item", {
            p0: ["status", "=", 2],
            p1: ["is_del", "=", 0],
            p2: ["id", "in", &info[0].que_item_ids],
            r: "p0 && p1 && p2",
            select: "id,alias,title,note,placeholder,required,prompt,que_type,listening_id,disable",
        }),
    )?;

    let que_info = QueForm {
        id: info[0].id,
        title: info[0].title.clone(),
        form_name: info[0].form_name.clone(),
        tips: info[0].tips.clone(),
        que_items: list
            .into_iter()
            .map(|x| QueFormItem {
                id: x.id,
                alias: x.alias,
                title: x.title,
                note: x.note,
                placeholder: x.placeholder,
                prompt: x.prompt,
                required: x.required,
                listening_id: x.listening_id,
                que_type: x.que_type.into(),
                disable: x.disable,
            })
            .collect::<Vec<QueFormItem>>(),
        remark: info[0].remark.clone(),
        submit_prompts: info[0].submit_prompts.clone(),
        json: if let Some(json_str) = info[0].json.clone() {
            serde_json::from_str(&json_str).map_err(|e| {
                error::ErrorInternalServerError(log_err(&e, "que_form表单json格式解析错误"))
            })?
        } else {
            None
        },
    };

    Ok(web::Json(Res::success(que_info)))
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct QueFormItemSubmit {
    /// 每个问题（表单项）的 id
    id: u64,
    /// 每个表单项的回答结果 可传 数字，字符，空字符
    #[serde(deserialize_with = "deserialize_string_from_number")]
    value: String,
}
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct QueFormSubmit {
    /// 问卷表单的id
    que_form_id: u64,
    /// 问题表单的别名
    form_name: String,
    /// 每个表单项的结果
    ans_list: Vec<QueFormItemSubmit>,
}
/// 【表单】提交
#[utoipa::path(
    request_body = QueFormSubmit,
    responses((status = 200, description = "【请求：QueFormSubmit】【返回：String】", body = Res<u8>))
)]
#[post("/que_form/submit")]
pub async fn que_form_submit(
    user: AuthUser,
    params: web::Json<QueFormSubmit>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let uid = user.id;

    // 每天每个人，每个调查，最多3次
    freq_user_day(
        uid,
        &format!("que_form_submit_{}", params.que_form_id),
        30000,
    )?;

    let afid = my_run_drop(
        &mut conn,
        myset!("cqf_ans_form", {
            "uid": uid,
            "que_form_id": params.que_form_id,
        }),
    )?;

    #[derive(Serialize, Deserialize, Debug)]
    struct AnsItemSet {
        ans_form_id: u64,
        que_item_id: u64,
        value: Option<String>,
    }
    let list = params
        .ans_list
        .iter()
        .map(|x| AnsItemSet {
            ans_form_id: afid,
            que_item_id: x.id,
            value: if x.value.is_empty() {
                None
            } else {
                Some(x.value.clone())
            },
        })
        .collect::<Vec<_>>();
    my_run_drop(&mut conn, mysetmany!("cqf_ans_item", list))?;

    Ok(web::Json(Res::<u8>::info(1, "提交成功")))
}
