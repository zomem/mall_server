use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

mod order;
pub use order::*;
mod coupon;
pub use coupon::*;
mod product;
pub use product::*;
mod product_file;
pub use product_file::*;
mod product_group;
pub use product_group::*;
mod brand;
pub use brand::*;
mod cat;
pub use cat::*;
mod attr;
pub use attr::*;
mod store;
pub use store::*;
mod write_off;
pub use write_off::*;

#[derive(Serialize, Debug, Deserialize, ToSchema, Clone)]
pub struct UnitAttrInfo {
    pub primary_name: String,
    pub secondary_name: String,
}
