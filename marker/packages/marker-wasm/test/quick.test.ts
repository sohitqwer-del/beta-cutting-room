import { describe, expect, it } from 'vitest';
import { bindQuickMarkerCore, type QuickPiece, type QuickScan, type QuickScanOptions } from '../src/quick';
import type { Job, MarkerResult } from '../src/types';

describe('Quick Marker typed wire contract', () => {
  const calls: Record<string, unknown[]> = {};
  const wire = {
    quick_scan(photo: Uint8Array, options: string) {
      calls.scan = [photo, JSON.parse(options)];
      return JSON.stringify({photoId:'a', pieces:[], overlayPng:[137,80,78,71], warnings:[],
        scale:{mmPerPx:0.5, confidence:'low', method:'a4-sheet', expectedErrorMm:3,
          tiltScore:0.2, referenceCorners:[], imageToMm:[1,0,0,0,-1,0,0,0,1]},
        sourceSize:[4000,3000], processedSize:[2000,1500], runtimeMs:0});
    },
    quick_assist(pieces: string) { calls.assist = [JSON.parse(pieces)]; return pieces; },
    quick_merge(scans: string) { calls.merge = [JSON.parse(scans)]; return '{"pieces":[],"warnings":[],"duplicates":[]}'; },
    quick_pieces_to_job(pieces: string, fabric: string, garments: string) {
      calls.job = [JSON.parse(pieces), JSON.parse(fabric), JSON.parse(garments)]; return '{"format":"beta-marker.job/v1"}';
    },
    tiled_marker_pdf(job: string, result: string, options: string) {
      calls.tiles = [JSON.parse(job), JSON.parse(result), JSON.parse(options)]; return new Uint8Array([37,80,68,70]);
    },
    chalk_guide_pdf(job: string, result: string, options: string) {
      calls.chalk = [JSON.parse(job), JSON.parse(result), JSON.parse(options)]; return new Uint8Array([37,80,68,70]);
    },
    projector_layout() { return '{"format":"beta-marker.projector/v1","units":"mm"}'; },
  };
  const core = bindQuickMarkerCore(wire);
  it('converts binary PNG without changing geometry or options', () => {
    const photo = new Uint8Array([255,216,255]);
    const options: QuickScanOptions = { reference:'a4-sheet', photoId:'a', useSecondSheet:true };
    const scan = core.quickScan(photo, options);
    expect(scan.overlayPng).toBeInstanceOf(Uint8Array);
    expect([...scan.overlayPng]).toEqual([137,80,78,71]);
    expect(calls.scan).toEqual([photo, options]);
    expect(scan.runtimeMs).toBeGreaterThanOrEqual(0);
  });
  it('does not send PNG bytes back through JSON when merging', () => {
    const scan = core.quickScan(new Uint8Array(), {reference:'beta-target'});
    core.quickMerge([scan]);
    expect(calls.merge).toEqual([[{photoId:'a', pieces:[], warnings:[]}]]);
  });
  it('passes accepted quantities and explicit sizes without applying suggestions', () => {
    const pieces = [{id:'a:p',name:'Panel',outline:[[0,0],[100,0],[0,100]],size:'M',quantity:1,
      mirror_quantity:0,photoId:'a',expectedErrorMm:1,notches:[],drills:[],
      suggestions:{pair:'mirror',quantity:{value:{normal:1,mirrored:1},confidence:'medium',reason:'test'}}}] satisfies QuickPiece[];
    core.quickPiecesToJob(pieces, {width_mm:1500}, {M:2});
    expect(calls.job).toEqual([pieces,{width_mm:1500},{M:2}]);
    expect(core.quickAssist(pieces)[0]?.quantity).toBe(1);
  });
  it('preserves PDF bytes and the print options', () => {
    const job = {format:'beta-marker.job/v1'} as Job;
    const result = {format:'beta-marker.result/v1'} as MarkerResult;
    expect(core.tiledMarkerPdf(job,result,{paper:'A3',overlap_mm:15})).toEqual(new Uint8Array([37,80,68,70]));
    expect(calls.tiles?.[2]).toEqual({paper:'A3',overlap_mm:15});
    core.chalkGuidePdf(job,result);
    expect(calls.chalk?.[2]).toEqual({});
    expect(core.projectorLayout(job,result).units).toBe('mm');
  });
  it('propagates engine failures rather than returning a partial scan', () => {
    const failing = bindQuickMarkerCore({...wire,quick_scan(){throw new Error('No clear A4 sheet found');}});
    expect(()=>failing.quickScan(new Uint8Array(),{reference:'a4-sheet'})).toThrow('No clear A4');
  });
});
