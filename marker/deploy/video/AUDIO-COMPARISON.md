# Narration comparison and v2 results

## Choice

Use **Kokoro Heart (`af_heart`)** for English. Its weights and voice style are Apache-2.0. The official voice list rates Heart A and Emma B-. On this small, unprompted offline recognition test, Heart and Emma each had 2 word errors in 51 words; Heera had 4 and Piper LJ Speech had 9. Heart also preserved "one lay," where the recognizer heard Heera as "one leg." This supports Heart as a candidate; it does not prove human preference or general superiority.

Heart is US English, **not an Indian-accent voice**. Clear script wording, a deliberate 145 wpm pace, spelled-out rupee amounts and explicit brand phonemes make it practical for an Indian audience to review. A local owner recording is the strongest accent/authenticity option.

## Same three lines

1. "BETA Marker uses nesting to place every pattern piece. Check the marker length and efficiency."
2. "Compare Lectra and Tukatech with care. BETA is close, but not better yet."
3. "At 150 rupees a metre, five metres saves 750 rupees. That is fabric saved on one lay."

Hindi samples are direct translations of these lines. Each candidate has all three raw WAVs, cleaned WAVs and level-matched review WAVs. No sound was played on the shared laptop. Raw model output and final listening copies remain separate.

| Candidate | Measured speech wpm after edge trim | Raw LUFS range | PCM clipped samples | English ASR errors / words |
|---|---:|---:|---:|---:|
| Microsoft Heera, en-IN | 148.5 | -19.99 to -19.15 | 0 | 4 / 51 |
| Kokoro Heart, US English | 146.5 | -22.38 to -22.26 | 0 | 2 / 51 |
| Kokoro Emma, British English | 146.7 | -19.70 to -19.33 | 0 | 2 / 51 |
| Piper LJ Speech, US English | 146.3 | -24.38 to -22.15 | 0 | 9 / 51 |
| Kokoro Omega, Hindi draft | 146.0 | -17.40 to -17.11 | 0 | Not scored |
| Kokoro Alpha, Hindi draft | 146.3 | -17.13 to -16.34 | 0 | Not scored |

All final review WAVs target -16 LUFS. Individual short-file results range from -16.40 to -15.72 LUFS; the exact values are in `samples/comparison.json`. Do not mistake the louder raw Hindi samples for better sound. Final **video** files use a separate measured delivery check within 0.2 LU of the target.

## Pronunciation and naturalness

- The recognizer preserved BETA, marker, nesting and efficiency for Heart, Emma and Heera. For Piper it heard "Vator" for BETA and "deficiency" for efficiency. This is specific to LJ Speech with this dictionary frontend; it is not a judgment on every Piper model or its standard frontend.
- No candidate got exact Lectra and Tukatech spellings from the small recognizer. Heart produced "Electra" and "Tuketech"; Emma "lecture" and "tukitech"; Heera "lecture" and "took edge." Explicit source phonemes are `lˈɛktɹə` and `tˈuːkətɛk`. **Have the owner confirm both names by listening.**
- The recognizer recovered 150 and 750 rupees for all English candidates; Piper misrecognized "metre" as "mirror." Numbers are expanded before synthesis, so the narration does not read currency glyphs or digit strings.
- Naturalness is not established by loudness, clipping, ASR or the model author's grade. No blind human score or fabricated listening score is reported. Listen for word stress, metallic vowels, sibilance, breath, and the pause after each sentence.
- English uses an original, limited dictionary frontend with no statistical part-of-speech model. Unknown words fail. Homographs and edited names need review.
- Hindi Omega and Alpha both rendered all three lines without PCM clipping. Alpha required pre-PCM attenuation on one floating-point output. Omega was selected provisionally for the draft; it did not need that attenuation on these samples. Neither has a validated native naturalness rating.
- Whisper base produced Urdu-script text and substitutions for the Hindi samples despite a Hindi language hint. It is unsuitable as a Hindi approval gate here. **Hindi script needs native review**, including our hand-written demo-specific phonemes. The Hindi video is a review artifact, not an approved translation.

## Other options and licence boundaries

