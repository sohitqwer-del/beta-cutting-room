# Launch 0.8 video checks

Base app source: `d4932e6a9658390a29f67f35b15a9301721e3e25`.
The revised app uses the same four Rust WASM modules, with a new built-in sample,
locale plural handling and accurate review status. TypeScript and Vite static
builds passed. Current scene revisions are built from this checkout; capture
reports retain the static HTML hash. Unchanged tour/marker clips may be reused.

- Headless real-app capture: home, real photo reader, outline review, tape check,
  paper marker, T-shirt block, grade, lay, live marker, cut, exports, order cut plan,
  limits and feedback. No practice button or mock engine. Only synthetic data.
- Zero recorded page errors and zero external requests. The recorder blocks
  non-loopback requests and closes its browser and local server.
- Synthetic T-shirt teaching set: five parts generated from Rust block geometry
  at 30% scale. All five are found, all five drawn grain arrows are detected,
  the estimated error is 1.867 mm, and no image-quality warnings occur. The real
  check passes the independent 143.8546 mm side-edge dimension. A4 confidence
  remains medium; the good UI status means the tape check passed. Routine review
  stays visible as guidance; real quality warnings remain warnings. This is not
  a factory-photo accuracy test, and one distance does not validate fit or curves.
- The sample marker uses four teaching garments at 450 mm width. The recorded
  result was 77.7% fabric use, versus 24.9% for the previous one-panel fixture.
  The recorder rejects results below 65% for this sample.
- All 59 Quick Marker web tests pass, including EN/HI/TA plural variants and the
  real WASM sample-scan regression. Tests verify that missing grain, blur and
  low-confidence warnings cannot be hidden by the guidance classification.
- Three actual browser downloads passed format checks: PLT begins `IN;`, DXF has
  a SECTION header, and PDF begins `%PDF`. Byte counts are in the capture report.
  No physical plotter or cutter was tested.
- All four social formats are 45 seconds. The walkthrough is 188.53 seconds.
  MP4s use H.264, with AAC narration or silence. The 7-second loop has no audio.
- Every final MP4 is fully decoded and scanned for black frames. Social files
  must stay below 30 MB; the GIF must stay below 8 MB. Faststart and codec checks
  are automatic. SHA-256 and stream metadata are recorded in `manifest.json`.
- Caption text is compared against the exact Kokoro input for all five edits.
  Beat timing uses measured clean speech lengths. Long lines split into phrases;
  phrase timing within a beat follows word count rather than forced alignment.
- Final AAC loudness is measured after encoding. The delivery gate is -16 LUFS
  within 1 LU. First-pass readings were -16.17 (social) and -16.12 (walkthrough);
  the final manifest carries the measurements for the actual delivered files.
- Five-second contact sheets were inspected across the social cuts and walkthrough.
  Phone samples at 375 px were checked for captions and feature headings. Square
  and 4:5 edits use enlarged real-app crops for the photo, pattern, grade and marker.
  The photo view combines the actual outline image with the actual checked status.
  No blank feature frames, browser chrome, personal paths or private data were seen.
- All 14 pinned local audio model/data/licence source files passed verification.
  No music, remote TTS, runtime downloads or redistributed font files are used.

## Review limits

A person should listen before public posting for synthetic voice naturalness and
pronunciation. Text equality and loudness checks do not establish that. Photo
accuracy, garment fit and physical machine compatibility remain product limits.
The social edit cuts between real actions; explanations may hold the last frame.
The marker search itself is played from its recorded timestamps without speed-up.

Use the external kit's `VIDEO-0.8.md`, `manifest.json`, `work8/record-report.json`
and contact sheets for the actual file inventory and evidence. Nothing was pushed
or published, and no video, model or audio file is committed.
