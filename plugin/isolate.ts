import { newIsolate, isolateIsComplete, isolateRegisterOp, isolateExecute, isolateExecuteModule, isolateSnapshot } from "./ops.ts";
import { Dispatcher } from "./dispatch.ts";
import { Loader } from "./modules.ts";
import { Snapshot } from "./snapshots.ts";

interface NewIsolateAllOptions {
  will_snapshot: boolean;
  snapshot?: Snapshot;
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

  constructor(loader: Loader, private readonly options?: NewIsolateOptions) {
    const optionsFinal: NewIsolateAllOptions = {
      ...defaultNewIsolateOptions,
      ...this.options,
    };
    const snapshot_rid = optionsFinal.snapshot ? optionsFinal.snapshot.rid : undefined;
    this.rid_ = newIsolate.dispatchSync({
      will_snapshot: optionsFinal.will_snapshot,
      snapshot_rid,
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
    await this.run();
  }

  async executeModule(moduleSpecifier: string) {
    await isolateExecuteModule.dispatchAsync({
      rid: this.rid,
      module_specifier: moduleSpecifier,
    });
    await this.run();
  }

  snapshot(): Snapshot {
    if (this.options.will_snapshot) {
      const response = isolateSnapshot.dispatchSync({ rid: this.rid });
      return new Snapshot(response.rid);
    } else {
      throw Error("Snapshots are not enabled for this isolate");
    }
  }

  async run(): Promise<void> {
    await isolateIsComplete.dispatchAsync({
      rid: this.rid_,
    });
  }
}