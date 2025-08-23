use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

use crate::utils::files::{get_file_url, get_file_urls};

/// 用于 serde(deserialize_with) 的函数 - 处理单个url
pub fn deserialize_path_to_url<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let path = Option::<String>::deserialize(deserializer)?;
    let url = get_file_url(path).map_or("".to_string(), |u| u.to_string());
    Ok(url)
}

/// 用于 serde(deserialize_with) 的函数 - 处理多个url
pub fn _deserialize_path_to_urls<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let path = Option::<String>::deserialize(deserializer)?;
    let url = get_file_urls(path);
    Ok(url)
}

/// 递归解析 Value 中的字符串字段
fn parse_nested_json_strings(value: Value) -> Value {
    match value {
        // 如果是字符串，尝试解析为 JSON
        Value::String(s) => {
            // 尝试将字符串解析为 JSON
            match serde_json::from_str::<Value>(&s) {
                Ok(parsed) => {
                    // 递归处理解析后的值
                    parse_nested_json_strings(parsed)
                }
                Err(_) => {
                    // 解析失败，保持原字符串
                    Value::String(s)
                }
            }
        }
        // 如果是对象，递归处理每个字段
        Value::Object(obj) => {
            let mut new_obj = Map::new();
            for (key, val) in obj {
                new_obj.insert(key, parse_nested_json_strings(val));
            }
            Value::Object(new_obj)
        }
        // 如果是数组，递归处理每个元素
        Value::Array(arr) => {
            let new_arr: Vec<Value> = arr.into_iter().map(parse_nested_json_strings).collect();
            Value::Array(new_arr)
        }
        // 其他类型直接返回
        other => other,
    }
}

/// 用于 serde(deserialize_with) 的函数 - 处理单个 Value
pub fn deserialize_nested_json<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(parse_nested_json_strings(value))
}

/// 用于 serde(deserialize_with) 的函数 - 处理 Option<Value>
pub fn _deserialize_nested_json_option<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt_value = Option::<Value>::deserialize(deserializer)?;
    Ok(opt_value.map(parse_nested_json_strings))
}

/// 用于 serde(deserialize_with) 的函数 - 处理 Vec<Value>
pub fn _deserialize_nested_json_vec<'de, D>(deserializer: D) -> Result<Vec<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec_value = Vec::<Value>::deserialize(deserializer)?;
    Ok(vec_value
        .into_iter()
        .map(parse_nested_json_strings)
        .collect())
}

/// 用于 serde(deserialize_with) 的函数 - 处理 HashMap<String, Value>
pub fn _deserialize_nested_json_map<'de, D>(
    deserializer: D,
) -> Result<std::collections::HashMap<String, Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let map = std::collections::HashMap::<String, Value>::deserialize(deserializer)?;
    Ok(map
        .into_iter()
        .map(|(k, v)| (k, parse_nested_json_strings(v)))
        .collect())
}

/// 原始的完整反序列化函数（保留向后兼容）
pub fn _deserialize_json_value<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: Deserializer<'de>,
{
    struct JsonValueVisitor;

    impl<'de> serde::de::Visitor<'de> for JsonValueVisitor {
        type Value = Value;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a JSON value or a JSON string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let parsed_value =
                serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()));
            Ok(parse_nested_json_strings(parsed_value))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let parsed_value =
                serde_json::from_str(&value).unwrap_or_else(|_| Value::String(value));
            Ok(parse_nested_json_strings(parsed_value))
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Value::Bool(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Value::Number(value.into()))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Value::Number(value.into()))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(serde_json::Number::from_f64(value)
                .map(Value::Number)
                .unwrap_or(Value::Null))
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut object = Map::new();
            let mut map = map;
            while let Some((key, value)) = map.next_entry()? {
                let parsed_value = parse_nested_json_strings(value);
                object.insert(key, parsed_value);
            }
            Ok(Value::Object(object))
        }

        fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut array = Vec::new();
            let mut seq = seq;
            while let Some(value) = seq.next_element()? {
                let parsed_value = parse_nested_json_strings(value);
                array.push(parsed_value);
            }
            Ok(Value::Array(array))
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Value::Null)
        }
    }

    deserializer.deserialize_any(JsonValueVisitor)
}

// // 使用示例的结构体
// #[derive(Deserialize, Debug)]
// struct ApiResponse {
//     pub status: String,

//     // 使用 deserialize_with 处理单个 Value
//     #[serde(deserialize_with = "deserialize_nested_json")]
//     pub data: Value,

//     // 使用 deserialize_with 处理 Option<Value>
//     #[serde(deserialize_with = "deserialize_nested_json_option")]
//     pub metadata: Option<Value>,

//     // 使用 deserialize_with 处理 Vec<Value>
//     #[serde(deserialize_with = "deserialize_nested_json_vec")]
//     pub items: Vec<Value>,
// }

// #[derive(Deserialize, Debug)]
// struct ConfigData {
//     pub name: String,

//     // 使用 deserialize_with 处理 HashMap
//     #[serde(deserialize_with = "deserialize_nested_json_map")]
//     pub settings: std::collections::HashMap<String, Value>,
// }

// // 便捷函数：直接从字符串解析带有嵌套 JSON 的数据
// pub fn parse_json_with_nested_strings(json_str: &str) -> Result<Value, serde_json::Error> {
//     let value: Value = serde_json::from_str(json_str)?;
//     Ok(parse_nested_json_strings(value))
// }
