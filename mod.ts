import { newIsolate } from "./ops.ts";

const textEncoder = new TextEncoder();

function encodeMessage(message: any): Uint8Array {
    return textEncoder.encode(JSON.stringify(message));
}

interface DIDError {
    message: string;
}

interface DIDResponse<D = any> {
    error?: DIDError;
    data?: D; 
}

const textDecoder = new TextDecoder();

function decodeMessage<D = any>(message: Uint8Array): DIDResponse<D> {
    return JSON.parse(textDecoder.decode(message));
}

type OpResponse = undefined | Uint8Array;

function wrapSyncOp(response: Promise<OpResponse> | OpResponse): Uint8Array {
    if (response instanceof Uint8Array) {
        return response;
    } else {
        throw new Error(`Unexpected response type for sync op ${typeof response}`);
    }
}




