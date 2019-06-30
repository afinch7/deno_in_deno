use crate::errors::TsDenoError;
use serde::{Serialize};

#[derive(Serialize)]
pub struct TsDenoResponse<D: Serialize> {
    pub error: Option<TsDenoError>,
    pub data: Option<D>,
}

#[derive(Serialize)]
pub struct ResourceIdResponse {
    pub rid: u32,
}
