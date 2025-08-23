use rand::Rng;
use uuid::Uuid;

/// 随机字符串 生成 ，传入生成长度。
/// ```
/// let ran_1 = rand_string(12);
///
/// let ran_2 = rand_string(32);
/// ```
///
pub fn rand_string(len: u16) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    // const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    let ran_string = (0..len)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    ran_string
}

/// uuid 的随机字符串，32位，不含 -
pub fn rand_unique() -> String {
    Uuid::new_v4().to_string().replace("-", "")
}

#[cfg(test)]
mod test {
    #[test]
    fn test_get_time() {
        // rand_slown();

        assert!(true)
    }
}
