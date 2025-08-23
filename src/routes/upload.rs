use actix_files::NamedFile;
use actix_multipart::form::{MultipartForm, bytes::Bytes, text::Text};
use actix_web::{Responder, Result, error, get, post, web};
use chrono::Local;
use img_comp::{ImgCompConfig, ImgType, img_comp_with_buf};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::Res;
use crate::common::FILE_STORAGE_TYPE;
use crate::common::types::{FileDir, OssBucket};
use crate::control::app_data::{AppData, SlownWorker};
use crate::middleware::AuthUser;
use crate::utils::files::{get_file_url, put_local_file, put_oss_file, sign_local_file};
use crate::utils::utils::log_err;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UploadRes {
    /// 返回的文件路径 /images/a.jpg
    path: String,
    /// 返回的完整链接 https://...
    url: String,
}

#[derive(Debug, MultipartForm, ToSchema)]
pub struct UploadFile {
    /// 上传的文件，Bytes
    #[multipart(limit = "200 MiB")] //  #[multipart(limit = "5 KiB")]
    #[schema(value_type  = Vec<u8>)]
    file: Bytes,
    /// 上传的文件名,如 a.jpg
    #[schema(value_type  = String)]
    name: Text<String>,
    /// 参数有： banner
    #[schema(value_type  = String)]
    category: Text<String>,
    // /// 具体项目
    // item: Option<Text<String>>,
}
/// 【文件】文件上传
#[utoipa::path(
    request_body = UploadFile,
    responses((status = 200, description = "【请求：UploadFile】【返回：UploadRes】", body = Vec<UploadRes>)),
)]
#[post("/upload/file")]
pub async fn upload_file(
    _user: AuthUser,
    form: MultipartForm<UploadFile>,
    app_data: web::Data<AppData>,
) -> Result<impl Responder> {
    let buf = form.file.data.to_vec();
    let name = form.name.0.clone();
    let cat = form.category.0.clone();
    let data = &app_data;
    // let mut item = "".to_owned();
    // if let Some(it) = &form.item {
    //     let temp = it.0.split("/").collect::<Vec<&str>>();
    //     item = temp[0].to_string();
    //     item.push('/');
    // }
    let name_split = name.split(".").collect::<Vec<&str>>();
    if name_split.len() <= 1 {
        return Err(error::ErrorBadRequest(log_err("文件名问题：", &name_split)));
    }
    let file_type = name_split[name_split.len() - 1];

    // ---- 开始图片压缩 ----
    let buf = match file_type {
        "jpg" | "JPG" | "jpeg" | "JPEG" => img_comp_with_buf(
            buf,
            &ImgCompConfig {
                img_type: ImgType::Jpg,
                resize_width: None,
                quality: 80,
            },
        )
        .map_err(|e| error::ErrorInternalServerError(log_err(&e, "img_compress")))?,
        "png" | "PNG" => img_comp_with_buf(
            buf,
            &ImgCompConfig {
                img_type: ImgType::Png,
                resize_width: None,
                quality: 80,
            },
        )
        .map_err(|e| error::ErrorInternalServerError(log_err(&e, "img_compress")))?,
        _ => buf,
    };
    // ---- 结束图片压缩 ----

    let rand_name = data.rand_no(SlownWorker::OssFileName);
    let oss_dir: FileDir = cat.into();
    if oss_dir == FileDir::Empty {
        return Err(error::ErrorBadRequest("category 参数错误"));
    }
    let dir_path = oss_dir.get_dir();
    let filepath = format!("{dir_path}/{rand_name}.{file_type}");

    if FILE_STORAGE_TYPE == 1 {
        put_local_file(&buf, &dir_path, &filepath)?;
    } else {
        match put_oss_file(&buf, &filepath, OssBucket::EobFiles).await {
            Ok(_) => (),
            Err(e) => return Err(error::ErrorInternalServerError(e)),
        }
    }
    let temp_url = match get_file_url(Some(&filepath)) {
        Some(u) => u,
        None => {
            return Err(error::ErrorInternalServerError("get_file_url"));
        }
    };
    Ok(web::Json(Res::success(UploadRes {
        path: filepath,
        url: temp_url,
    })))
}

