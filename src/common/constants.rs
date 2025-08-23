/// 项目名称
pub const PROJECT_NAME: &str = "mall_server";

/// 服务器端口
pub const SERVER_PROT: u16 = 3060; //  3060
/// 日志打印级别，小于此值(http status)时不记录，大于等于时，记录
pub const LOG_LEVEL_STATUS: u16 = 400;

/// 微信支付，回调地址
pub const WECHAT_PAY_NOTIFY_URL: &str = "https://dev/pay/notify";
/// 公众号 js sdk 域名
pub const WECHAT_GZH_JS_SDK_URL: &str = "https://";

/// 管理后台，jwt 过期时间 S
pub const JWT_MANAGE_EXPIRES_SEC: i64 = 8 * 3600;
/// 普通用户 jwt 过期时间 S
pub const JWT_NORMAL_EXPIRES_SEC: i64 = 2 * 3600;

/// 文件存储类型，1为本地存储，2为oss存储
pub const FILE_STORAGE_TYPE: i8 = 1;
/// 本地存储，文件路径
#[cfg(debug_assertions)]
pub const STATIC_FILE_URL: &str = "http://localhost:3060/static"; //"http://localhost:3060/static";
#[cfg(not(debug_assertions))]
pub const STATIC_FILE_URL: &str = "http://localhost:3060/static";
/// oss 或 本地 文件url的文件链接过期时间 秒
pub const FILE_URL_PASS_SEC: i64 = 1 * 24 * 3600;

/// 产品起始id
pub const PRODUCT_START_SN: u32 = 100000;
/// 商品起始id
pub const UNIT_START_SN: u32 = 1000000;
/// 店铺起始id
pub const STORE_START_CODE: u32 = 1000;
/// 品牌起始id
pub const BRAND_START_CODE: u32 = 1000;

/// 核销码，二维码，的过期时间
pub const WRITE_OFF_QRCODE_EXPIRES_SEC: i64 = 1800;
