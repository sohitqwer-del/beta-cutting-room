/** Browser storage for cutting-room work: cut plans in IndexedDB, other screens in
 * localStorage. Everything stays on this device. A backup file holds all of it. */
export interface LocalStore {
  get<T>(key: string): Promise<T | undefined>;
  set(key: string, value: unknown): Promise<void>;
  remove(key: string): Promise<void>;
  keys(): Promise<string[]>;
}
type KeyValue = Pick<Storage, 'getItem' | 'setItem' | 'removeItem' | 'key' | 'length'>;
const DB = 'beta-cutting-room', TABLE = 'records', FALLBACK = 'beta-record:';

/** Plain localStorage, for browsers that block IndexedDB (some private windows). */
export function storageStore(storage: KeyValue): LocalStore {
  const all = () => Array.from({ length: storage.length }, (_, i) => storage.key(i) ?? '').filter(k => k.startsWith(FALLBACK));
  return {
    async get<T>(key: string) { const v = storage.getItem(FALLBACK + key); return v == null ? undefined : JSON.parse(v) as T; },
    async set(key, value) { storage.setItem(FALLBACK + key, JSON.stringify(value)); },
    async remove(key) { storage.removeItem(FALLBACK + key); },
    async keys() { return all().map(k => k.slice(FALLBACK.length)); },
  };
}
function request<T>(r: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => { r.onsuccess = () => resolve(r.result); r.onerror = () => reject(r.error ?? new Error('Browser storage failed.')); });
}
/** IndexedDB key-value records. Values are stored as JSON text so a backup round-trips exactly. */
export function indexedStore(idb: IDBFactory): LocalStore {
  let db: Promise<IDBDatabase> | undefined;
  const open = () => db ??= new Promise((resolve, reject) => {
    const r = idb.open(DB, 1);
    r.onupgradeneeded = () => { if (!r.result.objectStoreNames.contains(TABLE)) r.result.createObjectStore(TABLE); };
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => { db = undefined; reject(r.error ?? new Error('Browser storage is blocked.')); };
  });
  const table = async (mode: IDBTransactionMode) => (await open()).transaction(TABLE, mode).objectStore(TABLE);
  return {
    async get<T>(key: string) { const v = await request((await table('readonly')).get(key)) as string | undefined; return v == null ? undefined : JSON.parse(v) as T; },
    async set(key, value) { await request((await table('readwrite')).put(JSON.stringify(value), key)); },
    async remove(key) { await request((await table('readwrite')).delete(key)); },
    async keys() { return (await request((await table('readonly')).getAllKeys())).map(String); },
  };
}
let shared: LocalStore | undefined;
/** IndexedDB when available, otherwise localStorage. */
export function localStore(): LocalStore {
  if (shared) return shared;
  const idb = globalThis.indexedDB;
  shared = idb ? indexedStore(idb) : storageStore(globalThis.localStorage);
  return shared;
}

export const BACKUP_FORMAT = 'beta-cutting-room.backup/v1';
export interface Backup {
  format: string;
  saved_at: string;
  /** localStorage entries whose key starts with `beta-` (style, cutting room, floor, settings). */
  local: Record<string, string>;
  /** IndexedDB records (cut plan). */
  records: Record<string, unknown>;
}
const workKey = (key: string) => key.startsWith('beta-') && !key.startsWith(FALLBACK);
export async function makeBackup(store: LocalStore = localStore(), storage: KeyValue = globalThis.localStorage, now = new Date()): Promise<Backup> {
  const local: Record<string, string> = {};
  for (let i = 0; i < storage.length; i++) {
    const key = storage.key(i);
    if (key && workKey(key)) local[key] = storage.getItem(key) ?? '';
  }
  const records: Record<string, unknown> = {};
  for (const key of await store.keys()) records[key] = await store.get(key);
  return { format: BACKUP_FORMAT, saved_at: now.toISOString(), local, records };
}
/** Replace this browser's BETA work with the backup. Returns the number of entries written. */
export async function restoreBackup(value: unknown, store: LocalStore = localStore(), storage: KeyValue = globalThis.localStorage): Promise<number> {
  const backup = value as Partial<Backup> | null;
  if (!backup || backup.format !== BACKUP_FORMAT || typeof backup.local !== 'object' || typeof backup.records !== 'object' || !backup.local || !backup.records) {
    throw new Error('This is not a BETA Cutting Room backup file.');
  }
  for (const [key, text] of Object.entries(backup.local)) {
    if (!workKey(key) || typeof text !== 'string') throw new Error(`Backup entry ${key} is not valid.`);
  }
  const old = Array.from({ length: storage.length }, (_, i) => storage.key(i) ?? '').filter(workKey);
  for (const key of old) storage.removeItem(key);
  for (const key of await store.keys()) await store.remove(key);
  for (const [key, text] of Object.entries(backup.local)) storage.setItem(key, text);
  for (const [key, record] of Object.entries(backup.records)) await store.set(key, record);
  return Object.keys(backup.local).length + Object.keys(backup.records).length;
}
export const backupFileName = (now = new Date()): string => `beta-cutting-room-backup-${now.toISOString().slice(0, 10)}.json`;
