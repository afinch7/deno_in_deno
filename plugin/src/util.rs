use crate::errors::DIDResult;
use crate::msg::DIDResponse;
use crate::msg::EmptyResponse;
use deno::Op;
use deno::Buf;
use deno::CoreOp;
use deno::PinnedBuf;
use futures::future::Future;
use futures::future::FutureExt;
use serde::Serialize;
use std::pin::Pin;

pub type DIDOpAsyncFuture = Pin<Box<dyn Future<Output = DIDResult<Buf>> + Send>>;
pub enum DIDOp {
    Sync(Buf),
    Async(DIDOpAsyncFuture),
}

pub type DIDOpResult = DIDResult<DIDOp>;

pub type DIDOpFn = fn(&[u8], Option<PinnedBuf>) -> DIDOpResult;

pub fn wrap_op(op: DIDOpFn, data: &[u8], zero_copy: Option<PinnedBuf>) -> CoreOp {
    match op(data, zero_copy) {
        Ok(DIDOp::Sync(buf)) => Op::Sync(buf),
        Ok(DIDOp::Async(fut)) => {
            let result_fut = async {
                match fut.await {
                    Ok(v) => v,
                    Err(err) => {
                        let result = DIDResponse::<EmptyResponse> {
                            error: Some(err),
                            data: None,
                        };
                        let result_json = serde_json::to_string(&result).unwrap();
                        result_json.as_bytes().into()
                    },
                }
            }.boxed();
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

pub fn serialize_sync_result<D: Serialize>(data: D) -> DIDOpResult {
    Ok(DIDOp::Sync(serialize_response(data)))
}
