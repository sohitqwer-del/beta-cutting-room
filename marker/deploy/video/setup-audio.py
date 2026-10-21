"""Explicit setup-time downloads only; inference never imports this file."""
import argparse
import hashlib
import json
import os
import time
import urllib.request
from pathlib import Path


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--models", required=True, type=Path)
    parser.add_argument("--engine", choices=["all", "kokoro", "piper"], default="all")
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    manifest = json.loads(Path(__file__).with_name("audio-models.json").read_text(encoding="utf8"))
    args.models.mkdir(parents=True, exist_ok=True)
    count = 0
    for entry in manifest:
        if args.engine != "all" and entry["group"] not in ["common", args.engine]:
            continue
        target = args.models / entry["file"]
        if not (target.exists() and target.stat().st_size == entry["bytes"] and sha256(target) == entry["sha256"]):
            if args.verify_only:
                raise ValueError(f"Missing or changed file: {target}")
            partial = target.with_name(target.name + ".part")
            for attempt in range(3):
                try:
                    print(f"Download {entry['file']} ({entry['bytes']} bytes)", flush=True)
                    with urllib.request.urlopen(entry["url"], timeout=45) as response, partial.open("wb") as output:
                        while block := response.read(1024 * 1024):
                            output.write(block)
                    if partial.stat().st_size != entry["bytes"] or sha256(partial) != entry["sha256"]:
                        raise ValueError(f"Hash mismatch: {entry['file']}; keep the approved source manifest unchanged")
                    os.replace(partial, target)
                    break
                except Exception:
                    if attempt == 2:
                        raise
                    time.sleep(2)
        print(f"Verified {entry['file']}", flush=True)
        count += 1
    print(f"{count} source files verified")


if __name__ == "__main__":
    main()
