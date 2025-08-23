use std::fmt::Debug;

#[allow(unused)]
use crate::utils::crypto::aes_256_encrypt;

/// 将 struct 转成数字串，开发版本直接打印
#[cfg(debug_assertions)]
pub fn log_aes_err<T: Debug + ?Sized, E: Debug + ?Sized>(e: &E, p: &T) -> String {
    format!("{:?}---{:?}", e, p)
}

/// 将 struct 转成数字串，release 版本 加密的
#[cfg(not(debug_assertions))]
pub fn log_aes_err<T: Debug + ?Sized, E: Debug + ?Sized>(e: &E, p: &T) -> String {
    aes_256_encrypt(
        &format!("{:?}---{:?}", e, p),
        crate::common::LocalKeySeed::Logs,
    )
    .unwrap()
}

pub fn log_err<T: Debug + ?Sized, E: Debug + ?Sized>(e: &E, p: &T) -> String {
    format!("{:?}---{:?}", e, p)
}

/// 将 f64 保留为2位数小数，返回f64
pub fn keep_decimal(f_num: f64) -> f64 {
    let s_num = format!("{:.2}", f_num);
    s_num.parse::<f64>().unwrap()
}

/// 将 f64 保留为0位数小数，返回u64
pub fn keep_uint(f_num: f64) -> u64 {
    let s_num = format!("{:.0}", f_num);
    s_num.parse::<u64>().unwrap()
}

/// 计算两个经纬点之前的距离
#[allow(unused)]
pub fn distance_lat_lng(start: (f64, f64), end: (f64, f64)) -> f64 {
    let earth_radius_kilometer = 6371.0_f64;
    let (start_latitude_degrees, start_longitude_degrees) = start;
    let (end_latitude_degrees, end_longitude_degrees) = end;

    let start_latitude = start_latitude_degrees.to_radians();
    let end_latitude = end_latitude_degrees.to_radians();
    let delta_latitude = (start_latitude_degrees - end_latitude_degrees).to_radians();
    let delta_longitude = (start_longitude_degrees - end_longitude_degrees).to_radians();

    let central_angle_inner = (delta_latitude / 2.0).sin().powi(2)
        + start_latitude.cos() * end_latitude.cos() * (delta_longitude / 2.0).sin().powi(2);
    let central_angle = 2.0 * central_angle_inner.sqrt().asin();
    let distance = earth_radius_kilometer * central_angle;

    distance
}

/// role 用户角色格式，将 String 转成 Vec<String>
#[allow(unused)]
pub fn role_to_vec(role: String) -> Vec<String> {
    if role.is_empty() {
        vec![]
    } else {
        role.split(",")
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
    }
}

/// vec 数组中的数据去重
/// ```rust
/// let v = vec![1, 2, 3, 4, 2, 5, 6, 7, 8, 9, 0];
/// println!("Before: {:?}", v);
/// let result = remove_duplicates(v.clone());
/// println!("After : {:?}", result);
/// ```
#[allow(unused)]
pub fn remove_duplicates<T: Eq + Clone>(vec: Vec<T>) -> Vec<T> {
    let mut result = vec![];
    for item in &vec {
        if !result.iter().any(|x| x == item) {
            result.push(item.clone());
        }
    }
    result
}

/// 将手机号，中间四位用 * 号隐藏
pub fn hide_phone_number(phone_number: &str) -> String {
    let mut result = String::new();
    let mut count = 0;
    for c in phone_number.chars() {
        if count >= 3 && count <= 6 {
            result.push('*');
        } else {
            result.push(c);
        }
        count += 1;
    }
    result
}

#[cfg(test)]
mod test {
    use super::distance_lat_lng;

    #[test]
    fn test_distance() {
        let d = distance_lat_lng((29.53648, 106.469246), (29.549815, 106.518975));
        // let d = distance_lat_long((48.85341_f64, -2.34880_f64), (51.50853_f64, -0.12574_f64));

        println!("{:.1}公里", d);
    }
}
