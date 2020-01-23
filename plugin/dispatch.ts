import { getDispatcherAccessorPtrs, newStdDispatcher, stdDispatcherWaitForDispatch, stdDispatcherRespond } from "./ops.ts";

export interface Dispatcher {
    rid: number;
}

export interface DispatcherAccessorPtrs {
    getDispatcher: number;
    insertDispatcher: number;
}

interface GetDispatcherAccessorPtrsResponse {
    get_dispatcher_ptr: number;
    insert_dispatcher_ptr: number;
}

export function getDispatcherAccessors(): DispatcherAccessorPtrs {
    const response = getDispatcherAccessorPtrs.dispatchSync({});
    return {
        getDispatcher: response.get_dispatcher_ptr,
        insertDispatcher: response.insert_dispatcher_ptr,
    }
}

interface NewStandardDispatcherResponse {
    std_dispatcher_rid: number;
    dispatcher_rid: number;
}

interface StandardDispatcherWaitForDispatchResponse {
    cmd_id: number;
    data: number[];
    zero_copy?: number[];
}

export class StdDispatcher implements Dispatcher {

    private readonly rid_: number;
    private readonly stdDispatcherRid: number;
    public ondispatch?: (data: Uint8Array, zero_copy?: Uint8Array) => Uint8Array;

    constructor() {
        const response = newStdDispatcher.dispatchSync({});
        this.rid_ = response.dispatcher_rid;
        this.stdDispatcherRid = response.std_dispatcher_rid;
        this.run();
    }

    get rid(): number {
        return this.rid_;
    }

    async respond(cmd_id: number, response: Uint8Array) {
        stdDispatcherRespond.dispatchSync({
            rid: this.stdDispatcherRid,
            cmd_id: cmd_id,
        });
    }

    private async run() {
        while(true) {
            const request = await stdDispatcherWaitForDispatch.dispatchAsync({
                rid: this.stdDispatcherRid,
            });
            const data = new Uint8Array(request.data);
            const zero_copy = request.zero_copy ? new Uint8Array(request.zero_copy) : undefined;
            const response = this.ondispatch(data, zero_copy);
            stdDispatcherRespond.dispatchSync({
                    rid: this.stdDispatcherRid,
                    cmd_id: request.cmd_id,
                },
                response,
            );
        }
    }

}