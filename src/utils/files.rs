use actix_web::{Error, error};
use aliyun_oss_rust_sdk::{oss::OSS, request::RequestBuilder, url::UrlApi};
use chrono::{Duration, Local};
use redis::Commands;

use crate::common::types::OssBucket;
use crate::common::{
    FILE_STORAGE_TYPE, FILE_URL_PASS_SEC, LocalKeySeed, OSS_ACCESS_KEY_ID, OSS_ACCESS_KEY_SECRET,
    OSS_END_POINT, PROJECT_NAME,
};
use crate::db::redis_conn;
use crate::utils::crypto::aes_256_encrypt;

/// 将 oss 或本地 文件的url ，提取 path
pub fn get_path_from_url<T: AsRef<str>>(url: &T, bucket: &OssBucket) -> String {
    let base_url = bucket.get_base_url();
    let path_all = url.as_ref().replace(&format!("{base_url}/"), "");
    let path = path_all.split("?").collect::<Vec<&str>>();
    path[0].to_string()
}

/// 将 oss 或本地 文件的urls ，提取 path
pub fn get_path_from_urls<T: AsRef<str>>(urls: &Vec<T>, bucket: &OssBucket) -> Vec<String> {
    let base_url = bucket.get_base_url();
    let paths = urls
        .into_iter()
        .map(|x| {
            let path_all = x.as_ref().replace(&format!("{base_url}/"), "");
            let path = path_all.split("?").collect::<Vec<&str>>();
            path[0].to_string()
        })
        .collect::<Vec<String>>();
    paths
}

/// 文件保存在本地
pub fn put_local_file(buf: &[u8], dir_path: &str, file_path: &str) -> anyhow::Result<(), Error> {
    let save_dir = "static/".to_string() + dir_path;
    let path = std::path::Path::new(&save_dir);
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| error::ErrorInternalServerError(e))?;
    }
    let save_path = "static/".to_string() + file_path;
    std::fs::write(&save_path, &buf).map_err(|e| error::ErrorInternalServerError(e))?;
    Ok(())
}

/// oss 文件上传
pub async fn put_oss_file(
    buf: &[u8],
    file_path: &str,
    bucket: OssBucket,
) -> anyhow::Result<(), Error> {
    let oss = OSS::new(
        OSS_ACCESS_KEY_ID,
        OSS_ACCESS_KEY_SECRET,
        OSS_END_POINT,
        bucket.get_name(),
    );
    let builder = RequestBuilder::new();
    match oss.pub_object_from_buffer(file_path, buf, builder).await {
        Ok(_) => Ok(()),
        Err(e) => Err(error::ErrorBadGateway(e)),
    }

    // match AsyncObjectAPI::put_object(
    //     &oss_instance,
    //     buf,
    //     file_path,
    //     None::<HashMap<&str, &str>>,
    //     None,
    // )
    // .await
    // {
    //     Ok(_) => Ok(()),
    //     Err(e) => Err(e),
    // }
}

/// oss 或本地文件 获取文件链接 用户头像的url 时间和jwt时间一样
pub fn get_file_url_sec<T: AsRef<str>>(file_path: Option<T>, sec: i64) -> Option<String> {
    let bucket = OssBucket::EobFiles;
    let mut url_str = String::new();
    if let Some(p) = file_path {
        let p = p.as_ref();
        if !p.is_empty() {
            let mut redis_con = redis_conn().unwrap();
            let key_name = PROJECT_NAME.to_string()
                + ":"
                + bucket.get_name()
                + ":SEC:"
                + &sec.to_string()
                + ":"
                + &bucket.get_base_url()
                + "/"
                + p;
            match redis_con.get(&key_name) {
                Ok(u) => url_str = u,
                Err(_) => {
                    if FILE_STORAGE_TYPE == 1 {
                        // 本地
                        let expiration = Local::now() + Duration::seconds(sec);
                        let time_stamp = expiration.timestamp();
                        match sign_local_file(&p, time_stamp) {
                            Ok(sign_info) => {
                                let url = format!(
                                    "{}/{}?expires={}&signature={}",
                                    bucket.get_base_url(),
                                    p,
                                    time_stamp,
                                    sign_info
                                );
                                let _: () =
                                    redis_con.set_ex(&key_name, &url, sec as u64 - 5).unwrap();
                                url_str = url;
                            }
                            Err(_) => (),
                        };
                    } else {
                        let oss = OSS::new(
                            OSS_ACCESS_KEY_ID,
                            OSS_ACCESS_KEY_SECRET,
                            OSS_END_POINT,
                            bucket.get_name(),
                        );
                        let build = RequestBuilder::new().with_expire(sec);
                        //.with_cdn("https://mydomain.com") //使用cdn后，无法限制ip访问
                        // .oss_download_speed_limit(30);
                        let download_url = oss.sign_download_url(p, &build);
                        let list = download_url.split("?").collect::<Vec<&str>>();
                        let url = format!("{}/{}?{}", bucket.get_base_url(), p, list[1]);
                        let _: () = redis_con.set_ex(&key_name, &url, sec as u64 - 5).unwrap();
                        url_str = url;
                    }
                }
            }
        }
    }
    if url_str.is_empty() {
        None
    } else {
        Some(url_str)
    }
}

