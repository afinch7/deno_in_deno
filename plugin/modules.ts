import { 
    newModuleStore,
    newStdLoader,
    stdLoaderAwaitResolve,
    stdLoaderRespondResolve,
} from "./ops.ts";
import { encodeMessage, wrapSyncOpDecode, wrapAsyncOpDecode, ResourceIdResponse } from "./util.ts";

export class ModuleStore {

    private readonly rid_: number;

    constructor() {
        this.rid_ = wrapSyncOpDecode<ResourceIdResponse>(
            newModuleStore.dispatch(
                encodeMessage(""),
            ),
        ).rid;
    }

    get rid(): number {
        return this.rid_;
    }
}

export interface Loader {
    rid: number;
}

export interface SourceCodeInfo {
    module_name: string;
    code: string;
}

interface NewStdLoaderResponse {
    std_loader_rid: number;
    loader_rid: number;
}

interface StdLoaderAwaitResolveResponse {
    cmd_id: number;
    specifier: string;
    referrer: string;
    is_root: boolean;
}

interface StdLoaderAwaitLoadResponse {
    cmd_id: number;
    module_specifier: string;
}

export class StdLoader implements Loader {

    private readonly rid_: number;
    private readonly stdLoaderRid: number;

    constructor(
        public onresolve: (specifier: string, referrer: string, is_root: boolean) => string,
        public onload: (module_specifier: string) => SourceCodeInfo,
    ) {
        const response = wrapSyncOpDecode<NewStdLoaderResponse>(
            newStdLoader.dispatch(new Uint8Array(0)),
        );
        this.stdLoaderRid = response.std_loader_rid;
        this.rid_ = response.loader_rid;
        this.runResolve();
        this.runLoad();
    }

    get rid(): number {
        return this.rid_;
    }

    private async runResolve() {
        while(true) {
            const request = await wrapAsyncOpDecode<StdLoaderAwaitResolveResponse>(
                stdLoaderAwaitResolve.dispatch(
                    encodeMessage({
                        rid: this.stdLoaderRid,
                    }),
                ),
            );
            const module_specifier = this.onresolve(
                request.specifier,
                request.referrer,
                request.is_root,
            );
            await wrapSyncOpDecode(
                stdLoaderRespondResolve.dispatch(
                    encodeMessage({
                        rid: this.stdLoaderRid,
                        cmd_id: request.cmd_id,
                        module_specifier,
                    }),
                ),
            );
        }
    }

    private async runLoad() {
        while(true) {
            const request = await wrapAsyncOpDecode<StdLoaderAwaitLoadResponse>(
                stdLoaderAwaitResolve.dispatch(
                    encodeMessage({
                        rid: this.stdLoaderRid,
                    }),
                ),
            );
            const source_code_info = this.onload(
                request.module_specifier,
            );
            await wrapSyncOpDecode(
                stdLoaderRespondResolve.dispatch(
                    encodeMessage({
                        rid: this.stdLoaderRid,
                        cmd_id: request.cmd_id,
                        module_name: source_code_info.module_name,
                        code: source_code_info.code,
                    }),
                ),
            );
        }
    }
}