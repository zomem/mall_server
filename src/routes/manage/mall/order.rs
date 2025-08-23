use actix_web::{Responder, Result, error, get, post, web};
use mysql_quick::{MysqlQuickCount, TxOpts, mycount, myfind, myget, mysetmany};
use serde::{Deserialize, Serialize};

use crate::common::types::{DeliveryType, OrderItemStatus};
use crate::control::app_data::{AppData, SlownWorker};
use crate::routes::Res;
use crate::routes::utils_set::mall_set::{OrderChange, OrderChangeItems, upd_order_item_status};
use crate::utils::files::get_file_url;
use crate::utils::utils::log_err;
use crate::{PageData, UnitAttrInfo};
use crate::{
    db::{my_run_tran_drop, my_run_tran_vec, my_run_vec, mysql_conn},
    middleware::AuthMana,
};

#[derive(Serialize, Deserialize, Clone)]
struct OrderRes {
    id: u64,
    uid: u64,
    nickname: Option<String>,
    order_sn: String,
    total_amount: f64,
    total_quantity: u32,
    pay_amount: f64,
    reduce_amount: Option<f64>,
    reduce_des: Option<String>,
    notes: Option<String>,
    appointment_time: Option<String>,
    province: Option<String>,
    city: Option<String>,
    area: Option<String>,
    addr_detail: Option<String>,
    contact_user: Option<String>,
    contact_phone: Option<String>,
    status: i8,
    created_at: String,
    delivery_type: DeliveryType,
}
/// 订单列表
#[get("/manage/mall/order/list/{status}/{item}/{page}/{limit}")]
pub async fn manage_mall_order_list(
    _user: AuthMana,
    query: web::Path<(String, String, String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let (status, item, page, limit) = query.to_owned();
    let status: i8 = status.to_owned().parse().unwrap();
    // 当前 item status 为 -1 表示全部子订单
    let item_status: i8 = item.to_owned().parse().unwrap();
    let page: u32 = page.to_owned().parse().unwrap();
    let limit: u32 = limit.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("ord_order", {
            j0: ["order_sn", "right", "ord_order_item.order_sn"],
            p0: ["is_del", "=", 0],
            p1: ["status", "=", status],
            p2: ["ord_order_item.status", "=", item_status],
            r: if status == 2 { "p0 && p1 && p2" } else { "p0 && p1"},
        }),
    )?;

    #[derive(Serialize, Deserialize, Clone)]
    struct OrderGet {
        id: u64,
        uid: u64,
        nickname: Option<String>,
        order_sn: String,
        total_amount: String,
        pay_amount: String,
        reduce_amount: Option<String>,
        reduce_des: Option<String>,
        total_quantity: u32,
        notes: Option<String>,
        appointment_time: Option<String>,
        province: Option<String>,
        city: Option<String>,
        area: Option<String>,
        addr_detail: Option<String>,
        contact_user: Option<String>,
        contact_phone: Option<String>,
        status: i8,
        created_at: String,
        delivery_type: String,
    }
    let list: Vec<OrderGet> = my_run_vec(
        &mut conn,
        myfind!("ord_order", {
            j0: ["uid", "inner", "usr_silent.id"],
            j1: ["order_sn", "right", "ord_order_item.order_sn"],
            p0: ["is_del", "=", 0],
            p1: ["status", "=", status],
            p2: ["ord_order_item.status", "=", item_status],
            r: if item_status == -1 { "p0 && p1" } else { "p0 && p1 && p2"},
            page: page,
            limit: limit,
            order_by: "-created_at",
            select: "
                id,uid,usr_silent.nickname,order_sn,total_amount,reduce_amount,reduce_des,pay_amount,notes,appointment_time,
                total_quantity,province,city,area,addr_detail,contact_user,contact_phone,status,created_at,delivery_type
                ",
        }),
    )?;

    let list: Vec<OrderRes> = list
        .into_iter()
        .map(|x| OrderRes {
            id: x.id,
            uid: x.uid,
            nickname: x.nickname,
            order_sn: x.order_sn,
            total_amount: x.total_amount.parse::<f64>().unwrap(),
            pay_amount: x.pay_amount.parse::<f64>().unwrap(),
            total_quantity: x.total_quantity,
            reduce_amount: if let Some(ra) = x.reduce_amount {
                Some(ra.parse::<f64>().unwrap())
            } else {
                None
            },
            notes: x.notes,
            appointment_time: x.appointment_time,
            reduce_des: x.reduce_des,
            status: x.status,
            created_at: x.created_at,
            province: x.province,
            city: x.city,
            area: x.area,
            addr_detail: x.addr_detail,
            contact_user: x.contact_user,
            contact_phone: x.contact_phone,
            delivery_type: x.delivery_type.into(),
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize, Clone)]
struct OrderItemRes {
    id: u64,
    uid: u64,
    order_item_id: String,
    order_sn: String,
    unit_sn: u32,
    unit_name: Option<String>,
    unit_cover: String,
    unit_attr_info: Vec<UnitAttrInfo>,
    buy_quantity: u32,
    amount: f64,
    product_name: String,
    status: i8,
    created_at: String,
    delivery_code: Option<String>,
    delivery_id: Option<String>,
    waybill_id: Option<String>,
}
/// 通过ordersn 查寻所有子商品订单
#[get("/manage/mall/order/item/list/{order_sn}/{item_status}")]
pub async fn manage_mall_order_item_list(
    _user: AuthMana,
    query: web::Path<(String, String)>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;
    let order_sn_no = query.0.to_owned();
    // 当前 item status 为 -1 表示全部子订单
    let item_status: i8 = query.1.to_owned().parse().unwrap();

    let count: Vec<MysqlQuickCount> = my_run_vec(
        &mut conn,
        mycount!("ord_order_item", {
            p0: ["order_sn", "=", &order_sn_no],
            p1: ["is_del", "=", 0],
            p2: ["status", "=", item_status],
            r: if item_status == -1 { "p0 && p1" } else { "p0 && p1 && p2"},
        }),
    )?;

    #[derive(Serialize, Deserialize, Clone)]
    struct OrderItemGet {
        id: u64,
        uid: u64,
        order_item_id: String,
        order_sn: String,
        unit_sn: u32,
        unit_name: Option<String>,
        unit_cover: Option<String>,
        unit_attr_info: Option<String>,
        buy_quantity: u32,
        amount: String,
        product_name: String,
        status: i8,
        created_at: String,
        delivery_code: Option<String>,
        delivery_id: Option<String>,
        waybill_id: Option<String>,
    }
    let list: Vec<OrderItemGet> = my_run_vec(
        &mut conn,
        myfind!("ord_order_item", {
            j0: ["order_item_id", "left", "ord_delivery_order_item.order_item_id"],
            p0: ["is_del", "=", 0],
            p1: ["order_sn", "=", &order_sn_no],
            p2: ["status", "=", item_status],
            r: if item_status == -1 { "p0 && p1" } else { "p0 && p1 && p2"},
            order_by: "-created_at",
            select: "id,uid,order_item_id,order_sn,unit_name,unit_sn,unit_cover,unit_attr_info,buy_quantity,
                amount,product_name,status,created_at,ord_delivery_order_item.delivery_code,
                ord_delivery_order_item.delivery_id,ord_delivery_order_item.waybill_id
                ",
        }),
    )?;

    let list: Vec<OrderItemRes> = list
        .into_iter()
        .map(|x| OrderItemRes {
            id: x.id,
            uid: x.uid,
            order_item_id: x.order_item_id,
            order_sn: x.order_sn,
            unit_sn: x.unit_sn,
            unit_name: x.unit_name,
            unit_attr_info: if let Some(u) = x.unit_attr_info {
                serde_json::from_str::<Vec<UnitAttrInfo>>(&u).unwrap()
            } else {
                vec![]
            },
            buy_quantity: x.buy_quantity,
            amount: x.amount.parse::<f64>().unwrap(),
            product_name: x.product_name,
            unit_cover: get_file_url(x.unit_cover).unwrap_or("".to_string()),
            status: x.status,
            created_at: x.created_at,
            delivery_code: x.delivery_code,
            delivery_id: x.delivery_id,
            waybill_id: x.waybill_id,
        })
        .collect();

    Ok(web::Json(Res::success(PageData::new(
        count[0].mysql_quick_count,
        list,
    ))))
}

#[derive(Serialize, Deserialize, Clone)]
struct OrderProductRes {
    unit_sn: u32,
    delivery_type: String,
}
#[derive(Serialize, Deserialize, Clone)]
struct OrderProduct {
    unit_sns: Vec<u32>,
}
/// 获取当前订单的产品信息
#[post("/manage/mall/order/product/info")]
pub async fn manage_mall_order_product_info(
    _user: AuthMana,
    params: web::Json<OrderProduct>,
) -> Result<impl Responder> {
    let mut conn = mysql_conn()?;

    let sn = params
        .unit_sns
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>();
    let list: Vec<OrderProductRes> = my_run_vec(
        &mut conn,
        myfind!("sku_unit", {
            p0: ["unit_sn", "in", sn.join(",")],
            r: "p0",
            select: "unit_sn",
        }),
    )?;

    Ok(web::Json(Res::success(list)))
}

#[derive(Serialize, Deserialize, Clone)]
struct OrdersParams {
    waybill_id: Option<String>,
    delivery_id: Option<String>,
    order_item_id: String,
}
#[derive(Serialize, Deserialize, Clone)]
struct DeliveryParams {
    order_sn: String,
    orders: Vec<OrdersParams>,
    // 一个订单下面的商品，只有同一种物流方式
    delivery_type: DeliveryType,
}
/// 后台操手动发货
#[post("/manage/mall/order/do_delivery/start")]
pub async fn manage_mall_order_do_delivery_start(
    _user: AuthMana,
    params: web::Json<DeliveryParams>,
    app_data: web::Data<AppData>,
) -> Result<impl Responder> {
    let data = &app_data;
    let mut conn = mysql_conn()?;
    // ---- 事务开始 ----
    let mut tran = conn.start_transaction(TxOpts::default()).unwrap();

    // 如果当前订单的交易类型，不是手动物流，就报错
    if params.delivery_type != DeliveryType::DoDelivery {
        return Ok(web::Json(Res::fail("当前订单不支持手动发货")));
    }

    // 获取当前订单信息
    #[derive(Serialize, Deserialize, Clone)]
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
    }
    let order_get: Vec<OrderGet> = match my_run_tran_vec(
        &mut tran,
        myget!("ord_order", {"order_sn": &params.order_sn}),
    ) {
        Ok(d) => d,
        Err(_) => {
            tran.rollback().unwrap();
            return Ok(web::Json(Res::fail("没有找到相应订单")));
        }
    };

    // 新增快递信息
    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct DeliverySet {
        uid: u64,
        notes: Option<String>,
        appointment_time: Option<String>,
        receiver_province: Option<String>,
        receiver_city: Option<String>,
        receiver_area: Option<String>,
        receiver_addr_detail: Option<String>,
        receiver_name: Option<String>,
        receiver_phone: Option<String>,
        sender_province: Option<String>,
        sender_city: Option<String>,
        sender_area: Option<String>,
        sender_addr_detail: Option<String>,
        sender_name: Option<String>,
        sender_phone: Option<String>,
        waybill_id: Option<String>,
        delivery_id: Option<String>,
        delivery_code: String,
    }
    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct DeliveryItemSet {
        delivery_code: String,
        waybill_id: Option<String>,
        delivery_id: Option<String>,
        order_item_id: String,
        delivery_type: String,
    }
    let mut delivery: Vec<DeliverySet> = vec![];
    let mut delivery_items: Vec<DeliveryItemSet> = vec![];
    let mut order_items: Vec<OrderChangeItems> = vec![];
    for i in 0..params.orders.len() {
        if params.orders[i].waybill_id.is_none() || params.orders[i].delivery_id.is_none() {
            tran.rollback().unwrap();
            return Ok(web::Json(Res::fail("请填写需要发货的物流信息")));
        }
        let delivery_code = data.rand_id(SlownWorker::DeliveryCode);
        order_items.push(OrderChangeItems {
            order_item_id: params.orders[i].order_item_id.clone(),
            status: OrderItemStatus::WaitTakeDelivery as u8,
        });
        let pos = delivery.iter().position(|x| {
            x.delivery_id == params.orders[i].delivery_id
                && x.waybill_id == params.orders[i].waybill_id
        });
        if let Some(index) = pos {
            delivery_items.push(DeliveryItemSet {
                delivery_code: delivery[index].delivery_code.clone(),
                waybill_id: params.orders[i].waybill_id.clone(),
                delivery_id: params.orders[i].delivery_id.clone(),
                order_item_id: params.orders[i].order_item_id.clone(),
                delivery_type: params.delivery_type.to_string(),
            });
        } else {
            delivery.push(DeliverySet {
                delivery_code: delivery_code.clone(),
                uid: order_get[0].uid,
                notes: order_get[0].notes.clone(),
                appointment_time: order_get[0].appointment_time.clone(),
                receiver_province: order_get[0].province.clone(),
                receiver_city: order_get[0].city.clone(),
                receiver_area: order_get[0].area.clone(),
                receiver_addr_detail: order_get[0].addr_detail.clone(),
                receiver_name: order_get[0].contact_user.clone(),
                receiver_phone: order_get[0].contact_phone.clone(),
                sender_province: None,
                sender_city: None,
                sender_area: None,
                sender_addr_detail: None,
                sender_name: None,
                sender_phone: None,
                waybill_id: params.orders[i].waybill_id.clone(),
                delivery_id: params.orders[i].delivery_id.clone(),
            });
            delivery_items.push(DeliveryItemSet {
                delivery_code: delivery_code.clone(),
                waybill_id: params.orders[i].waybill_id.clone(),
                delivery_id: params.orders[i].delivery_id.clone(),
                order_item_id: params.orders[i].order_item_id.clone(),
                delivery_type: params.delivery_type.to_string(),
            });
        }
    }

    // 新增快递
    match my_run_tran_drop(&mut tran, mysetmany!("ord_delivery", delivery)) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };
    match my_run_tran_drop(
        &mut tran,
        mysetmany!("ord_delivery_order_item", delivery_items),
    ) {
        Ok(d) => d,
        Err(e) => {
            tran.rollback().unwrap();
            return Err(e);
        }
    };

    let orders = OrderChange {
        order_sn: params.order_sn.clone(),
        order_items,
    };
    // 修改订单的状态
    for item in orders.order_items {
        match upd_order_item_status(&mut tran, &item.order_item_id, item.status.into()) {
            Ok(d) => d,
            Err(e) => {
                tran.rollback().unwrap();
                return Err(e);
            }
        };
    }
    tran.commit().unwrap();

    Ok(web::Json(Res::success("")))
}