/// oss 或本地 获取文件链接 Some("dev/img.jpg")  None | "" 时返回 None
/// 返回完整的url
pub fn get_file_url<T: AsRef<str>>(file_path: Option<T>) -> Option<String> {
    let bucket = OssBucket::EobFiles;
    let mut url_str = String::new();
    if let Some(p) = file_path {
        let p = p.as_ref();
        if !p.is_empty() {
            let mut redis_con = redis_conn().unwrap();
            let key_name = PROJECT_NAME.to_string()
                + ":"
                + bucket.get_name()
                + ":"
                + &bucket.get_base_url()
                + "/"
                + p;
            match redis_con.get(&key_name) {
                Ok(u) => url_str = u,
                Err(_) => {
                    if FILE_STORAGE_TYPE == 1 {
                        // 本地
                        let expiration = Local::now() + Duration::seconds(FILE_URL_PASS_SEC);
                        let time_stamp = expiration.timestamp();
                        match sign_local_file(&p, time_stamp) {
                            Ok(sign_info) => {
                                let url = format!(
                                    "{}/{}?expires={}&signature={}",
                                    bucket.get_base_url(),
                                    p,
                                    time_stamp,
                                    sign_info
                                );
                                let _: () = redis_con
                                    .set_ex(&key_name, &url, FILE_URL_PASS_SEC as u64 - 5)
                                    .unwrap();
                                url_str = url;
                            }
                            Err(_) => (),
                        }
                    } else {
                        let oss = OSS::new(
                            OSS_ACCESS_KEY_ID,
                            OSS_ACCESS_KEY_SECRET,
                            OSS_END_POINT,
                            bucket.get_name(),
                        );
                        let build = RequestBuilder::new().with_expire(FILE_URL_PASS_SEC);
                        //.with_cdn("https://mydomain.com") //使用cdn后，无法限制ip访问
                        // .oss_download_speed_limit(30);
                        let download_url = oss.sign_download_url(p, &build);
                        let list = download_url.split("?").collect::<Vec<&str>>();
                        let url = format!("{}/{}?{}", bucket.get_base_url(), p, list[1]);
                        let _: () = redis_con
                            .set_ex(&key_name, &url, FILE_URL_PASS_SEC as u64 - 5)
                            .unwrap();
                        url_str = url;
                    }
                }
            }
        }
    }
    if url_str.is_empty() {
        None
    } else {
        Some(url_str)
    }
}

/// oss 或本地 批量获取文件链接，Some("a/a.jpg,a/b.jpg") None | "" 时返回 vec![]
/// 返回完整的url
pub fn get_file_urls<T: AsRef<str>>(file_paths: Option<T>) -> Vec<String> {
    let mut list: Vec<String> = vec![];
    if let Some(p) = file_paths {
        let p = p.as_ref();
        if !p.is_empty() {
            list = p
                .split(",")
                .filter(|x| !x.is_empty())
                .map(|x| get_file_url(Some(x)).unwrap_or("".to_string()))
                .collect::<Vec<String>>();
        }
    }
    list
}

// oss 删除
// pub async fn del_oss_file(file_path: &str) -> Result<(), Error> {
//     let oss_instance = OSS::new(
//         OSS_ACCESS_KEY_ID,
//         OSS_ACCESS_KEY_SECRET,
//         OSS_END_POINT,
//         OSS_BUCKET_NAME,
//     );

//     match AsyncObjectAPI::delete_object(&oss_instance, file_path).await {
//         Ok(_) => Ok(()),
//         Err(e) => Err(e),
//     }
// }

// /// 本地，删除指定路径的文件  如： /static/images/a.jpg
// pub fn del_file_one(url: &str) {
//     let b_s = url.to_string();
//     let file_path = &b_s.to_string()[1..b_s.len()];
//     let path = std::path::Path::new(file_path);
//     if path.exists() {
//         std::fs::remove_file(path).unwrap();
//     }
// }

// /// 本地，删除多个文件 '/static/images/a.jpg,/static/images/b.txt'
// pub fn del_file_many(urls: &str) {
//     for b in urls.split(",") {
//         let b_s = b.to_string();
//         let file_path = &b_s.to_string()[1..b_s.len()];
//         let path = std::path::Path::new(file_path);
//         if path.exists() {
//             std::fs::remove_file(path).unwrap();
//         }
//     }
// }

/// 通过url, 下载文件，并将文件存储到oss
pub async fn download_file_to_oss(
    url: &str,
    dir_path: &str,
    file_path: &str,
    bucket: OssBucket,
) -> anyhow::Result<(), Error> {
    let response = match reqwest::get(url).await {
        Ok(r) => r,
        Err(e) => return Err(error::ErrorBadGateway(e)),
    };
    let buffer = response
        .bytes()
        .await
        .map_err(|e| error::ErrorBadGateway(e))?
        .to_vec();
    let buf = buffer.as_slice();
    if FILE_STORAGE_TYPE == 1 {
        put_local_file(&buf, &dir_path, &file_path)?;
    } else {
        put_oss_file(buf, file_path, bucket).await?;
    }
    Ok(())
}

/// 本地文件进行签名
pub(crate) fn sign_local_file(file_path: &str, expiration: i64) -> anyhow::Result<String, Error> {
    let endata = format!("{}T{}", &file_path, expiration);
    let sign_info = aes_256_encrypt(&endata, LocalKeySeed::FileLink)
        .map_err(|_| error::ErrorBadRequest("文件签名错误"))?;
    Ok(sign_info)
}

#[cfg(test)]
mod test {
    use super::download_file_to_oss;
    #[tokio::test]
    async fn test_download_file_to_oss() {
        let url = "https://thirdwx.qlogo.cn/mmopen/g3MonUZtNHkdmzicIlibx6iaFqAc56vxLSUfpb6n5WKSYVY0ChQKkiaJSgQ1dZuTOgvLLrhJbERQQ4eMsv84eavHiaiceqxibJxCfHe/132";
        download_file_to_oss(
            url,
            "static/dev",
            "static/dev/test.jpeg",
            crate::common::types::OssBucket::EobFiles,
        )
        .await
        .unwrap();
    }
}
