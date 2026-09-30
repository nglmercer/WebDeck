import { HttpError } from '../../api/client';
import type { JsonObject } from '../../framework/types';

/** Persisted snapshot and mutable editor drafts have distinct ownership. */
export class EditorPersistence {
  revision: number | undefined;
  persisted: JsonObject = {};
  pending = false;
  seed(config: JsonObject, revision?: number): void {
    this.persisted = structuredClone(config);
    this.revision = revision;
  }
  draft(): JsonObject { return structuredClone(this.persisted); }
  async save<T extends { success?: boolean; revision?: number }>(operation: (revision: number | undefined) => Promise<T>): Promise<T> {
    if (this.pending) throw new Error('A save is already in progress');
    this.pending = true;
    try {
      const result = await operation(this.revision);
      if (result.success && result.revision !== undefined) this.revision = result.revision;
      return result;
    } catch (error) {
      if (error instanceof HttpError && error.status === 409) {
        throw new Error('Configuration changed on another device. Your draft is preserved; reload before saving.');
      }
      throw error;
    } finally { this.pending = false; }
  }
}
export const editorPersistence = new EditorPersistence();
