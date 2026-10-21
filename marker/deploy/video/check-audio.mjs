// Focused regression checks: timing, real media import, cleanup and delivery levels.
// Usage: node check-audio.mjs <temporary data folder outside git>
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { cleanVoice, duration, ffmpeg, importRecordings, loudness, normalize, retimeTimeline } from './audio.mjs';
const root = resolve(process.argv[2]); mkdirSync(root, { recursive: true });
let checks = 0;
const check = (fn) => { fn(); checks++; };
const original = { scenes: [{ id: 'test', duration: 5, beats: [{ id: 'test-1', at: .3, dur: 1 }, { id: 'test-2', at: 2.3, dur: 1 }], frames: [{ file: 'a.jpg', t: 0 }, { file: 'b.jpg', t: .3 }, { file: 'c.jpg', t: 2.3 }, { file: 'd.jpg', t: 4.9 }] }] };
const retimed = retimeTimeline(original, { 'test-1': 3, 'test-2': 2 }, root).scenes[0];
check(() => assert.equal(original.scenes[0].duration, 5));
check(() => assert.equal(retimed.beats[1].at, 3.8));
check(() => assert.ok(retimed.duration >= retimed.beats[1].at + 2.5));
check(() => assert.equal(retimed.frames[2].t, retimed.beats[1].at));
check(() => assert.ok(Math.abs(retimed.duration * 30 - Math.round(retimed.duration * 30)) < 1e-9));
check(() => assert.throws(() => retimeTimeline(original, {}, root), /Missing audio/));
const source = join(root, 'test.wav'), imported = join(root, 'imported');
// Two speech-like tones separated by a two-second recording pause.
ffmpeg(['-f', 'lavfi', '-i', 'sine=frequency=220:duration=1:sample_rate=48000', '-f', 'lavfi', '-i', 'anullsrc=r=48000:cl=mono:d=2', '-filter_complex', '[0:a][1:a][0:a]concat=n=3:v=0:a=1[a]', '-map', '[a]', source]);
const defs = [{ id: 'test', beats: [{}, {}] }];
importRecordings(root, imported, defs);
check(() => assert.ok(Math.abs(duration(join(imported, 'test-1.wav')) - 1) < .03));
check(() => assert.ok(Math.abs(duration(join(imported, 'test-2.wav')) - 1) < .03));
writeFileSync(join(root, 'test.json'), '\uFEFF' + JSON.stringify({ beats: [{ id: 'test-1', start: 0, end: 1 }, { id: 'test-2', start: 3, end: 4 }] }));
importRecordings(root, imported, defs);
check(() => assert.equal(duration(join(imported, 'test-2.wav')), 1));
writeFileSync(join(root, 'test.json'), JSON.stringify({ beats: [{ id: 'test-1', start: 0, end: 1 }, { id: 'test-2', start: .5, end: 4 }] }));
check(() => assert.throws(() => importRecordings(root, imported, defs), /overlapping cue/));
const clean = join(root, 'clean.wav'), normalized = join(root, 'normalized.wav');
cleanVoice(join(imported, 'test-1.wav'), clean); normalize(clean, normalized);
const level = loudness(normalized);
check(() => assert.ok(Math.abs(level.lufs + 16) < .2, JSON.stringify(level)));
check(() => assert.ok(level.truePeak <= -1.5, JSON.stringify(level)));
console.log(`${checks} audio checks passed. ${JSON.stringify(level)}`);
