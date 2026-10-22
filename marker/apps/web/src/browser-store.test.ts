import { describe, expect, it } from 'vitest';
import { BACKUP_FORMAT, makeBackup, restoreBackup, storageStore } from './browser-store';

class MemoryStorage {
  private map = new Map<string, string>();
  get length() { return this.map.size; }
  key(i: number) { return [...this.map.keys()][i] ?? null; }
  getItem(k: string) { return this.map.get(k) ?? null; }
  setItem(k: string, v: string) { this.map.set(k, String(v)); }
  removeItem(k: string) { this.map.delete(k); }
}

describe('browser storage and backup file', () => {
  it('round-trips every BETA entry and nothing else', async () => {
    const storage = new MemoryStorage(), records = storageStore(new MemoryStorage());
    storage.setItem('beta-floor-v1', '{"lays":[1]}');
    storage.setItem('beta-cutting-room-v1', '{"register":{"rolls":[]}}');
    storage.setItem('other-app', 'keep me out');
    await records.set('plan', { plan: { lays: [{ plies: 40, passes: 20 }] } });
    const backup = await makeBackup(records, storage, new Date('2026-10-22T10:00:00Z'));
    expect(backup).toEqual({ format: BACKUP_FORMAT, saved_at: '2026-10-22T10:00:00.000Z',
      local: { 'beta-floor-v1': '{"lays":[1]}', 'beta-cutting-room-v1': '{"register":{"rolls":[]}}' },
      records: { plan: { plan: { lays: [{ plies: 40, passes: 20 }] } } } });
    const target = new MemoryStorage(), targetRecords = storageStore(new MemoryStorage());
    target.setItem('beta-style-v1', 'old work is replaced');
    target.setItem('other-app', 'untouched');
    await targetRecords.set('stale', 1);
    expect(await restoreBackup(JSON.parse(JSON.stringify(backup)), targetRecords, target)).toBe(3);
    expect(target.getItem('beta-style-v1')).toBeNull();
    expect(target.getItem('other-app')).toBe('untouched');
    expect(target.getItem('beta-floor-v1')).toBe('{"lays":[1]}');
    expect(await targetRecords.keys()).toEqual(['plan']);
    expect(await targetRecords.get('plan')).toEqual(backup.records.plan);
  });
  it('rejects files that are not BETA backups before changing anything', async () => {
    const storage = new MemoryStorage();
    storage.setItem('beta-floor-v1', 'kept');
    const records = storageStore(new MemoryStorage());
    const bad = [null, {}, { format: 'other' }, { format: BACKUP_FORMAT, local: { evil: 'x' }, records: {} },
      { format: BACKUP_FORMAT, local: { 'beta-x': 5 }, records: {} }];
    for (const value of bad) await expect(restoreBackup(value, records, storage)).rejects.toThrow();
    expect(storage.getItem('beta-floor-v1')).toBe('kept');
  });
});
