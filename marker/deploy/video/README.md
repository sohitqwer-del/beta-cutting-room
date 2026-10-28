# Demo video pipeline

## Launch 0.8 (current)

Use `launch8.mjs` for the 45-second social cuts and the three-path walkthrough.
`launch-scenes.mjs` is the single source for English narration and captions;
`SCRIPT.md` is generated from it. The older commands below retain the previous release edit.
No mock engine or server API is used for 0.8. The sample button uses the real scanner.
The recorder serves this checkout's static build on loopback and closes the server and
headless Chrome when it finishes. It blocks requests outside loopback. All feature
footage is the actual app; only the closing card is editorial graphics.

First install the workspace dependencies and build all four Rust WASM modules:

```sh
npm --prefix marker run build:static --workspace @beta/marker-web
```

Set `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS=3` and `WASM_BINDGEN` for that build.
Use an external output folder. Supply the absolute paths below through arguments
or set `PLAYWRIGHT_CORE`, `NARRATION_PYTHON`, `NARRATION_MODELS` and `CHROME_PATH`.
The recorder presses the built-in sample button. Its original PNG is generated
from the real Rust T-shirt block: front, back, two sleeves and neck rib at 30%
teaching scale, beside a complete A4 sheet. The image has drawn grain arrows,
even light, slight perspective and no factory data. The rib's grain arrow runs
along its long edge so the short teaching strip can show a detectable arrow.
This is a demonstration fixture, not a fit or camera-accuracy claim.

Regenerate it with one command, then copy the PNG into `src/quick/sample-tshirt.png`:

```sh
node marker/deploy/video/make-paper-sample.mjs <absolute-wasm-pkg> <external-fixture-dir> <playwright-core-dir>
```

Rust WASM supplies block geometry. The fixture manifest records original cut
outlines and the known 143.8546 mm straight side edge. The tape check uses
that independent fixture dimension (14.38546 cm), never a length inferred from
the scan. The app explains the rounded 14.39 cm check to sample visitors.
The A4 scanner intentionally reports medium confidence; only a passed tape
check makes the UI's size-check chip good. Routine review remains guidance;
actual quality warnings are not suppressed. `sample-scan.mjs` tests the real
WASM reader: five pieces, five grain arrows, error estimate below 2.2 mm and
no quality warnings. Single-sheet accuracy still needs physical validation.

Social square/4:5 edits crop complete important regions. The photo scene
combines enlarged crops of the actual outline view and passed-check status.
The generated fixture is labelled as synthetic and at 30% scale.

One command for the complete kit (build first):

```sh
node marker/deploy/video/launch8.mjs all --out <external-video-dir> --build <absolute-marker-dir>/out/static-app --playwright <playwright-core-dir> --python <venv>/Scripts/python.exe --models <audio-models-dir>
```

Steps can also run separately: `tts`, `record`, `build`, `stills`, `check`, `script`.
After TTS and recording, each output has a single command:

```sh
node marker/deploy/video/launch8.mjs build --target square --out <external-video-dir> --playwright <playwright-core-dir>
node marker/deploy/video/launch8.mjs build --target portrait --out <external-video-dir> --playwright <playwright-core-dir>
node marker/deploy/video/launch8.mjs build --target wide --out <external-video-dir> --playwright <playwright-core-dir>
node marker/deploy/video/launch8.mjs build --target silent --out <external-video-dir> --playwright <playwright-core-dir>
node marker/deploy/video/launch8.mjs build --target walk --out <external-video-dir> --playwright <playwright-core-dir>
node marker/deploy/video/launch8.mjs stills --out <external-video-dir>
node marker/deploy/video/launch8.mjs check --out <external-video-dir>
```

Square and 4:5 are silent, with an AAC silence track and burned captions. Wide
has voiced and silent choices. Walkthrough has burned captions plus SRT/VTT.
No music is used. Voiced AAC must measure -16 LUFS within 1 LU. Check writes
SHA-256, sizes, durations, stream metadata, loudness, and `VIDEO-0.8.md`.
It decodes every MP4 and detects black frames. Review the five-second contact sheets
and phone-size samples for cropping and privacy. Captions are split at phrase boundaries;
the exact same words are passed to Kokoro. A human should listen before release.

`work8/` retains source JPEG frames with real timestamps, clips, clean WAV lines,
edit timings, exports and capture report. Actions are never sped up; short social
clips select intervals, and longer explanations hold the last frame. Rebuilds can
differ in nesting layout. Keep the source frames for an exact footage rebuild.

## Earlier release pipeline

This folder makes the BETA Cutting Room demo video again for each release. It records the real app in a
headless browser, adds offline narration and burned-in captions, and writes all deliverables.
Nothing opens on screen, and nothing is downloaded at run time.

