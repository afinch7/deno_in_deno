import { newStandardDispatcher, standardDispatcherWaitForDispatch, standardDispatcherRespond } from "./ops.ts";
import { encodeMessage, wrapSyncOpDecode, wrapAsyncOpDecode, ResourceIdResponse } from "./util.ts";

export interface Dispatcher {
    rid: number;
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

export class StandardDispatcher implements Dispatcher {

    private readonly rid_: number;
    private readonly stdDispatcherRid: number;
    public ondispatch?: (data: Uint8Array, zero_copy?: Uint8Array) => Uint8Array;

    constructor() {
        const response = wrapSyncOpDecode<NewStandardDispatcherResponse>(
            newStandardDispatcher.dispatch(new Uint8Array(0)),
        );
        this.rid_ = response.dispatcher_rid;
        this.stdDispatcherRid = response.std_dispatcher_rid;
        this.run();
    }

    get rid(): number {
        return this.rid_;
    }

    async respond(cmd_id: number, response: Uint8Array) {
        await wrapSyncOpDecode(
            standardDispatcherRespond.dispatch(
                encodeMessage(
                    {
                        rid: this.stdDispatcherRid,
                        cmd_id: cmd_id,
                    },
                ),
                response,
            ),
        );
    }

    private async run() {
        while(true) {
            const request = await wrapAsyncOpDecode<StandardDispatcherWaitForDispatchResponse> (
                standardDispatcherWaitForDispatch.dispatch(
                    encodeMessage(
                        {
                            rid: this.stdDispatcherRid,
                        },
                    ),
                ),
            );
            const data = new Uint8Array(request.data);
            const zero_copy = request.zero_copy ? new Uint8Array(request.zero_copy) : undefined;
            const response = this.ondispatch(data, zero_copy);
            await wrapSyncOpDecode(
                standardDispatcherRespond.dispatch(
                    encodeMessage(
                        {
                            rid: this.stdDispatcherRid,
                            cmd_id: request.cmd_id,
                        },
                    ),
                    response,
                ),
            );
        }
    }

}