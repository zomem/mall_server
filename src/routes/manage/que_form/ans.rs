use actix_web::{Responder, Result, get, web};
use mysql_quick::myfind;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::types::QuestionFormType;
use crate::db::{my_run_vec, mysql_conn};
use crate::middleware::AuthMana;
use crate::routes::{BrandInfo, PageData, Res};
use crate::utils::files::{get_file_url, get_file_urls};

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct AnsItemRes {
    id: u32,
    /// 表单问题
    title: String,
    /// 用户回答
    value: Option<String>,
    /// 问题类型
    que_type: QuestionFormType,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct AnsFormRes {
    /// id
    id: u32,
    /// 用户id
    uid: u64,
    /// 表单id
    que_form_id: u32,
    /// 用户每个问题回答的列表
    ans_items: Vec<AnsItemRes>,
}
/// 【表单】表单回答列表
#[utoipa::path(
    responses((status = 200, description = "【返回：BrandInfo[]】", body = Res<PageData<BrandInfo>>)),
    params(("page", description="页码"), ("limit", description="每页数量"))
)]
#[get("/manage/que_form/ans/list/{form_id}/{page}/{limit}")]
pub async fn manage_que_form_ans_list(
    _mana: AuthMana,
    path: web::Path<(u32, u32, u32)>,
) -> Result<impl Responder> {
    let (form_id, page, limit) = path.to_owned();

    let mut conn = mysql_conn()?;

    #[derive(Deserialize, Debug)]
    struct AnsFormGet {
        id: u32,
        uid: u64,
        que_form_id: u32,
    }
    // 通过 form_id 查询答案列表
    let ans_form_list: Vec<AnsFormGet> = my_run_vec(
        &mut conn,
        myfind!("cqf_ans_form", {
            p0: ["is_del", "=", 0],
            p1: ["que_form_id", "=", form_id],
            r: "p0 && p1",
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "id,uid,que_form_id",
        }),
    )?;

    let all_ansids = ans_form_list
        .iter()
        .map(|ans| ans.id.to_string())
        .collect::<Vec<String>>()
        .join(",");

    #[derive(Deserialize, Debug)]
    struct AnsItemGet {
        id: u32,
        ans_form_id: u32,
        title: String,
        value: Option<String>,
        que_type: String,
    }
    // 通过 form_id 查询答案列表
    let ans_item_list: Vec<AnsItemGet> = my_run_vec(
        &mut conn,
        myfind!("cqf_ans_item", {
            j0: ["que_item_id", "inner", "cqf_que_item.id"],
            p0: ["is_del", "=", 0],
            p1: ["ans_form_id", "in", all_ansids],
            r: "p0 && p1",
            select: "id,ans_form_id,que_item_id,value,cqf_que_item.title,cqf_que_item.que_type",
        }),
    )?;

    let list = ans_form_list
        .into_iter()
        .map(|x| {
            let mut items: Vec<AnsItemRes> = Vec::new();
            for item in ans_item_list.iter().filter(|i| i.ans_form_id == x.id) {
                let q_type: QuestionFormType = item.que_type.clone().into();
                let value_info;
                if q_type == QuestionFormType::ImageSingle {
                    value_info = get_file_url(item.value.clone());
                } else if q_type == QuestionFormType::ImageMultiple {
                    value_info = Some(get_file_urls(item.value.clone()).join(","));
                } else {
                    value_info = item.value.clone();
                }
                items.push(AnsItemRes {
                    id: item.id,
                    title: item.title.clone(),
                    value: value_info,
                    que_type: item.que_type.clone().into(),
                });
            }
            AnsFormRes {
                id: x.id,
                uid: x.uid,
                que_form_id: x.que_form_id,
                ans_items: items,
            }
        })
        .collect::<Vec<AnsFormRes>>();

    Ok(web::Json(Res::success(list)))
}
