# Audio sources and licences

Only local CPU inference runs during rendering. Downloads happen in the separate setup step. Model files and audio stay outside git. The manifest `audio-models.json` pins each byte hash; `setup-audio.py --verify-only` checks it without a network request.

## Models and frontend data

- Kokoro weights and voice styles: Apache-2.0, as declared by the [official Hexgrad model card](https://huggingface.co/hexgrad/Kokoro-82M). The ONNX files come from the kokoro-onnx maintainer's model release. The exporter project is MIT. English uses `af_heart`; review alternatives are `bf_emma`, Hindi `hm_omega` and `hf_alpha`.
- Piper LJ Speech: MIT in the official piper-voices repository metadata; this specific model card names public-domain LJ Speech training data and no restrictive override. This does **not** clear other Piper voices.
- The original rhasspy/Piper engine is MIT. This pipeline does not install it or the newer GPL engine. It runs the ONNX graph directly with original inference glue. It does not install or call eSpeak NG, piper-phonemize, Misaki Python code, num2words, phonemizer, torch, or a cloud API.
- English phonemes use the Apache-2.0 Misaki dictionaries as data, with explicit project pronunciations and hard failures for unknown words. This limited frontend has no statistical part-of-speech model; review homographs when editing the script.
- Hindi uses original, hand-written IPA in `hindi-phonemes.json`, bound to the exact caption draft. **Hindi script needs native review**, including these phonemes and numbers. This is a demo-specific frontend, not a general Hindi TTS service.

| Download (official publisher or exporter) | Licence | SHA-256 |
|---|---|---|
| [kokoro-v1.0.onnx](https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.onnx) | Apache-2.0 | `7d5df8ecf7d4b1878015a32686053fd0eebe2bc377234608764cc0ef3636a6c5` |
| [voices-v1.0.bin](https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin) | Apache-2.0 | `bca610b8308e8d99f32e6fe4197e7ec01679264efed0cac9140fe9c29f1fbf7d` |
| [config.json](https://huggingface.co/hexgrad/Kokoro-82M/resolve/f3ff3571791e39611d31c381e3a41a3af07b4987/config.json) | Apache-2.0 | `5abb01e2403b072bf03d04fde160443e209d7a0dad49a423be15196b9b43c17f` |
| [en_US-ljspeech-high.onnx](https://huggingface.co/rhasspy/piper-voices/resolve/5041a7a49ae5b3a0f30e6837769eb03fdbb231bd/en/en_US/ljspeech/high/en_US-ljspeech-high.onnx) | MIT | `5d4f08ba6a2a48c44592eed3ce56bf85e9de3dd4e20df90541ae68a8310c029a` |
| [en_US-ljspeech-high.onnx.json](https://huggingface.co/rhasspy/piper-voices/resolve/5041a7a49ae5b3a0f30e6837769eb03fdbb231bd/en/en_US/ljspeech/high/en_US-ljspeech-high.onnx.json) | MIT | `7e1f4634af596d83cca997fb7a931ba80b70f8a316a2655ee69c55365e0ace14` |
| [PIPER-MODEL-CARD](https://huggingface.co/rhasspy/piper-voices/resolve/5041a7a49ae5b3a0f30e6837769eb03fdbb231bd/en/en_US/ljspeech/high/MODEL_CARD) | MIT repository; public-domain LJ Speech data | `289f7421072d689a3d91f6c632f486af46f9b8b04417536104a4ea667b8a394b` |
| [PIPER-README.md](https://huggingface.co/rhasspy/piper-voices/resolve/5041a7a49ae5b3a0f30e6837769eb03fdbb231bd/README.md) | MIT declaration | `33e93643fce5180886300f9f0070eeeab8af148efb8e84361f8904def80fd9fb` |
| [KOKORO-MODEL-CARD.md](https://huggingface.co/hexgrad/Kokoro-82M/resolve/f3ff3571791e39611d31c381e3a41a3af07b4987/README.md) | Apache-2.0 declaration | `91dcabced89db6f109b8786642f50402d3ee87450e8189589b6f85520e7f4d78` |
| [KOKORO-VOICES.md](https://huggingface.co/hexgrad/Kokoro-82M/resolve/f3ff3571791e39611d31c381e3a41a3af07b4987/VOICES.md) | Apache-2.0 | `ec7e4941ad7e194af61e3455928528a9ff5360c7c505e412efab27d6a69ea106` |
| [MISAKI-LICENSE](https://raw.githubusercontent.com/hexgrad/misaki/fba1236595f2d2bf21d414ba6e57d25256afada3/LICENSE) | Apache-2.0 | `c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4` |
| [gb_gold.json](https://raw.githubusercontent.com/hexgrad/misaki/fba1236595f2d2bf21d414ba6e57d25256afada3/misaki/data/gb_gold.json) | Apache-2.0 | `29e62f4b60261c88f7f3c2c7811ca3825978948090b72d2b27d565b729282f71` |
| [gb_silver.json](https://raw.githubusercontent.com/hexgrad/misaki/fba1236595f2d2bf21d414ba6e57d25256afada3/misaki/data/gb_silver.json) | Apache-2.0 | `48131e2d92ccc41655f4543e87e0f938e71463eb5a54be7f0693bb712ebb6bce` |
| [us_gold.json](https://raw.githubusercontent.com/hexgrad/misaki/fba1236595f2d2bf21d414ba6e57d25256afada3/misaki/data/us_gold.json) | Apache-2.0 | `dc414872a49a28ae6c141463d502fd945f3b2fde040484fdc47d00cc4612686f` |
| [us_silver.json](https://raw.githubusercontent.com/hexgrad/misaki/fba1236595f2d2bf21d414ba6e57d25256afada3/misaki/data/us_silver.json) | Apache-2.0 | `de8f67be911bb6c659187b4a65fd966b6a30e56350e0f790d763210b053ac475` |
| [KOKORO-CODE-LICENSE](https://raw.githubusercontent.com/hexgrad/kokoro/main/LICENSE) | Apache-2.0 | `c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4` |
| [ONNX-EXPORT-LICENSE](https://raw.githubusercontent.com/thewh1teagle/kokoro-onnx/main/LICENSE) | MIT | `f61fdd0217de9ece8fa699912848d91187d84879fc2e689f0906d0c1299e955e` |
| [PIPER-CODE-LICENSE](https://raw.githubusercontent.com/rhasspy/piper/master/LICENSE.md) | MIT | `4cd71dece7037f1d6d93cce7570c57ab75ea9ac566fd4990be2f3ab08d15b47f` |
| [ONNXRUNTIME-LICENSE](https://raw.githubusercontent.com/microsoft/onnxruntime/v1.23.2/LICENSE) | MIT | `2f07c72751aed99790b8a4869cf2311df85a860b22ded05fa22803587a48922c` |

## Runtime tools

Python 3.11 is already installed. Install only `requirements-audio.txt` in a dedicated external venv. Nothing changes system Python. Packages are pinned below and in that file. Existing ffmpeg/ffprobe, Node and the existing headless recording tools are reused; this change adds no npm dependency. No new binary encoder is downloaded.

| Package | Version | Licence |
|---|---|---|
| onnxruntime | 1.23.2 | MIT |
| numpy | 2.3.5 | BSD-3-Clause |
| coloredlogs | 15.0.1 | MIT |
| flatbuffers | 25.12.19 | Apache-2.0 |
| packaging | 26.3 | Apache-2.0 / BSD-2-Clause |
| protobuf | 7.36.2 | BSD-3-Clause |
| sympy | 1.14.0 | BSD-3-Clause |
| humanfriendly | 10.0 | MIT |
| mpmath | 1.3.0 | BSD-3-Clause |
| pyreadline3 | 3.5.6 | BSD-3-Clause |

Microsoft Heera was used only as the existing, installed Windows baseline. Its voice files are not redistributed. No new Microsoft voice was installed. SoundFile and its LGPL libsndfile payload were removed from the test venv; WAV I/O uses Python's standard library. No non-permissive speech dependency remains installed in that venv.

## Optional offline pronunciation cross-check

`check-pronunciation.py` runs Whisper base with no prompt, on CPU, against the six review
playlists. It never plays audio. The executable is kept inside the external audio venv;
weights stay in the external models folder. This is an intelligibility cross-check, not
a human naturalness score. Brand spellings can be wrong even when a person would understand.
The official binary archive also contains SDL2 (zlib licence); the CLI uses the MIT
whisper/ggml DLLs. No system installation is needed.

| File | Licence | SHA-256 |
|---|---|---|
| [whisper-bin-x64.zip, v1.7.6](https://github.com/ggml-org/whisper.cpp/releases/download/v1.7.6/whisper-bin-x64.zip) | MIT; bundled SDL2 zlib | `0d2eca299c248f965bd0341bcb219db4b433c7f0c0ce2200d4df85765e8156a9` |
| [ggml-base.bin, whisper.cpp maintainer conversion](https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin) | MIT, [OpenAI Whisper weights](https://github.com/openai/whisper#license) | `60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe` |
| [Whisper.cpp licence](https://raw.githubusercontent.com/ggml-org/whisper.cpp/v1.7.6/LICENSE) | MIT | `e562a2ddfaf8280537795ac5ecd34e3012b6582a147ef69ba6a6a5c08c84757d` |

```sh
<audio-data>/venv/Scripts/python.exe marker/deploy/video/check-pronunciation.py --whisper <audio-data>/venv/whisper-cpp/Release/whisper-cli.exe --model <audio-data>/models/ggml-base.bin --samples <audio-data>/samples
```

## Setup (network permitted here only)

```powershell
py -3.11 -m venv <audio-data>/venv
& <audio-data>/venv/Scripts/python.exe -m pip install --no-cache-dir -r marker/deploy/video/requirements-audio.txt
& <audio-data>/venv/Scripts/python.exe marker/deploy/video/setup-audio.py --models <audio-data>/models
& <audio-data>/venv/Scripts/python.exe marker/deploy/video/setup-audio.py --models <audio-data>/models --verify-only
```

Use `--engine kokoro` or `--engine piper` to prepare just that model family. The default prepares both for comparison. Inference files contain no network client. Copy the venv and model folder to an offline machine with the same Python/Windows architecture, or prepare a local wheelhouse first. Keep all downloaded licence and model-card files beside the weights.
