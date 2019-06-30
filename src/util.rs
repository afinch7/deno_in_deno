use crate::errors::TsDenoError;
use crate::msg::TsDenoResponse;
use deno::Buf;
use deno::CoreOp;
use deno::Op;
use deno::OpResult;
use deno::PinnedBuf;
use serde::Serialize;
use futures::future::Future;

pub type TsDenoOpResult = OpResult<TsDenoError>;

pub type TsDenoOpFn = fn(&[u8], Option<PinnedBuf>) -> TsDenoOpResult;

#[derive(Serialize)]
struct Empty;

pub fn wrap_op(op: TsDenoOpFn, data: &[u8], zero_copy: Option<PinnedBuf>) -> CoreOp {
    match op(data, zero_copy) {
        Ok(Op::Sync(buf)) => Op::Sync(buf),
        Ok(Op::Async(fut)) => {
            let result_fut = Box::new(
                fut.or_else(move |err: TsDenoError| -> Result<Buf, ()> {
                    let result = TsDenoResponse::<Empty> {
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
            let result = TsDenoResponse::<Empty> {
                error: Some(err),
                data: None,
            };
            let result_json = serde_json::to_string(&result).unwrap();
            Op::Sync(result_json.as_bytes().into())
        }
    }
}

pub fn serialize_response<D: Serialize>(data: D) -> Buf {
    let result = TsDenoResponse {
        data: Some(data),
        error: None,
    };
    let result_json = serde_json::to_string(&result).unwrap();
    result_json.as_bytes().into()
}

pub fn serialize_and_wrap<D: Serialize>(data: D) -> TsDenoOpResult {
    Ok(Op::Sync(serialize_response(data)))
}
