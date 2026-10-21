// Render level-matched review copies and measure the same three test lines.
// Usage: node compare-audio.mjs <samples folder>
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { cleanVoice, duration, ffmpeg, loudness, normalize } from './audio.mjs';
const root = resolve(process.argv[2]);
const cases = ['heera-baseline', 'kokoro-heart', 'kokoro-emma', 'piper-ljspeech', 'kokoro-hindi-omega', 'kokoro-hindi-alpha'];
const reference = JSON.parse(readFileSync(join(root, 'kokoro-heart', 'voice.json'), 'utf8'));
const report = [];
for (const name of cases) {
  const dir = join(root, name), voice = JSON.parse(readFileSync(join(dir, 'voice.json'), 'utf8').replace(/^\uFEFF/, ''));
  for (const id of ['01-product', '02-brands', '03-money']) {
    const source = join(dir, `${id}.wav`), clean = join(dir, `${id}.clean.wav`), review = join(dir, `${id}.review.wav`);
    if (!existsSync(source)) throw new Error(`Missing test sample: ${source}`);
    cleanVoice(source, clean); normalize(clean, review);
    const rawPcm = spawnSync('ffmpeg', ['-v', 'error', '-i', source, '-f', 's16le', '-ac', '1', '-ar', '48000', '-'], { windowsHide: true, maxBuffer: 1 << 25 });
    if (rawPcm.status !== 0) throw new Error(rawPcm.stderr.toString());
    let clipping = 0;
    for (let i = 0; i < rawPcm.stdout.length; i += 2) if (Math.abs(rawPcm.stdout.readInt16LE(i)) >= 32767) clipping++;
    const metadata = voice.lines?.find((x) => x.id === id);
    const words = metadata?.words ?? reference.lines.find((x) => x.id === id).words;
    report.push({ name, id, words, seconds: +duration(clean).toFixed(3), wpm: +(words * 60 / duration(clean)).toFixed(1), pcmClippedSamples: clipping, raw: loudness(source), review: loudness(review), reviewFile: review, tempo: metadata?.tempo ?? null, modelPeakDbfs: metadata?.modelPeakDbfs ?? null });
  }
  const list = join(dir, 'review.ffconcat');
  writeFileSync(list, ['ffconcat version 1.0', ...report.filter((x) => x.name === name).map((x) => `file '${x.reviewFile.replaceAll('\\', '/')}'`)].join('\n'));
  ffmpeg(['-f', 'concat', '-safe', '0', '-i', list, '-c', 'copy', join(dir, 'listen-first.wav')]);
}
writeFileSync(join(root, 'comparison.json'), JSON.stringify(report, null, 2));
console.log(JSON.stringify(cases.map((name) => {
  const rows = report.filter((x) => x.name === name);
  return { name, seconds: +rows.reduce((n, r) => n + r.seconds, 0).toFixed(2), wpm: +(rows.reduce((n, r) => n + r.words, 0) * 60 / rows.reduce((n, r) => n + r.seconds, 0)).toFixed(1), rawLufs: rows.map((r) => r.raw.lufs), reviewLufs: rows.map((r) => r.review.lufs), maxTruePeak: Math.max(...rows.map((r) => r.review.truePeak)), clipping: rows.reduce((n, r) => n + r.pcmClippedSamples, 0) };
}), null, 2));
