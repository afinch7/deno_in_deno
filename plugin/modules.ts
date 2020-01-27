import {
  newStdLoader,
  stdLoaderAwaitResolve,
  stdLoaderRespondResolve,
  stdLoaderAwaitLoad,
  stdLoaderRespondLoad
} from "./ops.ts";

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
    public onresolve: (
      specifier: string,
      referrer: string,
      isRoot: boolean
    ) => string,
    public onload: (moduleSpecifier: string) => SourceCodeInfo
  ) {
    const response = newStdLoader.dispatchSync({});
    this.stdLoaderRid = response.std_loader_rid;
    this.rid_ = response.loader_rid;
    this.runResolve();
    this.runLoad();
  }

  get rid(): number {
    return this.rid_;
  }

  private async runResolve() {
    while (true) {
      const request = await stdLoaderAwaitResolve.dispatchAsync({
        rid: this.stdLoaderRid
      });
      const module_specifier = this.onresolve(
        request.specifier,
        request.referrer,
        request.is_root
      );
      stdLoaderRespondResolve.dispatchSync({
        rid: this.stdLoaderRid,
        cmd_id: request.cmd_id,
        module_specifier
      });
    }
  }

  private async runLoad() {
    while (true) {
      const request = await stdLoaderAwaitLoad.dispatchAsync({
        rid: this.stdLoaderRid
      });
      const source_code_info = this.onload(request.module_specifier);
      stdLoaderRespondLoad.dispatchSync({
        rid: this.stdLoaderRid,
        cmd_id: request.cmd_id,
        module_name: source_code_info.module_name,
        code: source_code_info.code
      });
    }
  }
}
