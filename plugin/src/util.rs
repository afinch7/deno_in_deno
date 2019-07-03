use crate::errors::DIDError;
use crate::msg::DIDResponse;
use crate::msg::EmptyResponse;
use deno::Buf;
use deno::CoreOp;
use deno::Op;
use deno::OpResult;
use deno::PinnedBuf;
use serde::Serialize;
use tokio::prelude::Future;

pub type DIDOpResult = OpResult<DIDError>;

pub type DIDOpFn = fn(&[u8], Option<PinnedBuf>) -> DIDOpResult;

pub fn wrap_op(op: DIDOpFn, data: &[u8], zero_copy: Option<PinnedBuf>) -> CoreOp {
    match op(data, zero_copy) {
        Ok(Op::Sync(buf)) => Op::Sync(buf),
        Ok(Op::Async(fut)) => {
            let result_fut = Box::new(
                fut.or_else(move |err: DIDError| -> Result<Buf, ()> {
                    let result = DIDResponse::<EmptyResponse> {
                        error: Some(err),
                        data: None,
                    };
                    let result_json = serde_json::to_string(&result).unwrap();
                    Ok(result_json.as_bytes().into())
                })
            );
            Op::Async(result_fut)
        },
        Err(err) => {
            let result = DIDResponse::<EmptyResponse> {
                error: Some(err),
                data: None,
            };
            let result_json = serde_json::to_string(&result).unwrap();
            Op::Sync(result_json.as_bytes().into())
        }
    }
}

pub fn serialize_response<D: Serialize>(data: D) -> Buf {
    let result = DIDResponse {
        data: Some(data),
        error: None,
    };
    let result_json = serde_json::to_string(&result).unwrap();
    result_json.as_bytes().into()
}

pub fn serialize_and_wrap<D: Serialize>(data: D) -> DIDOpResult {
    Ok(Op::Sync(serialize_response(data)))
}
