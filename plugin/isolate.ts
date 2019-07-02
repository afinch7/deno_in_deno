import { newIsolate, isolateIsComplete, isolateSetDispatcher, isolateExecute } from "./ops.ts";
import { encodeMessage, wrapSyncOpDecode, wrapAsyncOpDecode, ResourceIdResponse } from "./util.ts";
import { Dispatcher } from "./dispatch.ts";

export interface StartupData {
    rid: number;
}

interface NewIsolateAllOptions {
    will_snapshot: boolean;
    startup_data?: StartupData;
}

const defaultNewIsolateOptions = {
    will_snapshot: false,
};

export type NewIsolateOptions = Omit<
    NewIsolateAllOptions, 
    keyof typeof defaultNewIsolateOptions
> & Partial<NewIsolateAllOptions>;

export class Isolate {

    private readonly rid_: number;

    constructor(options?: NewIsolateOptions) {
        const optionsFinal: NewIsolateAllOptions = {
            ...defaultNewIsolateOptions,
            ...options,
        };
        const startup_data_rid = optionsFinal.startup_data ? optionsFinal.startup_data.rid : undefined;
        this.rid_ = wrapSyncOpDecode<ResourceIdResponse>(
            newIsolate.dispatch(
                encodeMessage(
                    {
                        will_snapshot: optionsFinal.will_snapshot,
                        startup_data_rid,
                    },
                )
            )
        ).rid;
    }

    get rid(): number {
        return this.rid_;
    }

    setDispatcher(dispatcher: Dispatcher): void {
        wrapSyncOpDecode(
            isolateSetDispatcher.dispatch(
                encodeMessage(
                    {
                        rid: this.rid_,
                        dispatcher_rid: dispatcher.rid,
                    },
                ),
            ),
        );
    }

    async execute(source: string, filename: string = "<anonymous>"): Promise<void> {
        await wrapAsyncOpDecode(
            isolateExecute.dispatch(
                encodeMessage(
                    {
                        rid: this.rid,
                        source,
                        filename,
                    },
                ),
            ),
        );
        await this.run();
    }

    private async run(): Promise<void> {
        await wrapAsyncOpDecode(
            isolateIsComplete.dispatch(
                encodeMessage(
                    {
                        rid: this.rid_,
                    },
                ),
            ),
        );
        console.log("ISOLATE IS COMPLETE");
    }
}