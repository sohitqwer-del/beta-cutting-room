// Builds the BETA demo video from scenes.mjs: narration, recording, editing, captions and all deliverables.
// 0.8 launch outputs use launch8.mjs (see README); this editor preserves earlier release reproducibility.
// Usage (from the repository root):
//   node marker/deploy/video/make-video.mjs <step> --out <output dir> [--app http://127.0.0.1:7878/] [--playwright <dir>]
// Steps: tts | record | retime | build | stills | check | all  (all = tts, record, build, stills, check)
// Needs: a local neural voice or phone recordings, ffmpeg/ffprobe, Chrome or Edge, playwright-core (outside the repo),
// and a running `beta-marker serve` built from this checkout for the record step. See README.md.
import { execFileSync, spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { allBeats, clips, GAP, scenes, SITE_URL } from './scenes.mjs';
import { cleanVoice, importRecordings, loudness, normalize, retimeTimeline } from './audio.mjs';
import { explainerImage, page, PHONE_BOX, phoneBackground, verticalBackground, verticalPhoneBackground, VBOX, VPHONE, LOGO, C } from './cards.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const args = Object.fromEntries(process.argv.slice(3).reduce((a, v, i, all) => (v.startsWith('--') ? [...a, [v.slice(2), all[i + 1]?.startsWith('--') || all[i + 1] === undefined ? '1' : all[i + 1]]] : a), []));
const step = process.argv[2] ?? 'all';
const out = resolve(args.out ?? 'demo-video');
const language = args.lang ?? 'en';
if (!['en', 'hi'].includes(language)) throw new Error('Supported narration languages: en, hi');
const work = join(out, language === 'hi' ? 'work-hi' : 'work');
const FPS = 30;
const VOICE = args.voice ?? 'en-IN', VOICE_NAME = args['voice-name'] ?? 'Microsoft Heera', RATE = args.rate ?? '1.12';
const playwright = resolve(args.playwright ?? process.env.PLAYWRIGHT_CORE ?? 'node_modules/playwright-core');
const chrome = args.chrome ?? process.env.CHROME_PATH ?? 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const music = args.music === '1';
mkdirSync(work, { recursive: true });

const ff = (a, opts = {}) => { const r = spawnSync('ffmpeg', ['-hide_banner', '-v', 'error', '-y', '-threads', '3', '-filter_threads', '2', '-filter_complex_threads', '2', ...a], { encoding: 'utf8', maxBuffer: 1 << 26, windowsHide: true, ...opts }); if (r.status !== 0) throw new Error(`ffmpeg failed: ${r.stderr}\n${a.join(' ')}`); return r.stderr; };
const probe = (f) => Number(execFileSync('ffprobe', ['-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', f], { encoding: 'utf8', windowsHide: true }).trim());
const mb = (f) => (statSync(f).size / 1048576).toFixed(1);
const fwd = (p) => p.replaceAll('\\', '/');
const esc = (p) => fwd(p).replace(/:/g, '\\:').replace(/'/g, "\\'");

// ---------------- TTS ----------------
function tts() {
  const dir = join(work, 'tts'), raw = join(dir, 'raw'); mkdirSync(raw, { recursive: true });
  const lines = join(work, 'lines.json');
  writeFileSync(lines, JSON.stringify(allBeats().map((b) => ({ id: b.id, say: language === 'hi' ? b.hi : b.say }))));
  if (args['voice-dir']) importRecordings(resolve(args['voice-dir']), raw, scenes);
  else {
    const engine = args.engine ?? 'kokoro';
    if (language === 'hi' && engine !== 'kokoro') throw new Error('Hindi needs Kokoro with the curated draft phonemes, or --voice-dir recordings.');
    let command, params;
    if (engine === 'onecore') {
      command = 'powershell';
      params = ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', join(here, 'tts.ps1'), '-Lines', lines, '-OutDir', raw, '-Voice', VOICE, '-VoiceName', VOICE_NAME, '-Rate', RATE];
    } else {
      if (!['kokoro', 'piper'].includes(engine)) throw new Error(`Unknown voice engine: ${engine}`);
      const models = args.models ?? process.env.NARRATION_MODELS;
      if (!models) throw new Error('Pass --models <local model folder>. See AUDIO-SOURCES.md; inference never downloads files.');
      command = args.python ?? process.env.NARRATION_PYTHON ?? 'python';
      params = [join(here, 'neural-tts.py'), '--models', resolve(models), '--lines', lines, '--out', raw, '--engine', engine, '--voice', args['neural-voice'] ?? (language === 'hi' ? 'hm_omega' : 'af_heart'), '--wpm', args.wpm ?? '145', ...(language === 'hi' ? ['--phonemes', join(here, 'hindi-phonemes.json')] : [])];
    }
    const r = spawnSync(command, params, { stdio: 'inherit', windowsHide: true });
    if (r.status !== 0) throw new Error(`Narration failed: ${r.error ?? r.status}`);
  }
  const durations = {};
  for (const b of allBeats()) {
    const f = join(dir, `${b.id}.wav`);
    cleanVoice(join(raw, `${b.id}.wav`), f);
    durations[b.id] = +probe(f).toFixed(3);
  }
  writeFileSync(join(dir, 'durations.json'), JSON.stringify(durations, null, 1));
  copyFileSync(join(raw, 'voice.json'), join(dir, 'voice.json'));
  console.log(`Narration: ${Object.keys(durations).length} lines, ${Object.values(durations).reduce((a, b) => a + b, 0).toFixed(1)} s of speech.`);
}
function retime() {
  if (!args.source) throw new Error('retime needs --source <existing output folder>; it reads the original timeline and frames.');
  const source = resolve(args.source);
  if (source === out) throw new Error('Retiming needs a separate output folder.');
  const timeline = JSON.parse(readFileSync(join(source, 'work', 'timeline.json'), 'utf8'));
  const durations = JSON.parse(readFileSync(join(work, 'tts', 'durations.json'), 'utf8'));
  const result = retimeTimeline(timeline, durations, join(source, 'work', 'frames'));
  writeFileSync(join(work, 'timeline.json'), JSON.stringify(result));
  console.log(`Retimed ${result.scenes.length} scenes from existing synthetic footage.`);
}

// ---------------- Record ----------------
function record() {
  const r = spawnSync(process.execPath, [join(here, 'record.mjs'), '--app', args.app ?? 'http://127.0.0.1:7878/', '--work', work, '--playwright', playwright, '--chrome', chrome, ...(args.only ? ['--only', args.only] : [])], { stdio: 'inherit', windowsHide: true });
  if (r.status !== 0) throw new Error('Recording reported problems. See work/record-report.json.');
}

// ---------------- Stills (HTML rendered to PNG) ----------------
async function renderHtml(jobs) {
  const pw = await import(pathToFileURL(playwright).href + '/index.mjs');
  const browser = await pw.chromium.launch({ executablePath: chrome, headless: true });
  try {
    for (const { html, file, w, h } of jobs) {
      const p = await browser.newPage({ viewport: { width: w, height: h }, deviceScaleFactor: 1 });
      await p.setContent(html); await p.evaluate(() => document.fonts.ready);
      await p.evaluate(() => window.beat?.(9)); await p.waitForTimeout(2500);
      await p.screenshot({ path: file }); await p.close();
    }
  } finally { await browser.close(); }
}
async function backgrounds() {
  const dir = join(work, 'bg'); mkdirSync(dir, { recursive: true });
  const titles = { basics: 'The basics', marker: 'Marker', cutplan: 'Cut plan', floor: 'Rolls and bundles', pattern: 'Pattern', proof: 'Honest numbers', problem: 'The problem', feedback: 'Feedback' };
  await renderHtml([
    { html: phoneBackground(), file: join(dir, 'phone.png'), w: 1920, h: 1080 },
    { html: verticalPhoneBackground(), file: join(dir, 'v-phone.png'), w: 1080, h: 1920 },
    ...Object.entries(titles).map(([k, t]) => ({ html: verticalBackground(t), file: join(dir, `v-${k}.png`), w: 1080, h: 1920 })),
  ]);
}

// ---------------- Captions ----------------
const assTime = (t) => { const cs = Math.round(Math.max(0, t) * 100); const h = Math.floor(cs / 360000), m = Math.floor(cs / 6000) % 60, s = Math.floor(cs / 100) % 60; return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}.${String(cs % 100).padStart(2, '0')}`; };
const srtTime = (t) => { const ms = Math.round(Math.max(0, t) * 1000); const h = Math.floor(ms / 3600000), m = Math.floor(ms / 60000) % 60, s = Math.floor(ms / 1000) % 60; return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')},${String(ms % 1000).padStart(3, '0')}`; };
const assText = (s) => s.replace(/\{/g, '(').replace(/\}/g, ')').replace(/\n/g, '\\N');
/** Balanced 2-line wrap for long captions (libass would make a long first line and a short second line). */
function wrap2(text, max) {
  if (text.length <= max) return text;
  const words = text.split(' '); let best = null;
  for (let i = 1; i < words.length; i++) {
    const a = words.slice(0, i).join(' '), b = words.slice(i).join(' ');
    const score = Math.max(a.length, b.length);
    if (!best || score < best.score) best = { score, t: `${a}\n${b}` };
  }
  return best.t;
}
const ASS_HEAD = (w, h, styles) => `[Script Info]\nScriptType: v4.00+\nPlayResX: ${w}\nPlayResY: ${h}\nWrapStyle: 0\nScaledBorderAndShadow: yes\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n${styles.join('\n')}\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n`;
// Colours are &HAABBGGRR. Box = ink #10231c at ~12% transparency; chip = cutting green #176b52.
const style = (name, size, align, ml, mr, mv, box = '&H1E1C2310') => `Style: ${name},Segoe UI Semibold,${size},&H00FFFFFF,&H00FFFFFF,${box},&H00000000,1,0,0,0,100,100,0,0,3,16,0,${align},${ml},${mr},${mv},1`;

/** Caption events for a sequence of scenes laid end to end. */
function captionEvents(seq, lang = 'en') {
  const beats = Object.fromEntries(allBeats().map((b) => [b.id, b]));
  const ev = [];
  for (const { s, offset, from = 0, to = s.duration } of seq) {
    for (const [i, b] of s.beats.entries()) {
      if (b.at < from - 0.01 || b.at >= to) continue;
      const next = s.beats[i + 1]?.at ?? s.duration;
      const start = offset + b.at - from, end = offset + Math.min(b.at + b.dur + 0.6, next - 0.05, to) - from;
      const beat = beats[b.id];
      ev.push({ start, end, text: lang === 'en' ? beat.en : beat[lang], scene: s.scene ?? s.id, beat: b.id });
    }
  }
  return ev;
}
function writeSrt(file, ev) { writeFileSync(file, ev.map((e, i) => `${i + 1}\n${srtTime(e.start)} --> ${srtTime(e.end)}\n${wrap2(e.text, 48)}\n`).join('\n'), 'utf8'); }
function writeVtt(file, ev, note) { writeFileSync(file, `WEBVTT\n\n${note ? `NOTE ${note}\n\n` : ''}${ev.map((e) => `${srtTime(e.start).replace(',', '.')} --> ${srtTime(e.end).replace(',', '.')}\n${wrap2(e.text, 48)}\n`).join('\n')}`, 'utf8'); }

// ---------------- Scene encoding ----------------
function concatList(s, dir) {
  const fr = s.frames; if (!fr.length) throw new Error(`No frames for ${s.id}`);
  const lines = ['ffconcat version 1.0'];
  for (let i = 0; i < fr.length; i++) {
    const start = i === 0 ? 0 : fr[i].t;
    const end = i + 1 < fr.length ? fr[i + 1].t : s.duration;
    const d = Math.max(0.001, end - start);
    lines.push(`file '${fwd(join(dir, fr[i].file))}'`, `duration ${d.toFixed(4)}`);
  }
  lines.push(`file '${fwd(join(dir, fr[fr.length - 1].file))}'`);
  const f = join(work, 'lists', `${s.id}.ffconcat`); mkdirSync(dirname(f), { recursive: true }); writeFileSync(f, lines.join('\n'));
  return f;
}
const X264_MID = ['-c:v', 'libx264', '-threads', '3', '-preset', 'veryfast', '-crf', '12', '-pix_fmt', 'yuv420p', '-r', String(FPS)];
/** Raw scene video at its capture size, and a 1920 x 1080 composed version. */
function encodeScene(s) {
  const dir = s.frameDir ?? join(work, 'frames', s.id), list = concatList(s, dir);
  const raw = join(work, 'scenes', `${s.id}.raw.mp4`), comp = join(work, 'scenes', `${s.id}.mp4`);
  mkdirSync(dirname(raw), { recursive: true });
  ff(['-f', 'concat', '-safe', '0', '-i', list, '-vf', `fps=${FPS},scale=trunc(iw/2)*2:trunc(ih/2)*2:flags=lanczos`, '-t', s.duration.toFixed(3), ...X264_MID, raw]);
  if (s.take === 'phone') {
    const b = PHONE_BOX;
    ff(['-loop', '1', '-framerate', String(FPS), '-i', join(work, 'bg', 'phone.png'), '-i', raw, '-filter_complex', `[1:v]scale=${b.w}:${b.h}:flags=lanczos[p];[0:v][p]overlay=${b.x}:${b.y}:shortest=1,format=yuv420p`, '-t', s.duration.toFixed(3), ...X264_MID, comp]);
  } else if (s.take !== 'vcard') copyFileSync(raw, comp);
  return { raw, comp };
}

// ---------------- Audio ----------------
/** Narration placed at beat times; optional soft self-made pad, ducked under the voice. */
function narration(events, total, file, withMusic = music) {
  const inputs = [], filters = [];
  events.forEach((e, i) => { inputs.push('-i', join(work, 'tts', `${e.beat}.wav`)); filters.push(`[${i}:a]adelay=${Math.round(e.start * 1000)}:all=1[a${i}]`); });
  const mix = `${events.map((_, i) => `[a${i}]`).join('')}amix=inputs=${events.length}:normalize=0,apad,atrim=0:${total.toFixed(3)}`;
  const voice = join(work, `${file}.voice.wav`);
  ff([...inputs, '-filter_complex', `${filters.join(';')};${mix}[v]`, '-map', '[v]', '-ar', '48000', '-ac', '1', voice]);
  let pre = voice;
  if (withMusic) {
    // Pad: a soft A-major chord from sine tones with slow tremolo, low-passed. Made here; no downloaded music.
    const notes = [110, 164.81, 220, 277.18, 329.63];
    const src = notes.map((f, i) => `sine=f=${f}:d=${total.toFixed(2)}:r=48000,volume=${(0.22 / (i + 1)).toFixed(3)},tremolo=f=${(0.13 + i * 0.03).toFixed(2)}:d=0.5[n${i}]`).join(';');
    const pad = `${src};${notes.map((_, i) => `[n${i}]`).join('')}amix=inputs=${notes.length}:normalize=0,lowpass=f=900,volume=0.11,afade=t=in:d=2,afade=t=out:st=${Math.max(0, total - 3).toFixed(2)}:d=3[pad]`;
    const mixed = join(work, `${file}.mix.wav`);
    ff(['-i', voice, '-filter_complex', `${pad};[0:a]asplit=2[v1][v2];[pad][v2]sidechaincompress=threshold=0.02:ratio=6:attack=20:release=400[duck];[v1][duck]amix=inputs=2:normalize=0[m]`, '-map', '[m]', '-ar', '48000', '-ac', '1', mixed]);
    pre = mixed;
  }
  const outWav = join(work, `${file}.audio.wav`);
  normalize(pre, outWav);
  return outWav;
}
/** Measure delivery files after AAC encoding, and correct codec overshoot. */
function verifyAudio(file) {
  for (let pass = 0; pass < 3; pass++) {
    const m = loudness(file);
    if (Math.abs(m.lufs + 16) <= .2 && m.truePeak <= -1.5) return m;
    const temporary = file.replace(/\.mp4$/, '.normalizing.mp4');
    normalize(file, temporary, { aac: true });
    copyFileSync(temporary, file); rmSync(temporary);
  }
  throw new Error(`Delivery audio outside limits: ${file}: ${JSON.stringify(loudness(file))}`);
}

const X264_FINAL = (crf) => ['-c:v', 'libx264', '-threads', '3', '-preset', args.preset ?? 'slow', '-crf', String(crf), '-profile:v', 'high', '-level', '4.1', '-pix_fmt', 'yuv420p', '-r', String(FPS), '-g', String(FPS * 2), '-movflags', '+faststart'];
const AAC = ['-c:a', 'aac', '-b:a', '128k', '-ar', '48000'];

// ---------------- Build ----------------
async function build() {
  const tl = JSON.parse(readFileSync(join(work, 'timeline.json'), 'utf8'));
  const byId = Object.fromEntries(tl.scenes.map((s) => [s.id, s]));
  await backgrounds();
  console.log('Encoding scenes...');
  for (const s of tl.scenes) encodeScene(s);

  // ---- Main 16:9 video ----
  const order = scenes.map((s) => byId[s.id]).filter(Boolean);
  let t = 0; const seq = order.map((s) => { const o = { s, offset: t }; t += s.duration; return o; });
  const total = t;
  const list = join(work, 'lists', 'main.ffconcat');
  writeFileSync(list, ['ffconcat version 1.0', ...order.map((s) => `file '${fwd(join(work, 'scenes', `${s.id}.mp4`))}'`)].join('\n'));
  const rawMain = join(work, 'main.video.mp4');
  ff(['-f', 'concat', '-safe', '0', '-i', list, '-c', 'copy', rawMain]);
  const ev = captionEvents(seq, language);
  const sceneDef = Object.fromEntries(scenes.map((s) => [s.id, s]));
  const capStyle = (sceneId) => ({ marker: 'CapTop', pattern: 'CapTop', phone: 'CapPhone' })[sceneDef[sceneId].take === 'phone' ? 'phone' : sceneId] ?? 'Cap';
  const assLines = ev.map((e) => `Dialogue: 0,${assTime(e.start)},${assTime(e.end)},${capStyle(e.scene)},,0,0,0,,${assText(wrap2(e.text, capStyle(e.scene) === 'CapPhone' ? 30 : 44))}`);
  for (const { s, offset } of seq) {
    const def = sceneDef[s.id];
    if (language === 'hi' || !def.section || def.take === 'card') continue;
    const chip = capStyle(s.id) === 'CapTop' ? 'ChipLow' : 'Chip';
    assLines.push(`Dialogue: 1,${assTime(offset + 0.2)},${assTime(offset + 4.2)},${chip},,0,0,0,,{\\fad(250,300)}${assText(def.section)}`);
  }
  if (language === 'hi') assLines.push(`Dialogue: 2,0:00:00.00,${assTime(total)},Review,,0,0,0,,Hindi script needs native review`);
  const ass = join(work, 'main.ass');
  writeFileSync(ass, ASS_HEAD(1920, 1080, [style('Cap', 70, 2, 120, 120, 40), style('CapTop', 70, 8, 120, 120, 24), style('CapPhone', 62, 2, 850, 60, 40), style('Chip', 44, 7, 34, 34, 30, '&H00526B17'), style('ChipLow', 44, 1, 34, 34, 30, '&H00526B17'), style('Review', 26, 9, 30, 30, 4)]) + assLines.join('\n') + '\n', 'utf8');
  if (language === 'hi') writeFileSync(ass, readFileSync(ass, 'utf8').replaceAll('Segoe UI Semibold', 'Nirmala UI'));
  console.log(`Main timeline: ${total.toFixed(1)} s. Mixing audio...`);
  const audio = narration(ev, total, 'main');
  const main = join(out, `beta-demo-${language}.mp4`);
  ff(['-i', rawMain, '-i', audio, '-filter_complex', `[0:v]subtitles='${esc(ass)}'[v]`, '-map', '[v]', '-map', '1:a', ...X264_FINAL(20), ...AAC, '-shortest', '-metadata', 'title=BETA Cutting Room - demo', main]);
  writeSrt(join(out, `beta-demo-${language}.srt`), ev);
  writeVtt(join(out, `beta-demo-${language}.vtt`), ev, language === 'hi' ? 'Hindi script needs native review.' : undefined);
  if (language === 'hi') {
    const info = { total: +total.toFixed(2), main: { file: main, mb: +mb(main), loudness: verifyAudio(main) }, review: 'Hindi script needs native review', voice: JSON.parse(readFileSync(join(work, 'tts', 'voice.json'), 'utf8')), scenes: seq.map(({s, offset}) => ({id:s.id, start:offset, seconds:s.duration})) };
    writeFileSync(join(out, 'build-info-hi.json'), JSON.stringify(info, null, 2));
    console.log(JSON.stringify({total: info.total, main: info.main, review: info.review}, null, 2));
    return;
  }
  writeSrt(join(out, 'beta-demo-hi.draft-needs-native-review.srt'), captionEvents(seq, 'hi'));
  writeSrt(join(out, 'beta-demo-ta.draft-needs-native-review.srt'), captionEvents(seq, 'ta'));
  writeVtt(join(out, 'beta-demo-hi.draft-needs-native-review.vtt'), captionEvents(seq, 'hi'), 'Hindi draft translation. Needs review by a native speaker.');
  writeVtt(join(out, 'beta-demo-ta.draft-needs-native-review.vtt'), captionEvents(seq, 'ta'), 'Tamil draft translation. Needs review by a native speaker.');
  // Web copy for the website (must stay under 25 MB to be self-hosted on the site).
  const web = join(out, 'beta-demo-en.web.mp4');
  if (statSync(main).size < 24 * 1048576) copyFileSync(main, web);
  else ff(['-i', main, ...X264_FINAL(27), '-maxrate', '700k', '-bufsize', '1400k', '-c:a', 'copy', web]);

  // ---- Feature clips for the Help page ----
  const clipDir = join(out, 'clips'); mkdirSync(clipDir, { recursive: true });
  const clipInfo = {};
  for (const [name, ids] of Object.entries(clips())) {
    const parts = seq.filter((x) => ids.includes(x.s.id));
    const a = parts[0].offset, b = parts.at(-1).offset + parts.at(-1).s.duration;
    const f = join(clipDir, `beta-${name}.mp4`);
    ff(['-ss', a.toFixed(3), '-to', b.toFixed(3), '-i', main, ...X264_FINAL(21), ...AAC, '-af', 'afade=t=in:d=0.3', f]);
    clipInfo[name] = { file: f, seconds: +(b - a).toFixed(1) };
  }

  // ---- 60-second vertical teaser ----
  await teaser(byId);

  const info = { total: +total.toFixed(1), main: { file: main, mb: +mb(main), loudness: loudness(main) }, web: { file: web, mb: +mb(web) }, clips: clipInfo, scenes: seq.map(({ s, offset }) => ({ id: s.id, start: +offset.toFixed(2), seconds: +s.duration.toFixed(2) })) };
  info.voice = JSON.parse(readFileSync(join(work, 'tts', 'voice.json'), 'utf8'));
  info.audio = { targetLufs: -16, toleranceLu: .2, maxTruePeak: -1.5, music };
  info.main.loudness = verifyAudio(main);
  info.web.loudness = verifyAudio(web);
  info.teaser = { file: join(out, 'beta-teaser-vertical.mp4'), loudness: verifyAudio(join(out, 'beta-teaser-vertical.mp4')) };
  for (const clip of Object.values(info.clips)) clip.loudness = verifyAudio(clip.file);
  writeFileSync(join(out, 'build-info.json'), JSON.stringify(info, null, 2));
  console.log(JSON.stringify({ total: info.total, main: info.main, web: info.web, teaser: info.teaser, clips: info.clips }, null, 2));
}

async function teaser(byId) {
  // Teaser beats: flagged `teaser: true` in scenes.mjs. Cards with a vertical take use it; others are framed.
  const parts = [];
  for (const def of scenes) {
    const s = byId[def.vertical ? `v-${def.id}` : def.id]; if (!s) continue;
    def.beats.forEach((b, i) => {
      if (!b.teaser) return;
      const bt = s.beats[i], next = s.beats[i + 1]?.at ?? s.duration;
      const from = i === 0 ? 0 : bt.at - 0.15, to = Math.min(next, bt.at + bt.dur + GAP + 0.4, s.duration);
      parts.push({ def, s, from, to });
    });
  }
  // Merge adjacent beats of the same scene.
  const merged = [];
  for (const p of parts) { const last = merged.at(-1); if (last && last.s === p.s && Math.abs(last.to - p.from) < 0.6) last.to = p.to; else merged.push({ ...p }); }
  const dir = join(work, 'teaser'); mkdirSync(dir, { recursive: true });
  let t = 0; const seq = [];
  merged.forEach((p, i) => {
    const len = p.to - p.from, f = join(dir, `part-${String(i).padStart(2, '0')}.mp4`);
    const src = join(work, 'scenes', `${p.s.id}.raw.mp4`);
    if (p.s.take === 'vcard') ff(['-ss', p.from.toFixed(3), '-t', len.toFixed(3), '-i', src, '-vf', 'fps=30,format=yuv420p', ...X264_MID, f]);
    else if (p.s.take === 'phone') ff(['-loop', '1', '-framerate', String(FPS), '-i', join(work, 'bg', 'v-phone.png'), '-ss', p.from.toFixed(3), '-t', len.toFixed(3), '-i', src, '-filter_complex', `[1:v]scale=${VPHONE.w}:${VPHONE.h}:flags=lanczos[p];[0:v][p]overlay=${VPHONE.x}:${VPHONE.y}:shortest=1,format=yuv420p`, '-t', len.toFixed(3), ...X264_MID, f]);
    // 16:9 footage: crop to 4:3 (left edge for the basics cards, centre for the app) so text is larger on a phone.
    else ff(['-loop', '1', '-framerate', String(FPS), '-i', join(work, 'bg', `v-${p.def.id}.png`), '-ss', p.from.toFixed(3), '-t', len.toFixed(3), '-i', src, '-filter_complex', `[1:v]${p.def.id === 'basics' ? 'crop=ih*4/3:ih:iw/32:0' : 'crop=ih*4/3:ih:(iw-ih*4/3)/2:0'},scale=${VBOX.w}:${VBOX.h}:flags=lanczos[p];[0:v][p]overlay=${VBOX.x}:${VBOX.y}:shortest=1,format=yuv420p`, '-t', len.toFixed(3), ...X264_MID, f]);
    seq.push({ s: p.s, offset: t, from: p.from, to: p.to, file: f }); t += len;
  });
  if (t > 60.5) console.warn(`Teaser is ${t.toFixed(1)} s; remove a teaser beat in scenes.mjs.`);
  const list = join(work, 'lists', 'teaser.ffconcat');
  writeFileSync(list, ['ffconcat version 1.0', ...seq.map((x) => `file '${fwd(x.file)}'`)].join('\n'));
  const raw = join(work, 'teaser.video.mp4');
  ff(['-f', 'concat', '-safe', '0', '-i', list, '-c', 'copy', raw]);
  const ev = captionEvents(seq.map((x) => ({ ...x, s: { ...x.s, scene: x.s.scene ?? x.s.id } })), 'en');
  const ass = join(work, 'teaser.ass');
  const phoneLike = (sc) => scenes.find((d) => d.id === sc)?.take === 'phone';
  writeFileSync(ass, ASS_HEAD(1080, 1920, [style('Cap', 66, 2, 50, 50, 130), style('CapPhone', 62, 2, 40, 40, 60)]) + ev.map((e) => `Dialogue: 0,${assTime(e.start)},${assTime(e.end)},${phoneLike(e.scene) ? 'CapPhone' : 'Cap'},,0,0,0,,${assText(wrap2(e.text, 26))}`).join('\n') + '\n', 'utf8');
  const audio = narration(ev, t, 'teaser');
  const file = join(out, 'beta-teaser-vertical.mp4');
  ff(['-i', raw, '-i', audio, '-filter_complex', `[0:v]subtitles='${esc(ass)}'[v]`, '-map', '[v]', '-map', '1:a', ...X264_FINAL(21), ...AAC, '-shortest', file]);
  writeSrt(join(out, 'beta-teaser-vertical-en.srt'), ev);
  console.log(`Teaser: ${t.toFixed(1)} s, ${mb(file)} MB`);
}

// ---------------- Poster and explainer images ----------------
async function stills() {
  const main = join(out, 'beta-demo-en.mp4');
  const tl = JSON.parse(readFileSync(join(work, 'timeline.json'), 'utf8'));
  // Poster: a real frame of the checked marker with the title.
  let at = 0; for (const s of scenes) { const x = tl.scenes.find((y) => y.id === s.id); if (!x) continue; if (s.id === 'marker') { at += x.beats[6]?.at - 1.5 || x.duration / 2; break; } at += x.duration; }
  const frame = join(work, 'poster-frame.jpg');
  ff(['-ss', at.toFixed(2), '-i', join(work, 'main.video.mp4'), '-frames:v', '1', '-q:v', '2', frame]);
  const img = readFileSync(frame).toString('base64');
  const posterHtml = page({ css: `
.shot { position: absolute; right: -40px; top: 90px; width: 1240px; border-radius: 24px; overflow: hidden; box-shadow: 0 30px 70px rgba(16,35,28,.3); border: 6px solid #fff; }
.shot img { width: 100%; display: block; }
.copy { position: absolute; left: 90px; top: 120px; width: 600px; }
.logo { width: 150px; height: 150px; margin-bottom: 40px; }
h1 { font-size: 96px; margin-bottom: 28px; }
p { font-size: 40px; font-weight: 600; color: ${C.muted}; }
.play { margin-top: 48px; display: flex; gap: 26px; align-items: center; font: 700 44px 'Segoe UI'; color: ${C.green2}; }
.play i { width: 124px; height: 124px; border-radius: 50%; background: ${C.green}; display: grid; place-items: center; box-shadow: 0 14px 30px rgba(23,107,82,.35); }
.play i::after { content: ''; border-left: 42px solid #fff; border-top: 26px solid transparent; border-bottom: 26px solid transparent; margin-left: 10px; }`,
  body: `<div class="shot"><img src="data:image/jpeg;base64,${img}" alt=""></div><div class="copy"><div class="logo">${LOGO}</div><h1>Save fabric on every marker</h1><p>Free, open-source cutting-room software. See how it works.</p><div class="play"><i></i>Watch the demo</div></div>` });
  const posterPng = join(work, 'poster.png');
  await renderHtml([
    { html: posterHtml, file: posterPng, w: 1920, h: 1080 },
    { html: explainerImage('en'), file: join(out, 'beta-explainer-en.png'), w: 1080, h: 1350 },
    { html: explainerImage('hi'), file: join(out, 'beta-explainer-hi.draft-needs-native-review.png'), w: 1080, h: 1350 },
  ]);
  ff(['-i', posterPng, '-q:v', '3', join(out, 'beta-demo-poster.jpg')]);
  ff(['-i', posterPng, '-vf', 'scale=1280:-2', '-q:v', '4', join(out, 'beta-demo-poster-1280.jpg')]);
  for (const f of ['beta-explainer-en.png', 'beta-explainer-hi.draft-needs-native-review.png']) ff(['-i', join(out, f), '-q:v', '3', join(out, f.replace('.png', '.jpg'))]);
  if (!existsSync(main)) console.warn('Build the video before stills.');
}

// ---------------- Self-check: contact sheet ----------------
function check() {
  const font = 'C\\:/Windows/Fonts/segoeuib.ttf';
  const sheets = [['beta-demo-en.mp4', 'contact-sheet-main.png', 5], ['beta-teaser-vertical.mp4', 'contact-sheet-teaser.png', 6], ['beta-demo-hi.mp4', 'contact-sheet-hindi.png', 5]];
  for (const [src, dst, cols] of sheets) {
    const f = join(out, src); if (!existsSync(f)) continue;
    const d = probe(f), n = Math.floor(d / 10) + 1, rows = Math.ceil(n / cols);
    const w = src.includes('vertical') ? 270 : 480;
    ff(['-i', f, '-vf', `fps=1/10,scale=${w}:-2,drawtext=fontfile='${font}':text='%{pts\\:hms}':x=8:y=8:fontsize=22:fontcolor=white:box=1:boxcolor=0x10231cCC:boxborderw=6,tile=${cols}x${rows}:padding=6:margin=6:color=0xfbfaf6`, '-frames:v', '1', join(out, dst)]);
    console.log(`${dst}: ${n} frames from ${d.toFixed(1)} s`);
  }
  const files = readdirSync(out).filter((f) => /\.(mp4|srt|vtt|png|jpg)$/.test(f)).map((f) => `${f}  ${mb(join(out, f))} MB`);
  console.log(files.join('\n'));
}

/** Writes SCRIPT.md (storyboard and narration) from scenes.mjs, so the document never drifts from the video. */
function script() {
  const t = existsSync(join(work, 'timeline.json')) ? JSON.parse(readFileSync(join(work, 'timeline.json'), 'utf8')) : null;
  const what = {
    hook: 'Title card: logo, "Save fabric on every marker", pills Free / Open source / Runs in your factory. A strip of pattern pieces slides in.',
    basics: 'Animated explainer (original SVG). A shirt becomes flat pieces. The pieces are laid on a fabric strip with a width arrow and a hatched waste gap. The pieces close up and the marker gets shorter. A worked example labelled EXAMPLE: 10 cm x 50 layers = 5 m; x ₹150 = ₹750. A laptop screen fills with pieces and shows a Checked badge.',
    problem: 'Two cards: a chalk marker drawn by hand with red gaps; commercial CAD with rupee coins.',
    marker: 'Real app, 1440 x 810 CSS px rendered at 1920 x 1080. Load the demo shirt; type width 1500; ratio L 2, M 2; time 6 s; Nest. Zoom on "Best so far", then on length and efficiency, then on "Checks passed". Open the Export menu and ring PDF and PLT.',
    cutplan: 'Real app. Open Cut plan; type 40 L, 40 M and max plies 20; Make cut plan; zoom on the result.',
    floor: 'Real app with synthetic rolls DEMO-001 and DEMO-002. Save a roll and inspect; prepare spreading sheet DEMO-LAY-01; make bundles with QR tickets.',
    quick: 'Real app on a 390 x 844 phone screen in a phone frame. Practice photos, labelled SAMPLE on screen. Start screen; sample photo; outlines over the photo; accept pieces; width preset and garments; nest on the local server and stop; print tiles, chalk guide, projector.',
    pattern: 'Real app. Teaching bodice; grading for all sizes; Seam checks; Output (DXF, full-size PDF).',
    proof: 'Card: public Lectra benchmark, 300 tasks at 60 s: mean -0.73 and median -1.22 points, a 84 / 12 / 204 win-tie-loss bar, "Close, but not better. No general win." Then: every marker is checked by an independent validator.',
    feedback: 'Real app: ring and open the Feedback button.',
    close: 'Card: wordmark; Free / Works offline / Your data stays in your factory; a young project built for fun with modern AI tools; "Try it now in your browser" with the website address; code link; Feedback.',
  };
  const lines = ['# BETA Cutting Room demo video: script and storyboard', '', 'Generated from `scenes.mjs` by `make-video.mjs script`. Edit `scenes.mjs`, not this file.', '',
    'Audience: cutting masters, CAD operators and owners of small and mid garment units in India. Many watch on a phone with the sound off, so every word is also on screen.', '',
    'Flow: hook, the basics (pattern piece, marker, why a shorter marker saves money, what CAD does), the problem, the real app (marker, cut plan, floor, phone, pattern), honest proof, how to start.', ''];
  if (t) { const total = t.scenes.filter((s) => !s.id.startsWith('v-')).reduce((a, s) => a + s.duration, 0); lines.push(`Last recorded length: ${Math.floor(total / 60)} min ${Math.round(total % 60)} s.`, ''); }
  for (const s of scenes) {
    const rec = t?.scenes.find((x) => x.id === s.id);
    lines.push(`## ${s.id}${s.section ? ` - ${s.section}` : ''}${rec ? ` (${rec.duration.toFixed(1)} s)` : ''}`, '', `**Picture:** ${what[s.id] ?? ''}`, '', '| # | Voice (English) | Caption | Hindi draft | Tamil draft |', '|---|---|---|---|---|');
    s.beats.forEach((b, i) => lines.push(`| ${i + 1}${b.teaser ? ' (teaser)' : ''} | ${b.say} | ${b.en ?? 'same'} | ${b.hi} | ${b.ta} |`));
    lines.push('');
  }
  lines.push('Hindi and Tamil lines are drafts. They need review by a native speaker before anyone publishes them.', '');
  writeFileSync(join(here, 'SCRIPT.md'), lines.join('\n'));
  console.log('Wrote SCRIPT.md');
}

const steps = { tts, record, retime, build, stills, check, script };
for (const s of step === 'all' ? ['tts', 'record', 'build', 'stills', 'check'] : step.split(',')) {
  if (!steps[s]) throw new Error(`Unknown step ${s}`);
  console.log(`== ${s}`); await steps[s]();
}
void SITE_URL;
