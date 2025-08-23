use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ProductAttr {
    id: u32,
    primary_id: u32,
    secondary_id: u32,
    primary_name: String,
    secondary_name: String,
    content: Option<String>,
}
