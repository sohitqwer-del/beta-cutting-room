# Demo video pipeline

This folder makes the BETA Cutting Room demo video again for each release. It records the real app in a
headless browser, adds offline narration and burned-in captions, and writes all deliverables.
Nothing opens on screen, and nothing is downloaded at run time.

| File | What it does |
|---|---|
| `scenes.mjs` | The script as data: scenes, narration lines, English captions, Hindi and Tamil drafts, recorder actions, teaser and clip choices. **Edit this file to change the video.** |
| `SCRIPT.md` | Storyboard and narration, written from `scenes.mjs` (`make-video.mjs script`). Do not edit by hand. |
| `cards.mjs` | Title cards, the animated "basics" explainer, phone and vertical backgrounds, the poster and the one-page explainer image. Original SVG and HTML; the logo and font come from `apps/site/public/assets`. |
| `record.mjs` | Drives the real app with Playwright and captures frames with the DevTools screencast (full-resolution JPEG with timestamps). Adds a cursor, tap marks, highlight rings and zooms to the recording page only. |
| `tts.ps1` | Offline narration with a Windows OneCore voice (WinRT `Windows.Media.SpeechSynthesis`). Prefers an en-IN voice. |
| `make-video.mjs` | Runs the steps and does the editing with ffmpeg. |

## What you need

- Windows 10 or 11 with an English OneCore voice. The default is **Microsoft Heera (en-IN)**; `--voice-name "Microsoft Ravi"` picks the male en-IN voice.
- `ffmpeg` and `ffprobe` on PATH, built with libx264, libass and libfreetype.
- Chrome or Edge (`--chrome <exe>` or `CHROME_PATH`).
- `playwright-core` installed **outside the repository** (`--playwright <dir>` or `PLAYWRIGHT_CORE`). It is not a dependency of BETA.
- A release CLI and the web app built from this checkout, served on loopback:
  `beta-marker serve --port 7891 --web-dir apps/web/dist` (run inside `marker/`).

## Make the video

From the repository root:

```sh
node marker/deploy/video/make-video.mjs all --out <output folder> --app http://127.0.0.1:7891/ --playwright <dir with playwright-core>
```

Steps can run one at a time: `tts`, `record`, `build`, `stills`, `check`, `script`.
`record --only marker,quick` re-records some scenes and merges them into the existing timeline.
`--music 0` leaves out the soft background pad. `--rate 1.12` is the speaking rate.

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
| `beta-demo-hi.draft-needs-native-review.srt/.vtt`, `beta-demo-ta...` | Hindi and Tamil drafts. **A native speaker must review them before publishing.** |
| `beta-demo-poster.jpg` | Poster: a real frame of the checked marker with the title |
| `beta-explainer-en.png/.jpg`, `beta-explainer-hi.draft-needs-native-review.png/.jpg` | 1080 x 1350 one-page "What is BETA?" image for WhatsApp |
| `contact-sheet-main.png`, `contact-sheet-teaser.png` | One frame every 10 s, for a quick visual check |
| `build-info.json` | Lengths, sizes, loudness, scene start times |

Never commit the video files. The `work/` folder holds frames and intermediate files and can be deleted.

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

- The narration is a computer voice. A human voice-over in Hindi or Tamil would reach more people.
- Burned-in captions are English only. Hindi and Tamil are separate subtitle files (drafts).
- The nest in the video runs 6 seconds on the local server, so its numbers can differ slightly between runs.
- The DevTools screencast delivers frames at CSS-pixel size, whatever the device scale. The app is laid out at
  1440 x 810 (so its text is large on a phone) and scaled 1.33x to 1080p; the phone screen is captured at
  390 x 844 and scaled 1.2x (main video) or 1.5x (teaser). Sharp device-pixel screenshots
  (`Page.captureScreenshot` with a scaled clip) reach only about 11 fps on the desktop page, which makes the
  cursor jerky. Cards are rendered natively at 1920 x 1080 and 1080 x 1920.