Kokoro ONNX, the original MIT Piper engine, per-voice Piper metadata and the official Hindi example were checked. The standard Hindi Kokoro frontend uses eSpeak NG; a current Piper engine is GPL. Neither is installed. Direct ONNX inference plus permissive dictionary data avoids those components. The LJ Speech model is covered by the official repository's MIT declaration and its model card names public-domain source data. Other Piper voices may have different terms and were not cleared. No commercial voice, cloud service, voice clone or downloaded music was used.

## What changed in the sound chain

Each line gets an 80 Hz high-pass; -1 dB around 300 Hz; +1.2 dB around 3 kHz; gentle gate and de-essing; and 2:1 compression. Internal pauses remain. Neural speech is rendered in sentences with a 280 ms pause between English sentences, then pitch-preserving tempo adjustment targets 145 wpm. Two-pass normalization targets -16 LUFS and -1.8 dBTP. Final AAC is checked again within 0.2 LU of -16 LUFS and at or below -1.5 dBTP. Music is off.

Original beat intervals are held or lengthened. Frame times, action boundaries, captions and narration move together. Scene ends snap to 30 fps. The original capture frames are read only. The English demo grows from 224.8 s to 236.1 s; the Hindi draft is 247.83 s. The vertical teaser is 58.8 s.

## Listen first and inspect

Paths below are relative to the external audio data folder, never the repository.
- `samples/kokoro-heart/listen-first.wav`: 21-second English comparison playlist; start here.
- `samples/heera-baseline/listen-first.wav` and `samples/kokoro-emma/listen-first.wav`: level-matched comparisons.
- `samples/piper-ljspeech/listen-first.wav`: the second neural engine.
- `samples/kokoro-hindi-omega/listen-first.wav` and `samples/kokoro-hindi-alpha/listen-first.wav`: Hindi review samples.
- `samples/comparison.json`: duration, wpm, raw/review loudness, true peak and clipping for all 18 test lines.
- `samples/pronunciation-asr.json` and each candidate's `asr.txt`: unprompted local recognition, including mistakes.

The external demo-video `v2/` folder contains all deliverables. Do not overwrite the original output folder. The build-info files contain full measured levels and exact voice metadata.

| v2 file | Seconds | LUFS | True peak (dBTP) |
|---|---:|---:|---:|
| `beta-demo-en.mp4` | 236.10 | -16.12 | -1.52 |
| `beta-demo-en.web.mp4` | 236.10 | -16.12 | -1.52 |
| `beta-teaser-vertical.mp4` | 58.80 | -16.12 | -4.51 |
| `clips/beta-basics.mp4` | 34.90 | -16.12 | -4.68 |
| `clips/beta-marker.mp4` | 39.60 | -16.04 | -4.09 |
| `clips/beta-cutplan.mp4` | 34.50 | -16.20 | -4.50 |
| `clips/beta-quick.mp4` | 39.80 | -16.06 | -4.31 |
| `clips/beta-pattern.mp4` | 19.80 | -16.03 | -3.19 |
| `beta-demo-hi.mp4` | 247.83 | -16.19 | -4.22 |

Also rebuilt: English/Hindi SRT and VTT, Hindi/Tamil caption drafts aligned to the English timeline, poster, explainer images and all three contact sheets. The Hindi-narrated file uses `beta-demo-hi.srt/.vtt`, with its own longer timeline and only Hindi narration captions. English UI/cards are still the original demo.

## Validation

- 12 focused checks: retiming, no source mutation, frame alignment, missing duration failure, real WAV pause splitting, cue-file import including BOM, invalid-overlap rejection, cleaning and normalization.
- 18 setup source-file SHA-256 checks.
- English and Hindi single-line inference passed with Python socket creation blocked.
- All 18 comparison lines measured; 0 PCM clipped samples.
- All 9 delivery videos meet the loudness/true-peak limits after AAC encoding.
- All 9 have H.264 video and AAC audio; audio/video durations differ by at most 0.034 s. Both language caption files have 40 cues and no overlapping voice lines.
- Contact sheets visually checked for English and Hindi captions and the Hindi review watermark.
- No push, publishing, system-wide installation or visible app launch.

[Exact licences, URLs and hashes](AUDIO-SOURCES.md). [Phone recording guide](RECORD-YOUR-VOICE.md).