#[derive(Deserialize, Debug)]
struct StaticQuery {
    expires: i64,
    signature: String,
}
/// 文件下载
#[get("/static/{file_path:.*}")]
pub async fn static_file_path(
    path: web::Path<String>,
    web::Query(info): web::Query<StaticQuery>,
) -> Result<impl Responder> {
    let file_path = path.to_string();

    let now = Local::now().timestamp();
    if now > info.expires {
        return Err(error::ErrorGone("文件链接已过期"));
    }
    let sign_info = sign_local_file(&file_path, info.expires)?;
    if sign_info != info.signature {
        return Err(error::ErrorForbidden("暂无权限"));
    }

    let file_path = "static/".to_string() + &path.into_inner();
    let file = NamedFile::open(&file_path)?;
    Ok(file)
}

// #[derive(Deserialize, Debug)]
// struct PathQuery {
//     path: String,
// }
// /// 获取文件url
// #[get("/oss/get/excel_img/url")]
// pub async fn oss_get_url(query: web::Query<PathQuery>) -> Result<impl Responder> {
//     let path = query.path.clone();
//     let url = match get_file_url(Some(&path)) {
//         Ok(u) => u,
//         Err(e) => return Err(error::ErrorInternalServerError(log_err(&e, "oss"))),
//     };
//     Ok(web::Json(Res::success(url)))
// }

// /// 上传到本地
// #[post("/upload/file")]
// pub async fn upload_image(
//     _user: AuthUser,
//     form: MultipartForm<UploadFile>,
//     app_data: web::Data<AppData>,
// ) -> Result<impl Responder> {
//     let buf = form.file.data.to_vec();
//     let name = form.name.0.clone();
//     let cat = form.category.0.clone();
//     let data = &app_data;

//     let name_split = name.split(".").collect::<Vec<&str>>();
//     if name_split.len() <= 1 {
//         return Err(error::ErrorBadRequest(log_err("文件名问题：", &name_split)));
//     }
//     let file_type = name_split[name_split.len() - 1];

//     let buf = match file_type {
//         "jpg" | "JPG" | "jpeg" | "JPEG" => img_comp_with_buf(
//             buf,
//             &ImgCompConfig {
//                 img_type: ImgType::Jpg,
//                 resize_width: None,
//                 quality: 80,
//             },
//         )
//         .map_err(|e| error::ErrorInternalServerError(log_err(&e, "img_compress")))?,
//         "png" | "PNG" => img_comp_with_buf(
//             buf,
//             &ImgCompConfig {
//                 img_type: ImgType::Png,
//                 resize_width: None,
//                 quality: 80,
//             },
//         )
//         .map_err(|e| error::ErrorInternalServerError(log_err(&e, "img_compress")))?,
//         _ => buf,
//     };

//     let rand_name = data.rand_no(SlownWorker::OssFileName);
//     let oss_dir: FileDir = cat.into();
//     if oss_dir == FileDir::Empty {
//         return Err(error::ErrorBadRequest("category 参数错误"));
//     }
//     let dir_path = "static/".to_string() + &oss_dir.get_dir();
//     let filepath = format!("{dir_path}/{rand_name}.{file_type}");
//     let path = Path::new(&dir_path);
//     if !path.exists() {
//         std::fs::create_dir(path)?;
//     }
//     let mut filename = String::new();

//     std::fs::write(&filepath, &buf)
//         .map_err(|e| error::ErrorInternalServerError(log_err(&e, "文件写入错误")))?;

//     Ok(web::Json(Res::success(UploadRes {
//         path: "/".to_string() + &filepath,
//         url: STATIC_FILE_URL.to_string() + "/" + &filepath,
//     })))
// }

// 上传到本地
// 用户头像上传
// #[post("/upload/avatar")]
// pub async fn upload_avatar(mut payload: Multipart) -> Result<impl Responder> {
//     let dir_path = "static/avatar/".to_string() + get_now_time(NowTimeType::Date).as_str();
//     let path = Path::new(&dir_path);
//     if !path.exists() {
//         std::fs::create_dir(path)?;
//     }
//     let mut filename = String::new();
//     while let Some(mut field) = payload.try_next().await? {
//         let content_disposition = field.content_disposition();
//         filename = content_disposition
//             .get_filename()
//             .map_or_else(|| rand_string(32), sanitize_filename::sanitize);
//         let filepath = format!("{dir_path}/{filename}");
//         let mut f = web::block(|| std::fs::File::create(filepath)).await??;
//         while let Some(chunk) = field.try_next().await? {
//             f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
//         }
//     }
//     Ok(web::Json(path {
//         status: 2,
//         message: "上传成功".to_string(),
//         url: format!("/{dir_path}/{filename}"),
//         url: format!("/{dir_path}/{filename}"),
//     }))
// }
