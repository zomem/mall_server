use actix_jobs::{run_forever, Job, Scheduler};

// TODO 用户优惠券，过期状态的定时任务
// TODO 用户立即购买，但 一直没去结算，的购物车状态修改，且要把对应数量的商品返回 30分钟一次？

/// 清理超过半年的，用户的检测数据。
#[allow(unused)]
struct SomeJob;

impl Job for SomeJob {
    fn cron(&self) -> &str {
        "*/5 * * * * * *"
    }
    fn run(&mut self) {
        println!("jobs...");
    }
}

/// 执行job操作
#[allow(unused)]
pub fn run_jobs() {
    let mut scheduler = Scheduler::new();
    scheduler.add(Box::new(SomeJob));

    run_forever(scheduler);
}
