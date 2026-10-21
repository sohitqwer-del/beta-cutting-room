"""Offline CPU inference. No model downloads, eSpeak, torch, or network at runtime.

Kokoro and Piper accept phonemes directly. This small script uses the Apache-2.0
Misaki lexicons as data, plus explicit project pronunciations. Unknown words fail
instead of being dropped or sent to an online/GPL fallback. See AUDIO-SOURCES.md.
"""
import argparse
import json
import re
import subprocess
import tempfile
import wave
from pathlib import Path

import numpy as np
import onnxruntime as ort
ort.disable_telemetry_events()


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def write_wav(path, audio, rate):
    audio = np.asarray(audio).reshape(-1)
    if not np.isfinite(audio).all():
        raise ValueError("Model returned non-finite samples")
    if np.max(np.abs(audio), initial=0) >= 1:
        raise ValueError("Model output clips; inspect it before use")
    with wave.open(str(path), "wb") as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(rate)
        wav.writeframes((audio * 32767).astype("<i2").tobytes())


ONES = "zero one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen".split()
TENS = "zero ten twenty thirty forty fifty sixty seventy eighty ninety".split()


def number(n):
    n = int(n)
    if n < 20:
        return ONES[n]
    if n < 100:
        return TENS[n // 10] + (" " + ONES[n % 10] if n % 10 else "")
    if n < 1000:
        return ONES[n // 100] + " hundred" + (" and " + number(n % 100) if n % 100 else "")
    if n < 1000000:
        return number(n // 1000) + " thousand" + (" " + number(n % 1000) if n % 1000 else "")
    raise ValueError("Spell out large numbers in scenes.mjs")


def expand(text):
    text = re.sub(r"\bA4\b", "A four", text)
    text = re.sub(r"\b(\d+)\.(\d+)\b", lambda m: number(m[1]) + " point " + " ".join(ONES[int(c)] for c in m[2]), text)
    text = re.sub(r"\d+", lambda m: number(m[0]), text)
    return text.replace("’", "'").replace("-", " ")


class Lexicon:
    def __init__(self, models, british):
        region = "gb" if british else "us"
        self.words = {**read_json(models / f"{region}_silver.json"), **read_json(models / f"{region}_gold.json")}
        # BETA is spoken bay-tuh for this project. Brand pronunciations are explicit,
        # provisional editorial choices: the owner should review the sample.
        self.custom = {
            "beta": "bˈAtə", "lectra": "lˈɛktɹə", "lectra's": "lˈɛktɹəz",
            "tukatech": "tˈuːkətɛk", "rupees": "ɹuːpˈiːz", "a": "ə",
            "the": "ðə", "cad": "kˈæd", "pdf": "pˌiː dˌiː ˈɛf",
            "dxf": "dˌiː ˌɛks ˈɛf", "qr": "kjˌuː ˈɑː" if british else "kjˌuː ˈɑɹ",
            "ai": "ˌA ˈI", "record": "ɹɪkˈɔːd" if british else "ɹɪkˈɔɹd",
            "rupee": "ɹuːpˈiː", "millimetres": "mˈɪlɪmiːtəz" if british else "mˈɪləmiːtɚz",
            "centimetres": "sˈɛntɪmiːtəz" if british else "sˈɛntəmiːtɚz",
            "garments": "ɡˈɑːmənts" if british else "ɡˈɑɹmənts",
            "layouts": "lˈAaʊts", "makes": "mˈAks", "outlines": "ˈWtˌlInz",
            "tries": "tɹˈIz", "units": "jˈuːnɪts", "seconds": "sˈɛkəndz",
            "tasks": "tˈɑːsks" if british else "tˈæsks",
        }

    def __call__(self, text):
        parts = re.findall(r"[A-Za-z]+(?:'[A-Za-z]+)?|[.,!?;:]", expand(text))
        output = []
        for token in parts:
            key = token.lower()
            if key in ".,!?;:":
                output[-1:] = [output[-1] + key] if output else [key]
                continue
            value = self.custom.get(key, self.words.get(key))
            if isinstance(value, dict):
                value = value["DEFAULT"]
            if not value:
                raise ValueError(f"Unknown word {token!r}; add a reviewed pronunciation to Lexicon.custom")
            output.append(value)
        return " ".join(output)


class Engine:
    def __init__(self, models, engine, voice):
        self.engine, self.voice = engine, voice
        self.lexicon = Lexicon(models, voice.startswith("b") and engine == "kokoro")
        opts = ort.SessionOptions()
        opts.intra_op_num_threads = 3
        opts.inter_op_num_threads = 1
        opts.log_severity_level = 3
        model = "kokoro-v1.0.onnx" if engine == "kokoro" else "en_US-ljspeech-high.onnx"
        self.session = ort.InferenceSession(str(models / model), sess_options=opts, providers=["CPUExecutionProvider"])
        if engine == "kokoro":
            self.vocab = read_json(models / "config.json")["vocab"]
            with np.load(models / "voices-v1.0.bin") as voices:
                self.style = voices[voice].copy()
            self.rate = 24000
        else:
            config = read_json(models / "en_US-ljspeech-high.onnx.json")
            self.vocab = config["phoneme_id_map"]
            self.rate = config["audio"]["sample_rate"]

    def render(self, text, phonemes=None):
        phonemes = self.lexicon(text) if phonemes is None else phonemes
        if self.engine == "kokoro":
            unknown = set(phonemes) - self.vocab.keys()
            if unknown:
                raise ValueError(f"Unsupported Kokoro phonemes: {unknown}")
            tokens = [self.vocab[c] for c in phonemes]
            if len(tokens) > 500:
                raise ValueError("Split this sentence into shorter beats")
            inputs = {"tokens": np.array([[0, *tokens, 0]], dtype=np.int64),
                      "style": self.style[len(tokens)], "speed": np.array([0.90], dtype=np.float32)}
        else:
            # Expand Kokoro's compact English diphthongs to Piper's IPA inventory.
            mapping = {"A": "eɪ", "I": "aɪ", "O": "oʊ", "Q": "əʊ", "W": "aʊ", "Y": "ɔɪ", "ʤ": "dʒ", "ʧ": "tʃ", "ᵊ": "ə", "ᵻ": "ɪ"}
            phonemes = "".join(mapping.get(c, c) for c in phonemes)
            unknown = set(phonemes) - self.vocab.keys()
            if unknown:
                raise ValueError(f"Unsupported Piper phonemes: {unknown}")
            ids = [*self.vocab["^"], *self.vocab["_"]]
            for c in phonemes:
                ids.extend(self.vocab[c]); ids.extend(self.vocab["_"])
            ids.extend(self.vocab["$"])
            inputs = {"input": np.array([ids], dtype=np.int64), "input_lengths": np.array([len(ids)], dtype=np.int64),
                      "scales": np.array([0.667, 1.08, 0.333], dtype=np.float32)}
        return self.session.run(None, inputs)[0].reshape(-1), phonemes


def run(args):
    args.out.mkdir(parents=True, exist_ok=True)
    lines = read_json(args.lines)
    engine = Engine(args.models, args.engine, args.voice)
    curated = read_json(args.phonemes) if args.phonemes else None
    # Check the whole script before doing expensive inference.
    for line in lines:
        if curated:
            if curated["texts"].get(line["id"]) != line["say"]:
                raise ValueError(f"Hindi script changed: review phonemes for {line['id']}")
            if line["id"] not in {**curated["lines"], **curated["samples"]}:
                raise ValueError(f"Missing curated phonemes: {line['id']}")
        else:
            engine.lexicon(line["say"])
    report = []
    for line in lines:
        chunks, phonemes = [], []
        sentences = [line["say"]] if curated else re.split(r"(?<=[.!?])\s+(?=[A-Z])", line["say"])
        for i, sentence in enumerate(sentences):
            phones = {**curated["lines"], **curated["samples"]}[line["id"]] if curated else None
            audio, phones = engine.render(sentence, phones)
            # Trim only edge silence, preserving internal breaths and consonants.
            active = np.flatnonzero(np.abs(audio) > 0.002)
            if not len(active):
                raise ValueError(f"Silent model output: {line['id']}")
            audio = audio[max(0, active[0] - int(engine.rate * .035)):active[-1] + int(engine.rate * .065)]
            if i:
                chunks.append(np.zeros(int(engine.rate * .28), dtype=np.float32))
            chunks.append(audio); phonemes.append(phones)
        audio = np.concatenate(chunks)
        model_peak = float(np.max(np.abs(audio), initial=0))
        above_full_scale = int(np.count_nonzero(np.abs(audio) >= 1))
        # Model samples are floating point, so peaks above 1 have not yet clipped.
        # Attenuate before PCM conversion; do not saturate or limit the waveform.
        if model_peak > .95:
            audio *= .95 / model_peak
        words = len(" ".join(phonemes).split()) if curated else len(re.findall(r"[A-Za-z]+(?:'[A-Za-z]+)?", expand(line["say"])))
        target = words * 60 / args.wpm
        tempo = len(audio) / engine.rate / target
        # ffmpeg atempo preserves pitch; report the real output duration below.
        if not .5 <= tempo <= 2:
            raise ValueError(f"Unnatural tempo {tempo:.2f} for {line['id']}; revise the script")
        with tempfile.TemporaryDirectory(dir=args.out) as temp:
            raw = Path(temp) / "native.wav"
            write_wav(raw, audio, engine.rate)
            dst = args.out / f"{line['id']}.wav"
            subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", str(raw), "-af", f"atempo={tempo:.6f},aresample=48000", "-ac", "1", str(dst)], check=True, creationflags=0x08000000)
        with wave.open(str(dst)) as wav:
            duration = wav.getnframes() / wav.getframerate()
        report.append({"id": line["id"], "text": line["say"], "expanded": expand(line["say"]), "phonemes": phonemes,
                       "words": words, "seconds": round(duration, 3), "wpm": round(words * 60 / duration, 1), "tempo": round(tempo, 3)})
        report[-1].update(modelPeakDbfs=round(20 * np.log10(max(model_peak, 1e-10)), 2), modelSamplesAboveFullScale=above_full_scale)
        print(f"{line['id']}: {duration:.2f} s, {report[-1]['wpm']} wpm", flush=True)
    (args.out / "voice.json").write_text(json.dumps({"engine": args.engine, "voice": args.voice, "weightsLicense": "Apache-2.0" if args.engine == "kokoro" else "MIT", "frontend": "Hand-written Hindi IPA, needs native review" if curated else "Misaki Apache-2.0 lexicons; explicit project phonemes; no eSpeak", "targetWpm": args.wpm, "lines": report}, indent=2, ensure_ascii=False), encoding="utf8")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--models", type=Path, required=True)
    parser.add_argument("--lines", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--engine", choices=["kokoro", "piper"], default="kokoro")
    parser.add_argument("--voice", default="af_heart")
    parser.add_argument("--wpm", type=float, default=145)
    parser.add_argument("--phonemes", type=Path, help="Reviewed, script-specific Hindi IPA JSON; no general Hindi G2P fallback")
    run(parser.parse_args())
