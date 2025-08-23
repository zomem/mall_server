/// 数据库连接
#[cfg(debug_assertions)]
pub const MYSQL_URL: &str = "mysql://root:12345678@localhost:3306/mall_scaffold";
#[cfg(debug_assertions)]
pub const REDIS_URL: &str = "redis://127.0.0.1/";

// --release 的配置
#[cfg(not(debug_assertions))]
pub const MYSQL_URL: &str = "mysql://xxx:xxxx@localhost:3306/mall_scaffold";
#[cfg(not(debug_assertions))]
pub const REDIS_URL: &str = "redis://:aaa@127.0.0.1/";

/// 超级管理员 的 uid
pub const SUPER_SYSTEM_USER_ID: u64 = 1;

/// jwt
pub const JWT_TOKEN_SECRET: &str = "Tvy5fzF8PhX0r0ZWB9RxDK2OwIkpBrlI";

/// /utils/crypto/ aes key
pub const LOCAL_AES_256_KEY: &str = "l8ljjHKOUPJGvbqpbejHS6NTCmhPTj3T";
#[allow(unused)]
pub enum LocalKeySeed {
    /// 测试
    Test = 1,
    /// 日志
    Logs = 1752,
    /// 核销码
    WriteOffCode = 3473,
    /// 用户零钱
    UserPocketMoney = 3600,
    /// 用户交易记录
    UserTranRecord = 6355,
    /// 邀请销售码
    InviteSaleCode = 7686,
    /// 邀请用户码
    InviteUserCode = 7986,
    /// 文件链接
    FileLink = 9677,
}

/// 微信应用配置参数
pub const WECHAT_MINI_APP_ID: &str = "wx2dda4c7xxx";
pub const WECHAT_MINI_APP_SECRET: &str = "55a2594fxxx";

/// 微信公众号 配置信息
pub const WECHAT_GZH_APP_ID: &str = "wx461a0xxx";
pub const WECHAT_GZH_APP_SECRET: &str = "e91de08dafcdb737xxxx";

pub const WECHAT_PAY_MCH_ID: &str = "15xxxx";
// pub const WECHAT_PAY_APIV2: &str = "xfahDJfLkxxxx";
pub const WECHAT_PAY_APIV3: &str = "ZwAklzmcTDfxxxx";
pub const WECHAT_PAY_SERIAL: &str = "4BF9AECC834xxxx";

/// 微信支付 v3 密钥
pub const WECHAT_PRIVATE_KEY: &str = "-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQC5n06KVhinQj1z
t4hnBlOs5/KKp7cbJ5p8/vn8SZV3vfkoUJeNnODgafBMaReBJmPBhuZHmK8jqTIh
guqnyf22ZqIdjgIuiHpf+KMmQXRGY5sk8e1EP0W/xlKEZpkkJ2f/qoecl1TE6qS5
CGFr5L+Vb041rCuzLKFQ6NgxSxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
XwaxdkX02Q+mItsYXQr5Fg==
-----END PRIVATE KEY-----";

// oss 配置
pub const OSS_ACCESS_KEY_ID: &str = "LTAI5tJxxxx";
pub const OSS_ACCESS_KEY_SECRET: &str = "jwMIlKbFxxxx";
pub const OSS_END_POINT: &str = "oss-cn-hangzhou.aliyuncs.com"; // oss-cn-hangzhou.aliyuncs.com   oss-cn-hangzhou-internal.aliyuncs.com

// 阿里短信
pub const SMS_ACCESS_KEY_ID: &str = "LTAI5tFm3Zqjexxxxxxx";
pub const SMS_ACCESS_KEY_SECRET: &str = "HKRGq8lw7oxxxxxxx";
pub const SMS_SIGN_NAME: &str = "科技";
pub const SMS_TEMPLATE_CODE: &str = "SMS_xxxxx";

// 高德地图 web 服务key
pub const AMAP_WEB_SERVER_KEY: &str = "fd3f456bd4f191axxxxxxx";
pub const AMAP_WEB_URL: &str = "https://restapi.amap.com/v3";

// 邮箱
pub const EMAIL_HOST: &str = "smtp.xxxx.com";
pub const EMAIL_USERNAME: &str = "187xxxxx";
pub const EMAIL_PASSWORD: &str = "XQR4TTCxxxxx";
