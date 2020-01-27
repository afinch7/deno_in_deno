import { newSnapshot, snapshotRead } from "./ops.ts";

export class Snapshot {
  constructor(private readonly _rid: number) {}

  get rid(): number {
    return this._rid;
  }

  read(): any {
    const response = snapshotRead.dispatchSync({ rid: this._rid });
    return new Uint8Array(response.data);
  }
}

export class StdSnapshot extends Snapshot {
  constructor(data: Uint8Array) {
    const response = newSnapshot.dispatchSync({}, data);
    super(response.rid);
  }
}
