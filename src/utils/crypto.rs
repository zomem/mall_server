use actix_web::{Error, error};
use base64::{Engine, engine};
use libaes::Cipher;

use crate::common::{LOCAL_AES_256_KEY, LocalKeySeed};

// const MAX_STR_LEN: usize = 5000;

/// 字符串&[u8]转 base64
pub fn base64_encode(data: &Vec<u8>) -> String {
    let encoded: String = engine::general_purpose::URL_SAFE.encode(data);
    encoded
}
/// base64 转 &[u8]
pub fn base64_decode<'a>(data: &str) -> anyhow::Result<Vec<u8>, Error> {
    let decoded = engine::general_purpose::URL_SAFE
        .decode(data)
        .map_err(|_| error::ErrorBadRequest("字符码错误"))?;
    Ok(decoded)
}

/// 基于种子的位置重排
fn obfuscate_reorder(input: &str, seed: u16) -> String {
    if input.len() != 32 {
        panic!("Input must be exactly 32 characters");
    }

    // 使用种子生成确定性的重排规则
    let mut reorder_map: Vec<usize> = (0..32).collect();

    // 基于种子的简单随机数生成器（线性同余生成器）
    let mut rng = seed as u32;

    // Fisher-Yates 洗牌算法
    for i in (1..32).rev() {
        rng = (rng.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff;
        let j = (rng as usize) % (i + 1);
        reorder_map.swap(i, j);
    }

    let chars: Vec<char> = input.chars().collect();
    let mut result = vec![' '; 32];

    for (new_pos, &old_pos) in reorder_map.iter().enumerate() {
        result[new_pos] = chars[old_pos];
    }

    result.iter().collect()
}

/// 恢复原始字符串
fn deobfuscate_reorder(input: &str, seed: u16) -> String {
    if input.len() != 32 {
        panic!("Input must be exactly 32 characters");
    }

    // 重新生成相同的重排规则
    let mut reorder_map: Vec<usize> = (0..32).collect();
    let mut rng = seed as u32;

    for i in (1..32).rev() {
        rng = (rng.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff;
        let j = (rng as usize) % (i + 1);
        reorder_map.swap(i, j);
    }

    let chars: Vec<char> = input.chars().collect();
    let mut result = vec![' '; 32];

    // 反向映射
    for (new_pos, &old_pos) in reorder_map.iter().enumerate() {
        result[old_pos] = chars[new_pos];
    }

    result.iter().collect()
}

/// aes_256_加密
pub fn aes_256_encrypt(endata: &str, seed: LocalKeySeed) -> anyhow::Result<String, Error> {
    let obfuscated = obfuscate_reorder(LOCAL_AES_256_KEY, seed as u16);
    let plain = endata.as_bytes();
    let my_key: &[u8; 32] = obfuscated
        .as_bytes()
        .try_into()
        .map_err(|_| error::ErrorBadRequest("字符码错误"))?; // key is 16 bytes, i.e. 128-bit
    let iv = &my_key[0..16];
    // Create a new 256-bit cipher
    let cipher = Cipher::new_256(my_key);
    // Encryption
    let encrypted = cipher.cbc_encrypt(iv, plain);
    Ok(base64_encode(&encrypted))
    // 勿删！！！ 这里是一个示例，，&[u8] 如何应对任意长度
    // let mut buf = [0u8; MAX_STR_LEN];
    // let pt_len = plain.len();
    // buf[..pt_len].copy_from_slice(plain);
    // let ct = Aes256CbcEnc::new(LOCAL_AES_256_KEY.as_bytes().into())
    //     .encrypt_padded_b2b_mut::<Pkcs7>(plain, &mut buf)
    //     .unwrap();
}

/// aes_256_解密
pub fn aes_256_decrypt(dedata: &str, seed: LocalKeySeed) -> anyhow::Result<String, Error> {
    let obfuscated = obfuscate_reorder(LOCAL_AES_256_KEY, seed as u16);
    let dec_data = base64_decode(dedata).map_err(|_| error::ErrorBadRequest("字符码错误"))?;
    let my_key: &[u8; 32] = obfuscated
        .as_bytes()
        .try_into()
        .map_err(|_| error::ErrorBadRequest("字符码错误"))?; // key is 16 bytes, i.e. 128-bit
    let iv = &my_key[0..16];
    let cipher = Cipher::new_256(my_key);
    // Decryption
    let decrypted = cipher.cbc_decrypt(iv, &dec_data[..]);
    Ok(
        String::from_utf8(decrypted.to_owned())
            .map_err(|_| error::ErrorBadRequest("字符码错误"))?,
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::common::LOCAL_AES_256_KEY;
    use crate::common::LocalKeySeed;
    #[test]
    fn test_aes() {
        let original = "XnhjsWM7gVnV2tWLF9xSdp2AalxElw8J";
        let seed = 2u16;
        let obfuscated = obfuscate_reorder(original, seed);
        let recovered = deobfuscate_reorder(&obfuscated, seed);
        println!("_______ {} {}", recovered, obfuscated);

        let str1 = "SELECT * FROM usr_silent WHERE usr_silent.id = 1 ";
        let ss = aes_256_encrypt(str1, LocalKeySeed::Logs);
        println!("加密数据：{}", ss.unwrap());
        // let data = "N9f/zP4140jA9gwchf/hc96zmJxumBIkC1xTWCyFuieeyob09DJKJpy55HJv4dT0mGaJIl5kJGPbMPgWHLvxzg==";
        // let data = "3g5NxKttmPEbtHm7BEGg2UpVBOvW9hsIKP63ts4i502_r2jS46nNQDsT-dLR-_ndz6N1UvSH4TeXTsKVCCdcDPG_npbJrsqNs2ReaGsTsVIbtCP5RXS3fJi88kv95sC5cSu_wt3gfW3ipoXZrKoByITZvuJhvgQETmzKxfQGmAjWMovMx02yutak1V8O9dwk8dYnQj4Q5RjJ5_1eKee35YZMOuG-gngHF-3xC6cRnBh9733DF8F8iKIijQmHPnGpGBsmbK75U79dfCoGiMonJSDB8oyYvxpDYoons5z4KBh7DrXaRqucvhF_zplWEYGVXsIiQ38_uCvWLmdGlcgKpw6yaKFWRor5EyrAw48e5S6saiGwiUC8VB6zHnqfJqrLLREEoQCmiQnnI2q6A9NqPnYtH4FmX_1mYY3II0tNMcGzNZ9riAcU1R_TjB-UnpurY65xYNbYVs0JgYDnAMeJBFdx8D6iwux5gpXDlkIWT7X0pRbWsYmEslREvn6lTPN3";
        // let es = aes_256_decrypt(&data, LocalKeySeed::Logs);
        // println!("解密数据：{}", es);

        let data = "3g5NxKttmPEbtHm7BEGg2UpVBOvW9hsIKP63ts4i502_r2jS46nNQDsT-dLR-_ndz6N1UvSH4TeXTsKVCCdcDPG_npbJrsqNs2ReaGsTsVIbtCP5RXS3fJi88kv95sC5cSu_wt3gfW3ipoXZrKoByITZvuJhvgQETmzKxfQGmAjWMovMx02yutak1V8O9dwk8dYnQj4Q5RjJ5_1eKee35YZMOuG-gngHF-3xC6cRnBh9733DF8F8iKIijQmHPnGpGBsmbK75U79dfCoGiMonJSDB8oyYvxpDYoons5z4KBh7DrXaRqucvhF_zplWEYGVXsIiQ38_uCvWLmdGlcgKpw6yaKFWRor5EyrAw48e5S6saiGwiUC8VB6zHnqfJqrLLREEoQCmiQnnI2q6A9NqPnYtH4FmX_1mYY3II0tNMcGzNZ9riAcU1R_TjB-UnpurY65xYNbYVs0JgYDnAMeJBFdx8D6iwux5gpXDlkIWT7X0pRbWsYmEslREvn6lTPN3";
        let dec_data = base64_decode(data).unwrap();
        let my_key: &[u8; 32] = LOCAL_AES_256_KEY.as_bytes().try_into().unwrap(); // key is 16 bytes, i.e. 128-bit
        let iv = &my_key[0..16];
        let cipher = Cipher::new_256(my_key);
        // Decryption
        let decrypted = cipher.cbc_decrypt(iv, &dec_data[..]);
        let es = String::from_utf8(decrypted.to_owned()).unwrap();
        println!("解密数据：{}", es);
    }
}
