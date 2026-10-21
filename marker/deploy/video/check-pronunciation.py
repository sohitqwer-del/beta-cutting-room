"""Optional local ASR cross-check. Recognition errors are not a listening score.

Requires the MIT whisper.cpp CLI and MIT Whisper base weights, prepared outside
git. It makes no network requests and does not play sound. See AUDIO-SOURCES.md.
"""
import argparse
import json
import re
import subprocess
from pathlib import Path


def words(text):
    text = text.lower().replace("meters", "metres").replace("meter", "metre")
    for value, spelling in {"150": "one hundred and fifty", "750": "seven hundred and fifty", "5": "five"}.items():
        text = re.sub(r"\b" + value + r"\b", spelling, text)
    return re.findall(r"[a-z]+(?:'[a-z]+)?", text)


def word_errors(reference, hypothesis):
    before = list(range(len(hypothesis) + 1))
    for i, a in enumerate(reference, 1):
        after = [i]
        for j, b in enumerate(hypothesis, 1):
            after.append(min(after[-1] + 1, before[j] + 1, before[j - 1] + (a != b)))
        before = after
    return before[-1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--whisper", type=Path, required=True)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--samples", type=Path, required=True)
    parser.add_argument("--score-existing", action="store_true", help="Score already generated transcripts without repeating recognition")
    args = parser.parse_args()
    reference = words(" ".join(line["say"] for line in json.loads((args.samples / "test-lines.json").read_text(encoding="utf8"))))
    report = []
    for name in ["heera-baseline", "kokoro-heart", "kokoro-emma", "piper-ljspeech", "kokoro-hindi-omega", "kokoro-hindi-alpha"]:
        folder = args.samples / name
        source = folder / "listen-first.wav"
        # The CLI resamples WAV itself; no new codec library is installed.
        output = folder / "asr"
        command = [str(args.whisper), "-m", str(args.model), "-f", str(source), "-t", "3", "-ng", "-l", "hi" if "hindi" in name else "en", "-otxt", "-of", str(output), "-nt"]
        if not args.score_existing:
            result = subprocess.run(command, capture_output=True, creationflags=0x08000000)
            (folder / "asr.log").write_bytes(result.stderr)
            if result.returncode:
                raise RuntimeError(result.stderr.decode("utf8", errors="replace"))
        transcript = output.with_suffix(".txt").read_text(encoding="utf8").strip()
        lowered = transcript.lower()
        terms = {term: bool(re.search(r"\b" + term + r"\b", lowered)) for term in ["beta", "marker", "nesting", "lectra", "efficiency", "tukatech"]} if "hindi" not in name else {}
        report.append({"candidate": name, "transcript": transcript, "exactEnglishTermRecognition": terms, "method": "Whisper base multilingual, local CPU, no prompt. Exact spelling is an ASR check, not proof of correct or incorrect pronunciation."})
        if "hindi" not in name:
            errors = word_errors(reference, words(transcript))
            report[-1].update(referenceWords=len(reference), wordErrors=errors, wordErrorRate=round(errors / len(reference), 4), normalization="Case/punctuation ignored; meter/metre equivalent; 150/750/5 expanded. No brand spelling corrections.")
        print(name + ": " + ascii(transcript), flush=True)
    (args.samples / "pronunciation-asr.json").write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf8")


if __name__ == "__main__":
    main()
