// Shared offline voice processing, for neural speech and phone recordings.
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
export function ffmpeg(args, level = 'error') {
  const r = spawnSync('ffmpeg', ['-hide_banner', '-v', level, '-y', ...args], { encoding: 'utf8', windowsHide: true, maxBuffer: 1 << 26 });
  if (r.status !== 0) throw new Error(`ffmpeg: ${r.stderr}`);
  return r.stderr;
}
export function duration(file) {
  const r = spawnSync('ffprobe', ['-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', file], { encoding: 'utf8', windowsHide: true });
  const n = Number(r.stdout?.trim());
  if (r.status !== 0 || !Number.isFinite(n) || n <= 0) throw new Error(`Cannot measure ${file}: ${r.stderr}`);
  return n;
}
export function measure(file) {
  const log = ffmpeg(['-i', file, '-af', 'loudnorm=I=-16:TP=-1.8:LRA=11:print_format=json', '-f', 'null', '-'], 'info');
  const m = JSON.parse(log.slice(log.lastIndexOf('{'), log.lastIndexOf('}') + 1));
  if (![m.input_i, m.input_tp].every((n) => Number.isFinite(Number(n)))) throw new Error(`Silent or invalid audio: ${file}`);
  return m;
}
export function loudness(file) {
  const m = measure(file);
  return { lufs: +m.input_i, truePeak: +m.input_tp };
}
export function normalize(source, destination, { aac = false, fade = false } = {}) {
  const m = measure(source);
  // Leave 0.3 dB beyond the delivery ceiling to allow AAC reconstruction overshoot.
  const filter = `${fade ? 'afade=t=in:d=0.03,' : ''}loudnorm=I=-16:TP=-1.8:LRA=11:measured_I=${m.input_i}:measured_TP=${m.input_tp}:measured_LRA=${m.input_lra}:measured_thresh=${m.input_thresh}:offset=${m.target_offset}:linear=true,aresample=48000`;
  ffmpeg(['-i', source, ...(aac ? ['-map', '0:v:0', '-map', '0:a:0', '-c:v', 'copy', '-c:a', 'aac', '-b:a', '128k', '-movflags', '+faststart'] : ['-ac', '2']), '-af', filter, destination]);
}
export const CLEAN_CHAIN = 'highpass=f=80,equalizer=f=300:t=q:w=0.8:g=-1.0,equalizer=f=3000:t=q:w=0.7:g=1.2,agate=threshold=0.003:ratio=1.4:range=0.5:attack=15:release=180,deesser=i=0.12:m=0.3:f=0.5,acompressor=threshold=0.125:ratio=2:attack=20:release=180:makeup=1.1';
export function cleanVoice(source, destination) {
  // Remove only edge silence; internal sentence pauses survive the cleaning chain.
  ffmpeg(['-i', source, '-af', `silenceremove=start_periods=1:start_threshold=-48dB:start_silence=0.035,areverse,silenceremove=start_periods=1:start_threshold=-48dB:start_silence=0.065,areverse,${CLEAN_CHAIN},aresample=48000`, '-ac', '1', destination]);
  if (duration(destination) < 0.15) throw new Error(`Empty narration after cleaning: ${source}`);
}

/** One phone recording per scene. Two-second pauses delimit beats, or a sidecar
 * provides precise cues. Captions and actions then use these actual beat lengths.
 */
export function importRecordings(folder, output, scenes) {
  mkdirSync(output, { recursive: true });
  const metadata = [];
  for (const scene of scenes) {
    const source = ['wav', 'm4a', 'mp3', 'flac'].map((ext) => join(folder, `${scene.id}.${ext}`)).find(existsSync);
    if (!source) throw new Error(`Missing recording ${scene.id}.wav (or .m4a/.mp3/.flac)`);
    const seconds = duration(source), sidecar = join(folder, `${scene.id}.json`);
    let cues;
    if (existsSync(sidecar)) cues = JSON.parse(readFileSync(sidecar, 'utf8').replace(/^\uFEFF/, '')).beats;
    else {
      const log = ffmpeg(['-i', source, '-af', 'silencedetect=n=-36dB:d=1.2', '-f', 'null', '-'], 'info');
      const silences = [...log.matchAll(/silence_(start|end): ([\d.]+)/g)];
      const spans = []; let start = 0;
      for (const [, type, time] of silences) {
        const t = Number(time);
        if (type === 'start' && t - start > .15) spans.push({ start, end: t });
        if (type === 'end') start = t;
      }
      const last = silences.at(-1);
      if ((!last || last[1] === 'end') && seconds - start > .15) spans.push({ start, end: seconds });
      if (spans.length !== scene.beats.length) throw new Error(`${scene.id}: found ${spans.length} phrases, expected ${scene.beats.length}. Use two-second pauses or supply ${scene.id}.json; see RECORD-YOUR-VOICE.md.`);
      cues = spans.map((span, i) => ({ id: `${scene.id}-${i + 1}`, ...span }));
    }
    if (!Array.isArray(cues) || cues.length !== scene.beats.length) throw new Error(`Wrong cue count in ${sidecar}`);
    let previousEnd = 0;
    cues.forEach((cue, i) => {
      const id = `${scene.id}-${i + 1}`;
      if (cue.id !== id || !Number.isFinite(cue.start) || !Number.isFinite(cue.end) || cue.start < previousEnd || cue.end <= cue.start || cue.end > seconds + .02) throw new Error(`Invalid or overlapping cue ${id}`);
      ffmpeg(['-i', source, '-ss', cue.start.toFixed(6), '-t', (cue.end - cue.start).toFixed(6), '-ac', '1', '-ar', '48000', join(output, `${id}.wav`)]);
      previousEnd = cue.end;
    });
    metadata.push({ scene: scene.id, source, cues });
  }
  writeFileSync(join(output, 'voice.json'), JSON.stringify({ engine: 'human recording', scenes: metadata }, null, 2));
}

/** Reuse original, synthetic-only captured frames. Each beat interval can grow
 * independently. The next action, caption and voice move together. Source frames
 * are never deleted or overwritten; the new timeline refers to the source dir.
 */
export function retimeTimeline(timeline, durations, sourceFrames) {
  const result = structuredClone(timeline);
  for (const scene of result.scenes) {
    const original = structuredClone(scene), intervals = [];
    let at = original.beats[0]?.at ?? 0;
    scene.beats = original.beats.map((beat, i) => {
      const end = original.beats[i + 1]?.at ?? original.duration;
      const dur = durations[beat.id];
      if (!Number.isFinite(dur) || dur <= 0) throw new Error(`Missing audio duration: ${beat.id}`);
      const length = Math.max(end - beat.at, dur + .5);
      intervals.push({ from: beat.at, to: end, at, length });
      const mapped = { ...beat, at, dur }; at += length;
      return mapped;
    });
    scene.duration = Math.ceil(at * 30) / 30;
    scene.frames = original.frames.map((frame) => {
      const span = intervals.find((x) => frame.t >= x.from && frame.t < x.to);
      return { ...frame, t: span ? span.at + (frame.t - span.from) * span.length / (span.to - span.from) : frame.t };
    });
    scene.frameDir = original.frameDir ?? join(sourceFrames, scene.id);
  }
  result.retimed = { method: 'per-beat intervals; never speed up actions; frame-aligned scene ends', audioDurations: durations };
  return result;
}
