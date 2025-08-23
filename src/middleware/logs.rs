use std::fs::{OpenOptions, create_dir_all};
use std::io::prelude::*;
use std::path::Path;

use actix_web::Error;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use tracing::Span;
use tracing_actix_web::{DefaultRootSpanBuilder, RootSpanBuilder};

use crate::common::LOG_LEVEL_STATUS;
use crate::middleware::get_client_ip;
use crate::utils::jwt::uid_token;
use crate::utils::time::{NowTimeType, get_now_time};

/// 日志，手动记录  path:  dir/save/path/logname.log
pub fn save_logs(path: &str, content: &str) -> () {
    let target_path = Path::new(path);
    if target_path.exists() {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        file.write(content.as_bytes()).unwrap();
    } else {
        let mut path_v: Vec<&str> = path.split_terminator("/").collect();
        path_v.pop();
        create_dir_all(path_v.join("/")).unwrap();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        file.write(content.as_bytes()).unwrap();
    }
}

pub struct CustomRootSpanBuilder;
/// 日志，自动记录
impl RootSpanBuilder for CustomRootSpanBuilder {
    fn on_request_start(request: &ServiceRequest) -> Span {
        tracing_actix_web::root_span!(request)
    }
    fn on_request_end<B: MessageBody>(span: Span, outcome: &Result<ServiceResponse<B>, Error>) {
        match outcome {
            Ok(res) => {
                let status = res.response().status();
                let mut ip_info = String::new();
                if let Some(client_ip) = get_client_ip(&res.request()) {
                    ip_info = format!("IP:{}", client_ip.ip());
                }

                if status.as_u16() < LOG_LEVEL_STATUS {
                    return DefaultRootSpanBuilder::on_request_end(span, outcome);
                }
                let method = res.request().method();
                let path = res.request().path();
                let save_path = path.split_terminator("/").collect::<Vec<&str>>();
                let query = res.request().query_string();
                let error = res.response().error();
                let head = &res.request().head().headers;
                let client_id: String;
                let auth = head.get("Authorization");

                if let Some(a) = auth {
                    let a = a.to_str().unwrap();
                    if a == "Bearer " || a == "" || a == "Bearer undefined" {
                        client_id = "用户未登录，无 token".to_owned();
                    } else {
                        let token = a.split("Bearer ").collect::<Vec<&str>>().pop().unwrap();
                        client_id = format!("{}", uid_token(token).id);
                    }
                } else {
                    client_id = "用户未登录，无 authorization".to_owned();
                }

                let str_save = format!(
                    "------- {} {:?} {} -------\n{} {}\nuid: {}\nquery: {:?}\nerror: {:?}\n---------------------------------------\n\n",
                    ip_info,
                    status,
                    get_now_time(NowTimeType::DateTime),
                    method,
                    path,
                    client_id,
                    query,
                    error,
                );
                let file_time_name = get_now_time(NowTimeType::Date).replace("-", "_") + ".log";
                if save_path.len() > 1 {
                    save_logs(
                        (String::from("logs/") + save_path[1] + "/" + file_time_name.as_str())
                            .as_str(),
                        str_save.as_str(),
                    );
                } else {
                    save_logs(
                        (String::from("logs/others_error/") + file_time_name.as_str()).as_str(),
                        str_save.as_str(),
                    );
                }
            }
            Err(err) => {
                let file_time_name = get_now_time(NowTimeType::Date).replace("-", "_") + ".log";
                save_logs(
                    (String::from("logs/system_error/") + file_time_name.as_str()).as_str(),
                    err.to_string().as_str(),
                );
            }
        }

        DefaultRootSpanBuilder::on_request_end(span, outcome);
    }
}
