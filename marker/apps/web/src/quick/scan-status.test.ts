import {describe,it,expect} from 'vitest';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import type {QuickPiece,QuickWarning} from '@beta/marker-wasm';
import {scanWarnings} from './scan-status';
describe('real built-in paper sample',()=>{
  it('finds all five parts and their drawn grain lines without quality warnings',()=>{
    const result=JSON.parse(execFileSync(process.execPath,[fileURLToPath(new URL('./sample-scan.mjs',import.meta.url))],{encoding:'utf8',windowsHide:true}));
    expect(result.pieces).toBe(5);expect(result.grainArrows).toBe(5);expect(result.qualityWarnings).toBe(0);
  });
  it('never hides a missing grain line, scale or image-quality problem',()=>{
    const warnings:QuickWarning[]=[{code:'grain_unconfirmed',pieceId:'a',message:'Confirm grain'},{code:'review_required',message:'Review'},{code:'blur',message:'Blur'},{code:'scale_low_confidence',message:'Scale'}];
    expect(scanWarnings(warnings,[]).map(w=>w.code)).toEqual(['grain_unconfirmed','blur','scale_low_confidence']);
    const piece={id:'photo:a',suggestions:{grain:{value:[[0,0],[0,100]]}}} as QuickPiece;
    expect(scanWarnings(warnings,[piece]).map(w=>w.code)).toEqual(['blur','scale_low_confidence']);
  });
});
