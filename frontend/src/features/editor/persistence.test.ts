import { describe, expect, it } from 'vitest';
import { HttpError } from '../../api/client';
import { EditorPersistence } from './persistence';

describe('editor persistence ownership', () => {
  it('keeps editable copies separate and preserves revision/draft after a conflict', async () => {
    const owner = new EditorPersistence();
    owner.seed({ front: { buttons: { index: [{ name: 'original' }] } } }, 4);
    const draft = owner.draft();
    draft['front'] = { buttons: {} };
    await expect(owner.save(() => Promise.reject(new HttpError(409, 'Conflict', '/save')))).rejects.toThrow('draft is preserved');
    expect(owner.revision).toBe(4);
    expect(owner.persisted['front']).toEqual({ buttons: { index: [{ name: 'original' }] } });
    expect(draft['front']).toEqual({ buttons: {} });
    expect(owner.pending).toBe(false);
  });
  it('admits one save, updates only on success, and never retries', async () => {
    const owner = new EditorPersistence(); owner.seed({}, 2);
    let calls=0;
    let complete!: (result: { success: boolean; revision: number }) => void;
    const first=owner.save(revision => { calls++; expect(revision).toBe(2); return new Promise<{ success: boolean; revision: number }>(resolve => complete=resolve); });
    await expect(owner.save(async () => { calls++; return { success: true }; })).rejects.toThrow('already in progress');
    complete({ success: true, revision: 3 }); await first;
    expect(owner.revision).toBe(3); expect(calls).toBe(1);
    await owner.save(async () => ({ success: false, revision: 99 }));
    expect(owner.revision).toBe(3);
  });
});
