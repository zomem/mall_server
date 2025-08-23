use actix_web::{Responder, Result, error, get, web};
use mysql_quick::myfind;
use serde::{Deserialize, Serialize};
use serde_aux::prelude::deserialize_bool_from_anything;
use utoipa::ToSchema;

use crate::common::types::QuestionFormType;
use crate::db::{my_run_vec, mysql_conn};
use crate::middleware::AuthMana;
use crate::routes::Res;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct QueFormItemRes {
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
pub struct QueFormRes {
    /// id
    id: u32,
    /// 问卷表单标题
    title: String,
    /// 问卷表单唯一名
    form_name: String,
    /// 提示，描述
    tips: Option<String>,
    /// 题目列表
    que_items: Vec<QueFormItemRes>,
    /// 备注内容
    remark: Option<String>,
    /// 提交成功后提示内容
    submit_prompts: Option<String>,
    /// json 控制内容
    json: Option<String>,
}
/// 【表单】表单列表
#[utoipa::path(
    responses((status = 200, description = "【返回：QueFormRes[]】", body = Res<PageData<QueFormRes>>)),
    params(("page", description="页码"), ("limit", description="每页数量"))
)]
#[get("/manage/que_form/que/list/{page}/{limit}")]
pub async fn manage_que_form_que_list(
    _mana: AuthMana,
    path: web::Path<(u32, u32)>,
) -> Result<impl Responder> {
    let (page, limit) = path.to_owned();

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
            p1: ["is_del", "=", 0],
            r: "p1",
            page: page,
            limit: limit,
            order_by: "-created_at",
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
            p1: ["is_del", "=", 0],
            r: "p1",
            select: "id,alias,title,note,placeholder,required,prompt,que_type,listening_id,disable",
        }),
    )?;

    let que_info = info
        .into_iter()
        .map(|a| {
            let item_ids = a
                .que_item_ids
                .split(",")
                .map(|x| x.to_string())
                .collect::<Vec<String>>();
            QueFormRes {
                id: a.id,
                title: a.title,
                form_name: a.form_name,
                tips: a.tips,
                que_items: list
                    .iter()
                    .filter(|x| {
                        if item_ids.contains(&x.id.to_string()) {
                            true
                        } else {
                            false
                        }
                    })
                    .map(|x| QueFormItemRes {
                        id: x.id,
                        alias: x.alias.clone(),
                        title: x.title.clone(),
                        note: x.note.clone(),
                        placeholder: x.placeholder.clone(),
                        prompt: x.prompt.clone(),
                        required: x.required,
                        listening_id: x.listening_id,
                        que_type: x.que_type.clone().into(),
                        disable: x.disable,
                    })
                    .collect::<Vec<QueFormItemRes>>(),
                remark: a.remark,
                submit_prompts: a.submit_prompts,
                json: a.json,
            }
        })
        .collect::<Vec<_>>();

    Ok(web::Json(Res::success(que_info)))
}
