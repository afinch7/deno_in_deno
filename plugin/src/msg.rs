use crate::errors::DIDError;
use serde::{Serialize};

#[derive(Serialize)]
pub struct DIDResponse<D: Serialize> {
    pub error: Option<DIDError>,
    pub data: Option<D>,
}

pub type ResourceId = u32;

#[derive(Serialize)]
pub struct ResourceIdResponse {
    pub rid: ResourceId,
}

#[derive(Serialize)]
pub struct EmptyResponse;
