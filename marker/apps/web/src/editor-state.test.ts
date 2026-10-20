import { describe, expect, it } from 'vitest';
import { addLibraryEntry, cleanSelection, documentFromEdit, readLibrary } from './editor-state';
import { demoJob, demoResult } from './demo';
import { History } from './state';
import { editorSettings } from './editor-core';
describe('daily editor state', () => {
  const state = {job:demoJob,result:demoResult};
  it('deduplicates selection and removes stale or malformed indices', () => {
    expect(cleanSelection([3,1,3,-1,1.5,NaN,8],8)).toEqual([1,3]);
  });
  it('keeps drawing safe while Rust reports malformed saved editor metadata', () => {
    expect(editorSettings({...demoJob,meta:{editor_v1:{sections:'bad',table_length:'bad'}}})).toEqual({sections:[],table_length:undefined});
    expect(editorSettings({...demoJob,meta:{editor_v1:null}})).toEqual({});
    expect(editorSettings({...demoJob,meta:{editor_v1:{sections:[100,200],table_length:300}}})).toEqual({sections:[100,200],table_length:300});
  });
  it('commits a multi-piece edit once and preserves immutable undo/redo', () => {
    const history = new History(state);
    const changed = structuredClone(state);
    changed.result.placements[0]!.translate=[50,50];
    changed.result.placements[1]!.translate=[70,50];
    history.commit(documentFromEdit({...changed,selected:[0,1],measurement:5,notice:'Moved'}));
    expect(history.current).toEqual(changed);
    expect(history.undo()).toEqual(state);
    expect(history.canUndo).toBe(false);
    expect(history.redo()).toEqual(changed);
    expect(history.current).not.toHaveProperty('measurement');
  });
  it('keeps removed identities, sections and table length in history snapshots', () => {
    const history=new History(state), changed=structuredClone(state);
    const removed=changed.result.placements.splice(0,1);
    changed.job.meta={...changed.job.meta,editor_v1:JSON.parse(JSON.stringify({removed,sections:[740],table_length:1600}))};
    history.commit(changed);
    expect(history.current.job.meta?.editor_v1).toEqual({removed,sections:[740],table_length:1600});
    expect(history.undo()).toEqual(state);expect(history.redo()).toEqual(changed);
  });
  it('round trips a library backup and takes a deep copy of the saved job', () => {
    const entries=addLibraryEntry([],state,' Shirt M:L ','one','2026-01-01T00:00:00Z');
    expect(entries[0]!.label).toBe('Shirt M:L');
    expect(entries[0]!.job).not.toBe(demoJob);
    expect(readLibrary(JSON.stringify(entries))).toEqual(entries);
    expect(readLibrary(null)).toEqual([]);
  });
  it('rejects broken library storage, unnamed entries and excess entries', () => {
    expect(()=>readLibrary('{}')).toThrow();expect(()=>readLibrary('[{}]')).toThrow();
    expect(()=>addLibraryEntry([],state,' ','id','date')).toThrow();
    const entries=addLibraryEntry([],state,'test','id','date');
    expect(()=>addLibraryEntry(Array(50).fill(entries[0]),state,'extra','id','date')).toThrow();
  });
});
