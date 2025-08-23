use actix_web::{
    Error, HttpMessage, HttpRequest, HttpResponse, Result,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{Ready, ready},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    rc::Rc,
};

/// IP 提取器结构体
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ClientIp {
    pub ip: String,
    pub is_proxy: bool,
    pub x_forwarded_for: Option<String>,
    pub x_real_ip: Option<String>,
}

#[allow(unused)]
impl ClientIp {
    pub fn new(ip: String) -> Self {
        Self {
            ip,
            is_proxy: false,
            x_forwarded_for: None,
            x_real_ip: None,
        }
    }

    pub fn with_proxy_info(
        ip: String,
        x_forwarded_for: Option<String>,
        x_real_ip: Option<String>,
    ) -> Self {
        Self {
            ip,
            is_proxy: x_forwarded_for.is_some() || x_real_ip.is_some(),
            x_forwarded_for,
            x_real_ip,
        }
    }

    /// 获取IP地址
    pub fn ip(&self) -> &str {
        &self.ip
    }

    /// 是否通过代理
    pub fn is_behind_proxy(&self) -> bool {
        self.is_proxy
    }

    /// 获取原始的X-Forwarded-For头
    pub fn x_forwarded_for(&self) -> Option<&str> {
        self.x_forwarded_for.as_deref()
    }

    /// 获取X-Real-IP头
    pub fn x_real_ip(&self) -> Option<&str> {
        self.x_real_ip.as_deref()
    }
}

/// 中间件配置
#[derive(Clone)]
pub struct IpExtractorConfig {
    pub trusted_proxies: Vec<IpAddr>,
    pub trust_x_forwarded_for: bool,
    pub trust_x_real_ip: bool,
    pub trust_any_proxy: bool,
}

impl Default for IpExtractorConfig {
    fn default() -> Self {
        Self {
            trusted_proxies: vec![
                "127.0.0.1".parse().unwrap(),
                "::1".parse().unwrap(),
                // 常见的私有网络地址
                "10.0.0.0/8"
                    .parse::<IpAddr>()
                    .unwrap_or("10.0.0.1".parse().unwrap()),
                "172.16.0.0/12"
                    .parse::<IpAddr>()
                    .unwrap_or("172.16.0.1".parse().unwrap()),
                "192.168.0.0/16"
                    .parse::<IpAddr>()
                    .unwrap_or("192.168.0.1".parse().unwrap()),
            ],
            trust_x_forwarded_for: true,
            trust_x_real_ip: true,
            trust_any_proxy: false, // 在生产环境中应该设为 false
        }
    }
}
impl IpExtractorConfig {
    /// 创建适用于 Nginx 反向代理的配置
    pub fn for_nginx() -> Self {
        Self {
            trusted_proxies: vec![
                IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), // localhost
                IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), // ::1
            ],
            trust_x_forwarded_for: true,
            trust_x_real_ip: true,
            trust_any_proxy: false,
        }
    }

    /// 添加可信的代理 IP 地址
    pub fn add_trusted_proxy(mut self, ip: IpAddr) -> Self {
        self.trusted_proxies.push(ip);
        self
    }

    /// 添加可信的代理 IP 地址（从字符串解析）
    pub fn add_trusted_proxy_str(mut self, ip_str: &str) -> Result<Self, std::net::AddrParseError> {
        let ip = ip_str.parse::<IpAddr>()?;
        self.trusted_proxies.push(ip);
        Ok(self)
    }
}

/// 中间件主体
/// ```
/// // 默认配置
/// App::new()
///    .wrap(IpExtractor::new());
///
///
/// // 其他配置
/// let config = IpExtractorConfig {
///     trusted_proxies: vec!["127.0.0.1".parse().unwrap()],
///     trust_x_forwarded_for: true,
///     trust_x_real_ip: true,
///     trust_any_proxy: false,
/// };
/// App::new()
///    .wrap(IpExtractor::with_config(config))
/// ```
///
pub struct IpExtractor {
    config: Rc<IpExtractorConfig>,
}

#[allow(unused)]
impl IpExtractor {
    pub fn new() -> Self {
        Self {
            config: Rc::new(IpExtractorConfig::default()),
        }
    }