| File | What it does |
|---|---|
| `scenes.mjs` | The script as data: scenes, narration lines, English captions, Hindi and Tamil drafts, recorder actions, teaser and clip choices. **Edit this file to change the video.** |
| `SCRIPT.md` | Storyboard and narration, written from `scenes.mjs` (`make-video.mjs script`). Do not edit by hand. |
| `cards.mjs` | Title cards, the animated "basics" explainer, phone and vertical backgrounds, the poster and the one-page explainer image. Original SVG and HTML; the logo and font come from `apps/site/public/assets`. |
| `record.mjs` | Drives the real app with Playwright and captures frames with the DevTools screencast (full-resolution JPEG with timestamps). Adds a cursor, tap marks, highlight rings and zooms to the recording page only. |
| `neural-tts.py` | Local Kokoro/Piper ONNX inference with permissive dictionary data and explicit pronunciations. |
| `audio.mjs` | Shared cleaning, loudness checks, phone-recording import and timing. |
| `tts.ps1` | Existing Windows OneCore baseline, available with `--engine onecore`. |
| `AUDIO-SOURCES.md`, `audio-models.json`, `setup-audio.py` | Official download URLs, licences, SHA-256 hashes and explicit setup. |
| `AUDIO-COMPARISON.md` | Sample results and the voice choice. |
| `RECORD-YOUR-VOICE.md` | One-page phone recording guide. |
| `make-video.mjs` | Runs the steps and does the editing with ffmpeg. |

## What you need

- Python 3.11 in an external venv and local model files prepared with `setup-audio.py`. See [AUDIO-SOURCES.md](AUDIO-SOURCES.md). The default voice is **Kokoro Heart (`af_heart`)**, Apache-2.0 weights, CPU-only ONNX Runtime. No runtime downloads.
- Windows 10 or 11 for the existing recording tools. OneCore is optional and used only with `--engine onecore`.
- `ffmpeg` and `ffprobe` on PATH, built with libx264, libass and libfreetype.
- Chrome or Edge (`--chrome <exe>` or `CHROME_PATH`).
- `playwright-core` installed **outside the repository** (`--playwright <dir>` or `PLAYWRIGHT_CORE`). It is not a dependency of BETA.
- A release CLI and the web app built from this checkout, served on loopback:
  `beta-marker serve --port 7891 --web-dir apps/web/dist` (run inside `marker/`).

## Make the video

From the repository root:

```sh
node marker/deploy/video/make-video.mjs all --out <new output folder> --app http://127.0.0.1:7891/ --playwright <dir with playwright-core> --python <audio-data>/venv/Scripts/python.exe --models <audio-data>/models
```

Steps can run one at a time: `tts`, `record`, `retime`, `build`, `stills`, `check`, `script`.
To reuse the original synthetic demo capture, run `tts,retime,build,check --source <previous output folder> --out <new output folder>` with the same Python, models and Playwright arguments. `retime` reads the original frames and grows each beat interval when narration needs more time. It never speeds up the recorded actions or overwrites the source frames. Scene ends snap to 30 fps boundaries to prevent accumulated drift.
`record --only marker,quick` re-records some scenes and merges them into the existing timeline.
Music is off by default. `--music 1` adds the quiet self-generated pad. `--wpm 145` sets neural speech pace. `--neural-voice bf_emma` selects the British review alternative; `--engine piper` selects LJ Speech. English is the default. `--lang hi` makes `beta-demo-hi.mp4` with Hindi captions in `work-hi/`; it leaves the English outputs intact. **Hindi script needs native review.** The hand-written Hindi phonemes cover only this script and fail if a caption line changes.

`--voice-dir <folder>` imports phone recordings instead of TTS. See [RECORD-YOUR-VOICE.md](RECORD-YOUR-VOICE.md). It uses the same cleaning, measured timing and final loudness checks. The original OneCore comparison remains available with `--engine onecore --voice-name "Microsoft Heera" --rate 1.12`. `--rate` affects OneCore only. `--preset veryfast` trades file size for a faster local review encode; the default is `slow`.

How timing works: `tts` measures each narration line. `record` starts each line's action when the line
starts and holds the scene until the line ends (or longer when the app is busy), and it writes the real
start time of every line to `work/timeline.json`. `build` puts each voice line and caption at that real
time, so sound, captions and picture stay together even when the app is slower on another computer.
Steps that only prepare the app (for example adding a first roll) run in "hidden" intervals that the
editor cuts out.

## What it writes

