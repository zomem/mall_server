use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Local;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
    errors::Error as JwtError,
};
use serde::{Deserialize, Serialize};

use crate::common::JWT_TOKEN_SECRET;
use crate::middleware::AuthUser;

/// Our claims struct, it needs to derive `Serialize` and/or `Deserialize`
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub id: u64,
    pub exp: usize,
}

/// 创建token， expires_sec 为 token 的有效时间（秒）
pub fn get_token(user: AuthUser, expires_sec: i64) -> std::io::Result<String> {
    // let now_time = Local::now().timestamp();
    let now_time = Local::now()
        .checked_add_signed(chrono::Duration::seconds(expires_sec))
        .expect("valid timestamp")
        .timestamp();

    let header = Header::new(Algorithm::HS512);
    let secret = JWT_TOKEN_SECRET.to_string();

    let claims = Claims {
        id: user.id,
        exp: now_time as usize,
    };

    let token = encode(&header, &claims, &EncodingKey::from_secret(secret.as_ref())).unwrap();

    Ok(token)
}

/// 验证 token
pub fn validate_token(token: &str) -> Result<TokenData<Claims>, JwtError> {
    let validation = Validation::new(Algorithm::HS512);
    let secret = JWT_TOKEN_SECRET.to_string();
    let key = DecodingKey::from_secret(secret.as_ref());
    let data = decode::<Claims>(token, &key, &validation)?;
    Ok(data)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenUid {
    pub id: u64,
}
/// 仅获取 uid
pub fn uid_token(token: &str) -> TokenUid {
    let mut uid = TokenUid { id: 0 };
    let s = token.split(".").collect::<Vec<&str>>();
    if s.len() > 1 {
        let v = match URL_SAFE_NO_PAD.decode(s[1]) {
            Ok(d) => d,
            Err(_) => return uid,
        };
        uid = match serde_json::from_slice(&v) {
            Ok(d) => d,
            Err(_) => return uid,
        };
    }
    uid
}

#[cfg(test)]
mod test {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    #[test]
    fn test_get_jwt_header() {
        let t = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJpZCI6MSwiZXhwIjoxNzIzMjYzNjQ5fQ._B4IxQ1Ny16qXxUn56IlFKYO2Zn7V7gBaj_nm5DVaa_2yUIHLmHbi9ixo8KluNMamGLNM-g7MKDWg3aDUV1wTw";

        // use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        let s = t.split(".").collect::<Vec<&str>>();
        let a = URL_SAFE_NO_PAD.decode(s[1]).unwrap();
        let c: serde_json::Value = serde_json::from_slice(&a).unwrap();
        println!("a>>> {:?}", c);
        assert!(true)
    }
}