    pub fn with_config(config: IpExtractorConfig) -> Self {
        Self {
            config: Rc::new(config),
        }
    }
    /// 创建适用于 Nginx 反向代理的中间件
    pub fn for_nginx() -> Self {
        Self {
            config: Rc::new(IpExtractorConfig::for_nginx()),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for IpExtractor
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = IpExtractorMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(IpExtractorMiddleware {
            service,
            config: self.config.clone(),
        }))
    }
}

pub struct IpExtractorMiddleware<S> {
    service: S,
    config: Rc<IpExtractorConfig>,
}

impl<S, B> Service<ServiceRequest> for IpExtractorMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let config = self.config.clone();

        // 提取IP地址
        let client_ip = extract_client_ip(&req, &config);

        // 将IP信息存储到请求扩展中
        req.extensions_mut().insert(client_ip);

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}

// IP提取逻辑
fn extract_client_ip(req: &ServiceRequest, config: &IpExtractorConfig) -> ClientIp {
    let headers = req.headers();

    // 获取X-Forwarded-For头
    let x_forwarded_for = if config.trust_x_forwarded_for {
        headers
            .get("X-Forwarded-For")
            .or_else(|| headers.get("x-forwarded-for")) // 处理小写的情况
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    } else {
        None
    };

    // 获取X-Real-IP头
    let x_real_ip = if config.trust_x_real_ip {
        headers
            .get("X-Real-IP")
            .or_else(|| headers.get("x-real-ip")) // 处理小写的情况
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    } else {
        None
    };

    // 获取连接信息
    let connection_info = req.connection_info();
    let peer_ip = connection_info.peer_addr().unwrap_or("unknown");

    // 确定最终IP地址
    let final_ip = if config.trust_any_proxy {
        // 如果信任任何代理，按优先级选择
        extract_first_valid_ip(&x_forwarded_for)
            .or_else(|| x_real_ip.as_deref())
            .unwrap_or(peer_ip)
            .to_string()
    } else {
        // 只有在来自可信代理时才使用代理头
        if is_trusted_proxy(peer_ip, &config.trusted_proxies) {
            extract_first_valid_ip(&x_forwarded_for)
                .or_else(|| x_real_ip.as_deref())
                .unwrap_or(peer_ip)
                .to_string()
        } else {
            peer_ip.to_string()
        }
    };

    ClientIp::with_proxy_info(final_ip, x_forwarded_for, x_real_ip)
}

/// 从 X-Forwarded-For 头中提取第一个有效的 IP 地址
fn extract_first_valid_ip(x_forwarded_for: &Option<String>) -> Option<&str> {
    x_forwarded_for.as_deref().and_then(|xff| {
        xff.split(',')
            .map(|s| s.trim())
            .find(|ip| !ip.is_empty() && ip.parse::<IpAddr>().is_ok())
    })
}

/// 检查是否为可信代理
fn is_trusted_proxy(ip_str: &str, trusted_proxies: &[IpAddr]) -> bool {
    // 先处理端口号的情况，比如 "127.0.0.1:8080"
    let ip_without_port = ip_str.split(':').next().unwrap_or(ip_str);

    if let Ok(ip) = ip_without_port.parse::<IpAddr>() {
        trusted_proxies.contains(&ip)
    } else {
        false
    }
}

/// 便捷的提取器函数
pub fn get_client_ip(req: &HttpRequest) -> Option<ClientIp> {
    req.extensions().get::<ClientIp>().cloned()
}

/// 示例处理函数
async fn _show_ip_info(req: HttpRequest) -> Result<HttpResponse> {
    if let Some(client_ip) = get_client_ip(&req) {
        let info = serde_json::json!({
            "ip": client_ip.ip(),
            "is_behind_proxy": client_ip.is_behind_proxy(),
            "x_forwarded_for": client_ip.x_forwarded_for(),
            "x_real_ip": client_ip.x_real_ip(),
        });
        Ok(HttpResponse::Ok().json(info))
    } else {
        Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "无法获取IP地址"
        })))
    }
}

async fn _simple_ip(req: HttpRequest) -> Result<String> {
    if let Some(client_ip) = get_client_ip(&req) {
        Ok(format!("您的IP地址: {}", client_ip.ip()))
    } else {
        Ok("无法获取IP地址".to_string())
    }
}
