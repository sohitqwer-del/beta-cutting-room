import type { EditorOutput } from './editor-core';
import type { DocumentState } from './state';
/** Selection is view state. Out-of-range indices are removed after a document changes. */
export function cleanSelection(indices: number[], count: number): number[] {
  return [...new Set(indices.filter(i => Number.isInteger(i) && i >= 0 && i < count))].sort((a, b) => a - b);
}
/** Keep transient measurements outside undoable job/result snapshots. */
export function documentFromEdit(output: EditorOutput): DocumentState { return { job: output.job, result: output.result }; }
export interface LibraryEntry extends DocumentState { id: string; label: string; savedAt: string }
export const LIBRARY_KEY = 'beta-marker-library-v1';
/** Reject malformed library envelopes; Rust checks all saved geometry before reuse. */
export function readLibrary(text: string | null): LibraryEntry[] {
  if (!text) return [];
  const value: unknown = JSON.parse(text);
  if (!Array.isArray(value) || value.length > 50 || value.some(v => !v || typeof v.id !== 'string' || typeof v.label !== 'string' || typeof v.savedAt !== 'string' || v.job?.format !== 'beta-marker.job/v1' || v.result?.format !== 'beta-marker.result/v1')) throw new Error('The marker library is damaged. Restore its JSON backup.');
  return value as LibraryEntry[];
}
export function addLibraryEntry(entries: LibraryEntry[], state: DocumentState, label: string, id: string, savedAt: string): LibraryEntry[] {
  if (!label.trim()) throw new Error('Enter a marker name.');
  if (entries.length >= 50) throw new Error('The library holds 50 markers. Back it up and remove an older entry.');
  return [...entries, { ...structuredClone(state), id, label: label.trim(), savedAt }];
}
