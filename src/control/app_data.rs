use snowflaker::generator::{Generator, SnowflakeGenerator};
use std::time::{SystemTime, UNIX_EPOCH};

/// 雪花随机的center
const SLOWN_CENTER_ID: u64 = 1;
/// 雪花随机的 worker
pub enum SlownWorker {
    OrderSn = 1,
    OutTradeNo,
    OrderItemId,
    DeliveryCode,
    OssFileName,
}

/// 全局变量，获取雪花随机数
pub struct AppData {
    order_sn: SnowflakeGenerator,
    out_trade_no: SnowflakeGenerator,
    order_item_id: SnowflakeGenerator,
    delivery_code: SnowflakeGenerator,
    oss_file_name: SnowflakeGenerator,
}
impl AppData {
    pub fn new() -> Self {
        Self {
            order_sn: SnowflakeGenerator::new(SLOWN_CENTER_ID, SlownWorker::OrderSn as u64)
                .unwrap(),
            out_trade_no: SnowflakeGenerator::new(SLOWN_CENTER_ID, SlownWorker::OutTradeNo as u64)
                .unwrap(),
            order_item_id: SnowflakeGenerator::new(
                SLOWN_CENTER_ID,
                SlownWorker::OrderItemId as u64,
            )
            .unwrap(),
            delivery_code: SnowflakeGenerator::new(
                SLOWN_CENTER_ID,
                SlownWorker::DeliveryCode as u64,
            )
            .unwrap(),
            oss_file_name: SnowflakeGenerator::new(
                SLOWN_CENTER_ID,
                SlownWorker::OssFileName as u64,
            )
            .unwrap(),
        }
    }
    /// 获取雪花随机数字
    pub fn rand_no(&self, worker: SlownWorker) -> String {
        match worker {
            SlownWorker::OrderSn => self.order_sn.next_id().unwrap().to_string(),
            SlownWorker::OutTradeNo => self.out_trade_no.next_id().unwrap().to_string(),
            SlownWorker::OrderItemId => self.order_item_id.next_id().unwrap().to_string(),
            SlownWorker::DeliveryCode => self.delivery_code.next_id().unwrap().to_string(),
            SlownWorker::OssFileName => self.oss_file_name.next_id().unwrap().to_string(),
        }
    }
    /// 获取时间+雪花随机 字符 标记+雪花随机 字符
    pub fn rand_id(&self, worker: SlownWorker) -> String {
        match worker {
            SlownWorker::OrderSn => {
                let time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let num = self.order_sn.next_id().unwrap();
                format!("{:x}{:x}", time, num)
            }
            SlownWorker::OutTradeNo => {
                let time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let num = self.out_trade_no.next_id().unwrap();
                format!("{:x}{:x}", time, num)
            }
            SlownWorker::OrderItemId => {
                let time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let num = self.order_item_id.next_id().unwrap();
                format!("{:x}{:x}", time, num)
            }
            SlownWorker::DeliveryCode => {
                let symbol = "D";
                let num = self.delivery_code.next_id().unwrap();
                format!("{}{}", symbol, num)
            }
            SlownWorker::OssFileName => {
                let time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let num = self.oss_file_name.next_id().unwrap();
                format!("{:x}{:x}", time, num)
            }
        }
    }
}