| File | Notes |
|---|---|
| `beta-demo-en.mp4` | 1920 x 1080, 30 fps, H.264 + AAC, English captions burned in, audio at -16 LUFS |
| `beta-demo-en.web.mp4` | Copy for the website (re-encoded smaller only if the main file is over 24 MB) |
| `beta-teaser-vertical.mp4` | 1080 x 1920, about 60 s, for WhatsApp status and Shorts |
| `clips/beta-<feature>.mp4` | Basics, marker, cut plan and floor, Quick Marker, pattern, for the Help page |
| `beta-demo-en.srt`, `.vtt` | English captions |
| `beta-demo-hi.mp4`, `.srt`, `.vtt`, `build-info-hi.json` | Optional Hindi review video, Hindi burned-in captions, with native-review watermark |
| `beta-demo-hi.draft-needs-native-review.srt/.vtt`, `beta-demo-ta...` | Hindi and Tamil drafts. **A native speaker must review them before publishing.** |
| `beta-demo-poster.jpg` | Poster: a real frame of the checked marker with the title |
| `beta-explainer-en.png/.jpg`, `beta-explainer-hi.draft-needs-native-review.png/.jpg` | 1080 x 1350 one-page "What is BETA?" image for WhatsApp |
| `contact-sheet-main.png`, `contact-sheet-teaser.png` | One frame every 10 s, for a quick visual check |
| `build-info.json` | Lengths, sizes, loudness, scene start times |

Never commit the video files. The `work/` and `work-hi/` folders hold intermediate files. Retimed builds still reference their source frame folder; retain that source when rebuilding. No model, recording or video belongs in git.

## Data and honesty rules

- Only the built-in synthetic demo shirt, the teaching bodice, synthetic rolls (DEMO-001/002) and the Quick
  Marker practice photos. Never factory data. The phone scene shows the app's own "Practice · sample pieces"
  label, and the frame adds a SAMPLE badge.
- The rupee example in the basics is an illustration and is labelled EXAMPLE on screen.
- Benchmark numbers are the public Lectra results from `docs/benchmarks.md`: always mean, median and
  win/tie/loss together, and no claim that BETA beats Lectra. Update `scenes.mjs` and `cards.mjs` together
  when the numbers change.
- The background pad is a chord of sine tones made by ffmpeg in `make-video.mjs`. No downloaded music.

## Website

Copy `beta-demo-en.web.mp4` to `marker/apps/site/public/assets/video/beta-demo-en.mp4` (it is git-ignored),
and the poster and English VTT to the same folder, then rebuild the site. If the file is too big for the
site, leave it out and set `demoVideoUrl` in `marker/apps/site/src/config.ts`; the home page then shows the
poster as a link. Update `demoVideo.length` and `size` in the same file.

## Known limits

- Heart is a US-English voice, not an Indian-accent model. Brand pronunciations are explicit editorial choices. A person should listen before release; automated measurements cannot establish naturalness or pronunciation quality.
- The Hindi review version uses Kokoro Omega and original, script-specific phonemes. Its script, pronunciation and number reading need native review. Tamil remains a caption draft only.
- This dictionary frontend fails on unknown English words. Add and review a pronunciation instead of silently guessing. It has no statistical homograph disambiguation.
- The nest in the video runs 6 seconds on the local server, so its numbers can differ slightly between runs.
- The DevTools screencast delivers frames at CSS-pixel size, whatever the device scale. The app is laid out at
  1440 x 810 (so its text is large on a phone) and scaled 1.33x to 1080p; the phone screen is captured at
  390 x 844 and scaled 1.2x (main video) or 1.5x (teaser). Sharp device-pixel screenshots
  (`Page.captureScreenshot` with a scaled clip) reach only about 11 fps on the desktop page, which makes the
  cursor jerky. Cards are rendered natively at 1920 x 1080 and 1080 x 1920.

## Audio checks

Each line gets an 80 Hz high-pass, a small low-mid cut and presence lift, gentle gate,
de-essing and 2:1 compression. Internal sentence pauses stay intact. Masters use measured
two-pass loudness normalization. Every final AAC file (main, web, teaser and each clip)
is measured again and must be within 0.2 LU of -16 LUFS and at or below -1.5 dBTP.
The normalizer targets -1.8 dBTP to leave AAC headroom. Results and the exact voice metadata
are written to the build-info files. Clips are normalized independently.

```sh
node marker/deploy/video/check-audio.mjs <external audio-test folder>
node marker/deploy/video/compare-audio.mjs <external samples folder>
```

The first command checks import and timing with synthetic local audio. The second expects
the same three test WAVs per candidate and makes level-matched `*.review.wav` copies,
`listen-first.wav` playlists and `comparison.json`. No audio is played automatically.
