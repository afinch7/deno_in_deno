use serde::Serialize;

pub type ResourceId = u32;

#[derive(Serialize)]
pub struct ResourceIdResponse {
    pub rid: ResourceId,
}
