export function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
import type { ConfigResponse, Config } from './contracts';
export class Editor {
  revision = $state(0);
  draft: Config = $state(null!);
  dirty = $state(false);
  constructor(snapshot: ConfigResponse) {
    this.revision = snapshot.revision;
    this.draft = clone(snapshot.config);
  }
  change() {
    this.dirty = true;
  }
  saved(snapshot: ConfigResponse) {
    this.revision = snapshot.revision;
    this.draft = clone(snapshot.config);
    this.dirty = false;
  }
}
