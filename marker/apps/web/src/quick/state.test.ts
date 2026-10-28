import { describe, expect, it } from 'vitest';
import { mockMerge, mockScan } from './mock';
import { recordTape } from './state';
import {
  accept, acceptAllPending, addPhoto, applyChip, applySession, blocker, chips, clearNest, copies, dismissChip, ensureGarments, garmentsFor, go,
  initialState, isPair, jobPieces, nestFailed, nestFinished, nestProgress, nestStarted, orderSizes, photoFailed, photoScanned, remove,
  removePhoto, rename, restore, setFold, shownGrain, setGarments, setPair, setPhotoSize, setQuantity, setSize, setWidth, totalGarments, turnGrain, type QuickState,
} from './state';

const bytes = new Uint8Array([1]);
function scanned(photos = 1): QuickState {
  let s = initialState();
  const scans = [];
  for (let k = 1; k <= photos; k++) {
    const id = `photo-${k}`;
    s = addPhoto(s, id, `IMG_${k}.jpg`);
    const scan = mockScan(bytes, { reference: 'a4-sheet', photoId: id });
    s = photoScanned(s, id, scan); scans.push(scan);
  }
  return applySession(s, mockMerge(scans));
}
const id = (s: QuickState, name: string, photo = 'photo-1') => s.pieces.find(p => p.piece.photoId === photo && p.piece.suggestions.name?.value === name)!.piece.id;

describe('quick flow guards', () => {
  it('opens Start and Capture always and blocks Review until a photo is read', () => {
    let s = initialState();
    expect(blocker(s, 'capture')).toBeNull();
    expect(blocker(s, 'review')).toBe('no_photo');
    s = addPhoto(s, 'photo-1', 'a.jpg');
    expect(blocker(s, 'review')).toBe('reading');
    expect(go(s, 'review').step).toBe('start');
    s = photoFailed(s, 'photo-1', 'Too dark');
    expect(blocker(s, 'review')).toBe('no_photo');
  });
  it('needs every piece decided before the order step', () => {
    let s = scanned();
    expect(blocker(s, 'review')).toBeNull();
    expect(blocker(s, 'order')).toBe('no_pieces');
    s = accept(s, s.pieces[0]!.piece.id);
    expect(blocker(s, 'order')).toBe('pending');
    for (const p of s.pieces.slice(1)) s = remove(s, p.piece.id);
    expect(blocker(s, 'order')).toBeNull();
    expect(go(s, 'order').step).toBe('order');
  });
  it('blocks nesting without a width or garments, and Use without a result', () => {
    let s = ensureGarments(acceptAllPending(scanned()));
    expect(blocker(s,'nest')).toBe('tape_check');
    s = recordTape(s,s.pieces[0]!.piece.id,{ endpoints: [[0,0],[100,0]], measured_mm: 100, digital_mm: 100, tolerance_mm: 3, error_mm: 0, pass: true });
    expect(blocker(s, 'nest')).toBeNull();
    s = setWidth(s, null); expect(blocker(s, 'nest')).toBe('width');
    s = setWidth(s, 1500); s = setGarments(s, 'FREE', 0); expect(blocker(s, 'nest')).toBe('no_garments');
    s = setGarments(s, 'FREE', 2); expect(totalGarments(s)).toBe(2);
    expect(blocker(s, 'use')).toBe('no_result');
  });
});

