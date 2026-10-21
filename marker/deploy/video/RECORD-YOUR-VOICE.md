# Record the BETA demo on your phone

Your own voice can make this demo feel closer to your factory. Read the English lines in
`SCRIPT.md`, or ask a native Hindi speaker to check the Hindi draft first. Use only the public
demo script. Do not read customer names, orders or other factory data.

1. Use a small, quiet room with curtains or soft furniture. Turn off the fan if safe. Shut
   the door. Put the phone in airplane mode. Record a short test and listen with earphones.
2. Use the phone's normal voice recorder. WAV or M4A is fine. Keep the microphone about
   15–20 cm from your mouth, slightly to one side. Keep the phone still. Do not touch its
   microphone. Speak calmly, as if explaining the app to one cutting master.
3. Aim for 140–150 words per minute. Pause normally between sentences. **Leave two seconds
   of silence between numbered lines in the script table.** Leave two seconds at the start
   and end of each scene. Inside one numbered line, keep pauses shorter than one second.
4. Make **one file per scene**, named exactly as below. Use one language for the whole set.
   If you make a mistake, record that scene again. Do not add music or recorder effects.

| File name (WAV or M4A) | Number of numbered lines |
|---|---:|
| `hook.wav` | 2 |
| `basics.wav` | 6 |
| `problem.wav` | 1 |
| `marker.wav` | 8 |
| `cutplan.wav` | 3 |
| `floor.wav` | 3 |
| `quick.wav` | 6 |
| `pattern.wav` | 3 |
| `proof.wav` | 4 |
| `feedback.wav` | 1 |
| `close.wav` | 3 |

Say BETA as “bay-tuh” in English. Say the money amounts in full: “one hundred and fifty
rupees” and “seven hundred and fifty rupees.” Check Lectra and Tukatech with the owner.
Keep the honest benchmark wording. Listen once before handing over the files.

Put all 11 files in one folder outside git. From the repository root:

```sh
node marker/deploy/video/make-video.mjs tts,retime,build,check --voice-dir <recording-folder> --source <previous-video-folder> --out <new-video-folder> --playwright <playwright-core-folder>
```

Use a new output folder. Add `--lang hi` for Hindi narration and Hindi captions. Hindi needs
native review. `tts` here imports your recordings; it never replaces them with synthetic
speech. The pipeline cleans the audio, splits the long pauses, measures each line, moves
the picture and captions with it, and checks the final videos at -16 LUFS and ≤ -1.5 dBTP.
Speech timing stays as recorded; it is not forced to a computer voice's speed.

If automatic splitting finds the wrong number of lines, keep the recording and add a cue
file such as `hook.json`. Times are seconds from the start of `hook.wav`:

```json
{"beats":[{"id":"hook-1","start":2.0,"end":4.5},{"id":"hook-2","start":6.5,"end":12.6}]}
```

Cues must cover every numbered line, in order, without overlaps. Make all edits locally.
Nothing is uploaded or published by this pipeline.
