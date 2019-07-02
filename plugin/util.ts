const textEncoder = new TextEncoder();

export function encodeMessage(message: any): Uint8Array {
    return textEncoder.encode(JSON.stringify(message));
}

export interface DIDError {
    message: string;
}

export interface DIDResponse<D = any> {
    error?: DIDError;
    data?: D; 
}

const textDecoder = new TextDecoder();

export function decodeMessage<D = any>(message: Uint8Array): DIDResponse<D> {
    return JSON.parse(textDecoder.decode(message));
}

type OpResponse = undefined | Uint8Array;
type OpResponseAnySync = Promise<OpResponse> | OpResponse;

export function wrapSyncOp(response: OpResponseAnySync): Uint8Array {
    if (response instanceof Uint8Array) {
        return response;
    } else {
        throw new Error(`Unexpected response type for sync op ${typeof response}`);
    }
}

export function wrapSyncOpDecode<D = any>(response: OpResponseAnySync): D {
    const result = decodeMessage<D>(wrapSyncOp(response));
    if (result.error) {
        throw new Error(`Op error: ${result.error.message}`);
    }
    return result.data;
}

export async function wrapAsyncOp(response: OpResponseAnySync): Promise<Uint8Array> {
    if (response instanceof Promise) {
        const result = await response;
        if (result instanceof Uint8Array) {
            return result;
        } else {
            throw new Error(`Unexpected result type for async op ${typeof result}`)
        }
    } else {
        throw new Error(`Unexpected response type for async op ${typeof response}`)
    }
}

export async function wrapAsyncOpDecode<D = any>(response: OpResponseAnySync): Promise<D> {
    const result = decodeMessage<D>(await wrapAsyncOp(response));
    if (result.error) {
        throw new Error(`Op error: ${result.error.message}`);
    }
    return result.data;
}

export interface ResourceIdResponse {
    rid: number;
}