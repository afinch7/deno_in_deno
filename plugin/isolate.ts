import { newIsolate, isolateIsComplete, isolateSetDispatcher, isolateExecute, isolateExecuteModule } from "./ops.ts";
import { encodeMessage, wrapSyncOpDecode, wrapAsyncOpDecode, ResourceIdResponse } from "./util.ts";
import { Dispatcher } from "./dispatch.ts";
import { Loader, ModuleStore } from "./modules.ts";

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
        this.run();
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
    }

    async executeModule(moduleSpecifier: string, loader: Loader, module_store: ModuleStore) {
        await wrapAsyncOpDecode(
            isolateExecuteModule.dispatch(
                encodeMessage(
                    {
                        rid: this.rid,
                        loader_rid: loader.rid,
                        module_store_rid: module_store.rid,
                        module_specifier: moduleSpecifier,
                    },
                ),
            ),
        );
    }

    async run(): Promise<void> {
        await wrapAsyncOpDecode(
            isolateIsComplete.dispatch(
                encodeMessage(
                    {
                        rid: this.rid_,
                    },
                ),
            ),
        );
    }
}