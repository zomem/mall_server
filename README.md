# Rust + Actix Web - 商城后端服务

基于 Rust + Actix Web 构建的高性能商城后端服务，提供电商解决方案。

## 项目特性

- **高性能**: 基于 Rust 和 Actix Web 构建，具备出色的并发性能
- **完整功能**: 涵盖商城、用户、订单、支付、管理等完整电商功能
- **微服务架构**: 模块化设计，易于扩展和维护
- **API 文档**: 自动生成的 OpenAPI 文档，支持在线调试
- **多端支持**: 同时支持小程序端和管理端接口
- **安全认证**: JWT 认证体系，支持角色权限管理
- **第三方集成**: 微信支付、短信服务、邮件服务等集成

## 技术栈

### 核心框架
- **Rust**: 系统编程语言，保证内存安全和高性能
- **Actix Web 4.10**: 异步 Web 框架
- **Tokio**: 异步运行时

### 数据存储
- **MySQL**: 主数据库 (mysql_quick)
- **Redis**: 缓存和会话存储

### 认证与安全
- **JWT**: 用户认证 (jsonwebtoken)
- **BCrypt**: 密码加密
- **AES**: 数据加密 (libaes)

### 第三方服务
- **微信支付**: wx_pay 0.2.3
- **短信服务**: 阿里云短信、腾讯云短信
- **邮件服务**: lettre
- **文件存储**: 阿里云 OSS

### 文档与监控
- **OpenAPI**: utoipa + utoipa-scalar
- **日志**: tracing + env_logger
- **跨域**: actix-cors

## 项目结构

```
src/
├── main.rs              # 应用入口
├── common/              # 公共模块
│   ├── constants.rs     # 常量定义
│   ├── types.rs         # 类型定义
│   └── secret/          # 敏感配置
├── db/                  # 数据库连接
│   ├── mysql_conn.rs    # MySQL 连接
│   └── redis_conn.rs    # Redis 连接
├── middleware/          # 中间件
│   ├── auth.rs          # 认证中间件
│   ├── ip.rs            # IP 提取
│   └── logs.rs          # 日志中间件
├── routes/              # 路由处理
│   ├── mall/            # 商城相关
│   ├── user/            # 用户相关
│   ├── manage/          # 管理后台
│   ├── pay/             # 支付相关
│   ├── article/         # 文章系统
│   ├── sales/           # 销售系统
│   └── que_form/        # 问卷表单
├── utils/               # 工具函数
│   ├── jwt.rs           # JWT 工具
│   ├── crypto.rs        # 加密工具
│   ├── files.rs         # 文件处理
│   └── qrcode.rs        # 二维码生成
└── control/             # 第三方服务控制器
    ├── sms.rs           # 短信服务
    ├── email.rs         # 邮件服务
    └── wx_info.rs       # 微信服务
```

## 核心功能

### 用户系统
- ✅ 微信登录 (小程序/公众号)
- ✅ 手机号绑定
- ✅ 用户地址管理
- ✅ 用户收藏
- ✅ 用户反馈
- ✅ 钱包系统
- ✅ 优惠券管理

### 商城系统
- ✅ 商品管理 (多规格、多分类)
- ✅ 品牌管理
- ✅ 店铺管理
- ✅ 购物车
- ✅ 订单管理
- ✅ 优惠券系统
- ✅ 核销功能
- ✅ 商品文件 (数字商品)

### 支付系统
- ✅ 微信支付
- ✅ 支付回调处理
- ✅ 订单状态管理

### 管理后台
- ✅ 系统管理 (角色、权限、菜单)
- ✅ 用户管理
- ✅ 商品管理
- ✅ 订单管理
- ✅ 数据统计
- ✅ 内容管理

### 营销系统
- ✅ 销售员管理
- ✅ 邀请码系统
- ✅ 文章系统
- ✅ 问卷表单

## 环境配置

### 开发环境
1. 安装 Rust (推荐使用 rustup)
2. 安装 MySQL 和 Redis
3. 配置数据库连接信息
4. 导入数据库结构 (`sql/mall_scaffold.sql`)

### 数据库配置
在 `src/common/secret/` 目录下配置数据库连接信息。

## 快速开始

### 开发运行
```bash
# 开发模式运行
cargo run  --features doc

# 监听文件变化自动重启 (需要安装 cargo-watch)
cargo watch -x run  --features doc
```

### 生产构建
```bash
# 本地构建
cargo build --release

# 跨平台构建 (Linux)
cross build --release --target=x86_64-unknown-linux-musl

# 构建包含 API 文档的版本
cross build --release --target=x86_64-unknown-linux-musl --features doc
```

## API 文档

服务启动后，可以通过以下地址访问 API 文档：

- **小程序端 API**: `http://localhost:3060/doc/mini`
- **管理端 API**: `http://localhost:3060/doc/manage`

> 注意: API 文档仅在使用 `--features doc` 编译时可用

## 服务配置

- **默认端口**: 3060
- **跨域配置**: 支持多域名访问
- **文件上传**: 最大 200MB（可配置）
- **静态文件**: `/static/images` 路径访问

## 安全特性

- JWT 认证
- 角色权限管理
- IP 地址提取和记录
- 请求频率控制
- 敏感信息加密存储
- 请求日志记录

## 性能特点

- **内存安全**: Rust 语言特性保证内存安全
- **高并发**: Actix Web 的异步处理能力
- **零拷贝**: 高效的数据处理
- **小内存占用**: 生产环境资源占用低

---

⚡ **高性能 • 安全可靠 • 功能完整**
