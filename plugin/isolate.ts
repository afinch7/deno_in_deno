import { newIsolate, isolateIsComplete, isolateRegisterOp, isolateExecute, isolateExecuteModule } from "./ops.ts";
import { Dispatcher } from "./dispatch.ts";
import { Loader } from "./modules.ts";

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

    constructor(loader: Loader, options?: NewIsolateOptions) {
        const optionsFinal: NewIsolateAllOptions = {
            ...defaultNewIsolateOptions,
            ...options,
        };
        const startup_data_rid = optionsFinal.startup_data ? optionsFinal.startup_data.rid : undefined;
        this.rid_ = newIsolate.dispatchSync({
            will_snapshot: optionsFinal.will_snapshot,
            startup_data_rid,
            loader_rid: loader.rid,
        }).rid;
    }

    get rid(): number {
        return this.rid_;
    }

    registerOp(name: string, dispatcher: Dispatcher): void {
        isolateRegisterOp.dispatchSync({
            rid: this.rid_,
            dispatcherRid: dispatcher.rid,
            name,
        });
    }

    async execute(source: string, filename: string = "<anonymous>"): Promise<void> {
        await isolateExecute.dispatchAsync({
            rid: this.rid,
            source,
            filename,
        });
    }

    async executeModule(moduleSpecifier: string) {
        await isolateExecuteModule.dispatchAsync({
            rid: this.rid,
            module_specifier: moduleSpecifier,
        });
        await this.run();
    }

    async run(): Promise<void> {
        await isolateIsComplete.dispatchAsync({
            rid: this.rid_,
        });
    }
}