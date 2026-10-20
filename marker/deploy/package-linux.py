#!/usr/bin/env python3
"""Package a verified Linux ELF with the same web/docs/notices as the portable ZIP."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import tarfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--portable-stage", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    stage = args.portable_stage.resolve(strict=True)
    with binary.open("rb") as source:
        header = source.read(64)
    if header[:6] != b"\x7fELF\x02\x01" or struct.unpack_from("<H", header, 18)[0] != 62:
        raise ValueError("Expected a little-endian x86_64 Linux ELF")
    for name in ["web/index.html", "web/wasm/beta_marker_wasm_bg.wasm", "web/wasm/beta_pattern_wasm_bg.wasm", "web/wasm/beta_marker_floor_bg.wasm", "NOTICE", "THIRD-PARTY-NOTICES.txt"]:
        if not (stage / name).is_file():
            raise ValueError(f"Missing portable release content: {name}")
    config = Path(__file__).resolve().parents[1] / "apps/desktop/src-tauri/tauri.conf.json"
    version = json.loads(config.read_text(encoding="utf-8-sig"))["version"]
    root = f"BETA-Cutting-Room_{version}_linux-x86_64"
    args.output_dir.mkdir(parents=True, exist_ok=True)
    output = args.output_dir / f"{root}.tar.gz"
    # Exclusive creation avoids replacing an earlier reviewed artifact.
    with output.open("xb") as destination, tarfile.open(fileobj=destination, mode="w:gz") as archive:
        for source in sorted(stage.rglob("*")):
            if not source.is_file() or source.relative_to(stage).as_posix() in {"beta-marker.exe", "start-server.cmd"}:
                continue
            if source.is_symlink():
                raise ValueError("Portable staging must not contain symlinks")
            archive.add(source, arcname=f"{root}/{source.relative_to(stage).as_posix()}", recursive=False)
        info = archive.gettarinfo(str(binary), arcname=f"{root}/beta-marker")
        info.mode = 0o755
        with binary.open("rb") as source:
            archive.addfile(info, source)
    print(json.dumps({"path": str(output.resolve()), "bytes": output.stat().st_size,
                      "sha256": hashlib.sha256(output.read_bytes()).hexdigest()}))


if __name__ == "__main__":
    main()