describe('quick piece review', () => {
  it('gives new pieces the photo size and keeps reviewed pieces when another photo is merged', () => {
    let s = scanned();
    s = setPhotoSize(s, 'photo-1', 'm');
    expect(s.pieces.every(p => p.piece.size === 'M')).toBe(true);
    const front = id(s, 'Front');
    s = rename(accept(s, front), front, 'Front left');
    const scans = [s.photos[0]!.scan!, mockScan(bytes, { reference: 'a4-sheet', photoId: 'photo-2' })];
    s = addPhoto(s, 'photo-2', 'b.jpg'); s = photoScanned(s, 'photo-2', scans[1]!);
    s = applySession(s, mockMerge(scans));
    const kept = s.pieces.find(p => p.piece.id === front)!;
    expect(kept.status).toBe('accepted'); expect(kept.piece.name).toBe('Front left');
    expect(s.pieces.filter(p => p.piece.photoId === 'photo-2').every(p => p.status === 'pending' && p.piece.size === 'M')).toBe(true);
    s = removePhoto(s, 'photo-2');
    expect(s.pieces.some(p => p.piece.photoId === 'photo-2')).toBe(false);
  });
  it('shows suggestions as chips and never applies them on its own', () => {
    const s = scanned();
    const sleeve = id(s, 'Sleeve');
    const piece = s.pieces.find(p => p.piece.id === sleeve)!.piece;
    expect(piece.quantity).toBe(1); expect(piece.mirror_quantity).toBe(0); expect(piece.grain).toBeNull();
    expect(chips(s, sleeve).map(c => c.kind)).toEqual(['name', 'quantity', 'grain']);
  });
  it('applies a name chip and hides it afterwards', () => {
    let s = scanned();
    const sleeve = id(s, 'Sleeve');
    const chip = chips(s, sleeve).find(c => c.kind === 'name')!;
    s = applyChip(s, sleeve, chip);
    expect(s.pieces.find(p => p.piece.id === sleeve)!.piece.name).toBe('Sleeve');
    expect(chips(s, sleeve).some(c => c.kind === 'name')).toBe(false);
  });
  it('accepting a left/right pair keeps one pattern cut 1 + 1 mirrored and removes its mate', () => {
    let s = scanned();
    const a = id(s, 'Front');
    const chip = chips(s, a).find(c => c.kind === 'pair')!;
    expect(chip.other).toBeTruthy();
    s = applyChip(s, a, chip);
    const kept = s.pieces.find(p => p.piece.id === a)!, mate = s.pieces.find(p => p.piece.id === chip.other)!;
    expect([kept.piece.quantity, kept.piece.mirror_quantity]).toEqual([1, 1]);
    expect(mate.status).toBe('deleted'); expect(mate.removedAsPairOf).toBe(a);
    expect(chips(s, chip.other!)).toEqual([]);
    s = restore(s, chip.other!);
    expect(s.pieces.find(p => p.piece.id === chip.other)!.status).toBe('pending');
  });
  it('cut on fold uses the symmetry axis; grain chip and turn work; dismiss hides a chip', () => {
    let s = scanned();
    const back = id(s, 'Back');
    s = applyChip(s, back, chips(s, back).find(c => c.kind === 'fold')!);
    expect(s.pieces.find(p => p.piece.id === back)!.piece.fold_line).toEqual(s.pieces.find(p => p.piece.id === back)!.piece.suggestions.symmetryAxis!.value);
    s = setFold(s, back, false); expect(s.pieces.find(p => p.piece.id === back)!.piece.fold_line).toBeNull();
    const pocket = id(s, 'Pocket');
    expect(setFold(s, pocket, true)).toBe(s);
    s = turnGrain(s, pocket); expect(s.pieces.find(p => p.piece.id === pocket)!.piece.grain).toHaveLength(2);
    const cuff = id(s, 'Cuff');
    s = dismissChip(s, cuff, 'quantity'); expect(chips(s, cuff).some(c => c.kind === 'quantity')).toBe(false);
  });
  it('accepting a piece confirms the grain arrow shown on it', () => {
    let s = scanned();
    const sleeve = id(s, 'Sleeve'), pocket = id(s, 'Pocket');
    const shown = shownGrain(s.pieces.find(p => p.piece.id === sleeve)!.piece);
    expect(shown).toEqual(s.pieces.find(p => p.piece.id === sleeve)!.piece.suggestions.grain!.value);
    s = accept(s, sleeve);
    expect(s.pieces.find(p => p.piece.id === sleeve)!.piece.grain).toEqual(shown);
    s = acceptAllPending(s);
    expect(s.pieces.find(p => p.piece.id === pocket)!.piece.grain).toHaveLength(2);
    expect(jobPieces(ensureGarments(s)).every(p => !!p.grain)).toBe(true);
  });
  it('edits quantity, pair and size with limits', () => {
    let s = scanned();
    const cuff = id(s, 'Cuff');
    const q = () => s.pieces.find(p => p.piece.id === cuff)!.piece;
    s = setQuantity(s, cuff, 0, 0); expect(copies(q())).toBe(1);
    s = setQuantity(s, cuff, 2, 0); expect(copies(q())).toBe(2); expect(isPair(q())).toBe(false);
    s = setPair(s, cuff, true); expect([q().quantity, q().mirror_quantity]).toEqual([1, 1]);
    s = setPair(s, cuff, false); expect([q().quantity, q().mirror_quantity]).toEqual([2, 0]);
    s = setSize(s, cuff, '  xl '); expect(q().size).toBe('XL');
    s = setSize(s, cuff, '   '); expect(q().size).toBe('XL');
  });
  it('offers to remove a duplicate seen in a second photo', () => {
    let s = scanned(2);
    const dup = s.pieces.find(p => p.piece.photoId === 'photo-2' && p.piece.suggestions.name?.value === 'Pocket')!.piece.id;
    const chip = chips(s, dup).find(c => c.kind === 'duplicate')!;
    expect(chip.other).toBe(id(s, 'Pocket'));
    s = applyChip(s, dup, chip);
    expect(s.pieces.find(p => p.piece.id === dup)!.status).toBe('deleted');
  });
});

describe('quick order and nesting state', () => {
  it('orders only sizes that have accepted pieces, in shop order', () => {
    let s = acceptAllPending(scanned());
    s = setSize(s, id(s, 'Cuff'), 'XL'); s = setSize(s, id(s, 'Pocket'), 'S');
    expect(orderSizes(s)).toEqual(['FREE', 'S', 'XL']);
    s = ensureGarments(s);
    expect(garmentsFor(s)).toEqual({ FREE: 1, S: 1, XL: 1 });
    s = setGarments(s, 'XL', 0);
    expect(jobPieces(s).some(p => p.size === 'XL')).toBe(false);
    s = setGarments(s, 'S', 500); expect(s.garments.S).toBe(99);
  });
  it('tracks progress, keeps the best result on stop and clears it on change', () => {
    let s = nestStarted(scanned(), 30);
    expect(s.step).toBe('nest'); expect(s.nest.status).toBe('running');
    s = nestFinished(s, false); expect(s.nest.status).toBe('failed');
    s = nestStarted(s, 30);
    const result = { format: 'r', job_name: 'j', width: 1500, length: 2000, piece_area: 1, efficiency: 0.8, engine: 'sparrow', placements: [] };
    s = nestProgress(s, 4, 2000, 0.8, undefined, result);
    s = nestProgress(s, 6, null, null);
    expect(s.nest.bestLengthMm).toBe(2000); expect(s.nest.result).toBe(result);
    expect(nestFinished(s, true).nest.status).toBe('stopped');
    expect(nestFailed(s, 'Server down').nest.status).toBe('stopped');
    expect(clearNest(s).nest.result).toBeUndefined();
  });
});
