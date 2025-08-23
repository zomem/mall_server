use crate::common::LocalKeySeed;
use crate::utils::crypto::{aes_256_decrypt, aes_256_encrypt};
use actix_web::Error;

/// 用户交易记录的加密
pub fn hash_user_tran(
    uid: u64,
    tran_amount: f64,
    tran_type: &str,
    pay_type: &str,
    time: i64,
) -> anyhow::Result<String, Error> {
    let info = format!(
        "{}_{}_{}_{}_{}",
        uid, tran_amount, tran_type, pay_type, time
    );
    Ok(aes_256_encrypt(&info, LocalKeySeed::UserTranRecord)?)
}

/// 用户交易记录的防篡改校验
#[allow(unused)]
pub fn hash_user_tran_verify(
    hash: &str,
    uid: u64,
    tran_amount: f64,
    tran_type: &str,
    pay_type: &str,
    time: i64,
) -> anyhow::Result<bool, Error> {
    let info = format!(
        "{}_{}_{}_{}_{}",
        uid, tran_amount, tran_type, pay_type, time
    );
    let decrypted_info = aes_256_decrypt(&hash, LocalKeySeed::UserTranRecord)?;
    if decrypted_info == info {
        Ok(true)
    } else {
        Ok(false)
    }
}

/// 用户零钱的加密
pub fn hash_user_pocket_money(uid: u64, amount: f64) -> anyhow::Result<String, Error> {
    let info = format!("{}_{}", uid, amount);
    Ok(aes_256_encrypt(&info, LocalKeySeed::UserPocketMoney)?)
}
/// 用户零钱的检验
pub fn hash_user_pocket_money_verify(
    hash: &str,
    uid: u64,
    amount: f64,
) -> anyhow::Result<bool, Error> {
    let info = format!("{}_{}", uid, amount);
    let decrypted_info = aes_256_decrypt(&hash, LocalKeySeed::UserPocketMoney)?;
    if decrypted_info == info {
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_user_tran() {
        let uid = 12345;
        let tran_amount = 1000.;
        let tran_type = "transfer";
        let pay_type = "alipay";
        let time = 1672531201;

        let hash = hash_user_tran(uid, tran_amount, tran_type, pay_type, time).unwrap();
        println!("Hashed transaction: {}", hash);
        let is_pass =
            hash_user_tran_verify(&hash, uid, -1000., tran_type, pay_type, 1672531201).unwrap();
        println!("is_pass: {}", is_pass);

        println!("xxxx: {}", hash_user_pocket_money(146, 0.).unwrap());

        println!(
            "cccc: {}",
            hash_user_tran_verify(
                "eZr6On5O_nioZWd9Fkqr2HjIRY5wyS18ROY5Gday9rBVj3AG71x-QJDbPA0gMPHk",
                100,
                -298.0,
                "PURCHASE",
                "POCKET_PAY",
                1752047089,
            )
            .unwrap()
        );
    }
}
