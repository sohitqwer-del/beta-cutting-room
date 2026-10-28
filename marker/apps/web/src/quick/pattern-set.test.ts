import { describe, expect, it } from 'vitest';
import { mockMerge, mockScan } from './mock';
import * as S from './state';
import { openPatternSet, savePatternSet } from './pattern-set';
import type { QuickPiece, TapeReport } from '@beta/marker-wasm';

function fixture(): S.QuickState {
  let s = S.initialState();
  const scans = [1,2].map(i => mockScan(new Uint8Array([1]),{ reference:'a4-sheet',photoId:`photo-${i}` }));
  for (const scan of scans) s = S.photoScanned(S.addPhoto(s,scan.photoId,`${scan.photoId}.jpg`),scan.photoId,scan);
  return S.ensureGarments(S.acceptAllPending(S.applySession(s,mockMerge(scans))));
}
const pass: TapeReport = { endpoints:[[0,0],[100,0]], measured_mm:100, digital_mm:100, error_mm:0, tolerance_mm:3, pass:true };
const assist = async (p: QuickPiece[]) => structuredClone(p);

describe('paper pattern sets', () => {
  it('requires a real check for every used photo and retains known failures', () => {
    let s = fixture();
    expect(S.uncheckedPhotos(s)).toEqual(['photo-1','photo-2']);
    for (const photo of s.photos) s = S.recordTape(s,s.pieces.find(p => p.piece.photoId === photo.id)!.piece.id,pass);
    expect(S.blocker(s,'nest')).toBeNull();
    s = S.recordTape(s,s.pieces[1]!.piece.id,{ ...pass, error_mm:10,pass:false });
    expect(S.blocker(s,'nest')).toBe('tape_check');
    s = S.remove(s,s.pieces[1]!.piece.id);
    expect(S.blocker(s,'nest')).toBeNull();
    const piece = s.pieces[0]!.piece;
    s = { ...s, pieces: s.pieces.map(p => p.piece.id === piece.id ? { ...p, piece:{ ...piece, outline:piece.outline.map(([x,y]) => [x+1,y]) } } : p) };
    expect(S.uncheckedPhotos(s)).toContain('photo-1');
  });
  it('keeps each fabric width and GSM and excludes other fabric groups from the marker', () => {
    let s = fixture(), first = s.pieces[0]!.piece.id;
    s = S.setMaterial(s,first,'Lining');
    s = { ...s, fabric:{ ...s.fabric,widthMm:1500,gsm:180 } };
    s = S.ensureGarments(S.selectMaterial(s,'lining'));
    expect(S.jobPieces(s).map(p => p.id)).toEqual([first]);
    s = { ...s, fabric:{ ...s.fabric,widthMm:1100,gsm:80 } };
    s = S.selectMaterial(s,'main');
    expect(s.fabric.widthMm).toBe(1500); expect(s.fabric.gsm).toBe(180);
    expect(S.jobPieces(s).some(p => p.id === first)).toBe(false);
    s = S.selectMaterial(s,'lining'); expect(s.fabric.gsm).toBe(80);
    expect(S.patternGroups(s).find(g => g.material === 'lining')?.pieces).toHaveLength(1);
  });
  it('saves all group settings without source images, and requires fresh review on reopen', async () => {
    let s = fixture(); s = { ...s,setName:'Uniform shirt',practice:true };
    s = S.setMaterial(s,s.pieces[0]!.piece.id,'fusing');
    s = S.recordTape(s,s.pieces[0]!.piece.id,pass);
    const source = savePatternSet(s), saved = JSON.parse(source);
    expect(saved.photos[0].scan).toBeUndefined(); expect(source).not.toContain('overlayPng');
    const opened = await openPatternSet(source,assist);
    expect(opened.setName).toBe('Uniform shirt'); expect(opened.practice).toBe(true);
    expect(opened.pieces).toHaveLength(s.pieces.length);
    expect(opened.pieces.every(p => p.status === 'pending')).toBe(true);
    expect(opened.tapeChecks).toEqual({}); expect(opened.nest.result).toBeUndefined();
    expect(opened.fabricProfiles.fusing).toBeTruthy();
  });
  it('retains saved pieces when a further photo is merged', async () => {
    const loaded = await openPatternSet(savePatternSet(fixture()),assist);
    const scan = mockScan(new Uint8Array([1]),{ reference:'a4-sheet',photoId:'photo-3' });
    let s = S.photoScanned(S.addPhoto(loaded,'photo-3','new.jpg'),'photo-3',scan);
    const sessions = s.photos.map(p => p.scan ?? { photoId:p.id,pieces:s.pieces.filter(r => r.piece.photoId === p.id).map(r => r.piece),warnings:[] });
    s = S.applySession(s,mockMerge(sessions));
    expect(s.pieces.some(p => p.piece.photoId === 'photo-1')).toBe(true);
    expect(s.pieces.some(p => p.piece.photoId === 'photo-3')).toBe(true);
  });
  it('rejects malformed archives, missing fabric groups and duplicate identities before engine use', async () => {
    const saved = JSON.parse(savePatternSet(fixture()));
    for (const mutate of [
      (v: any) => { v.format = 'other'; },
      (v: any) => { v.pieces[1].id = v.pieces[0].id; },
      (v: any) => { v.pieces[0].photoId = 'missing'; },
      (v: any) => { v.fabrics = {}; },
      (v: any) => { v.fabrics.main.gsm = -10; },
      (v: any) => { v.garments.FREE = 1.2; },
    ]) {
      const v = structuredClone(saved); mutate(v);
      await expect(openPatternSet(JSON.stringify(v),assist)).rejects.toThrow();
    }
    await expect(openPatternSet(savePatternSet(fixture()),async () => { throw new Error('Rust rejected outline'); })).rejects.toThrow('Rust rejected');
  });
  it('blocks invalid fabric values and saves no nonfinite values', () => {
    let s = fixture();
    s = { ...s,fabric:{ ...s.fabric,gsm:NaN } };
    expect(S.blocker(s,'nest')).toBe('fabric_details');
    expect(() => savePatternSet(s)).toThrow('Correct');
    expect(S.setMaterial(s,s.pieces[0]!.piece.id,'__proto__')).toBe(s);
  });
  it('moves to an existing fabric group after the last piece of the selected group is removed', () => {
    let s = fixture();
    s = S.setMaterial(s,s.pieces[0]!.piece.id,'lining');
    const main = s.pieces.filter(p => S.materialOf(p.piece) === 'main');
    for (const p of main) s = S.remove(s,p.piece.id);
    expect(s.material).toBe('lining'); expect(S.jobPieces(s)).toHaveLength(1);
  });
});
