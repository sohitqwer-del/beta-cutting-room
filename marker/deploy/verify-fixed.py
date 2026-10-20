#!/usr/bin/env python3
"""Check QA-006 through the CLI with an explicitly supplied public Simon fixture."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("job", type=Path)
    parser.add_argument("fixed", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    job = json.loads(args.job.read_text(encoding="utf-8"))
    previous = json.loads(args.fixed.read_text(encoding="utf-8"))
    if not job.get("name", "").lower().startswith("simon"):
        raise ValueError("Use only the public FreeSewing Simon QA fixture")
    locked = [p for p in previous["placements"] if p.get("locked")]
    if len(previous["placements"]) != 114 or len(locked) != 1:
        raise ValueError("Expected the 114-placement QA fixture with one locked placement")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    command = [str(binary), "nest", str(args.job), "--fixed", str(args.fixed),
               "--time", "1", "--workers", "2", "--seed", "42", "-o", str(args.output)]
    started = time.perf_counter()
    completed = subprocess.run(command, capture_output=True, text=True, timeout=15, check=True)
    elapsed = time.perf_counter() - started
    result = json.loads(args.output.read_text(encoding="utf-8"))
    validation = subprocess.run([str(binary), "validate", str(args.job), str(args.output), "--json"],
                                capture_output=True, text=True, timeout=15, check=True)
    report = json.loads(validation.stdout)
    if not report["ok"] or len(result["placements"]) != 114:
        raise AssertionError("Result must be a complete independently valid marker")
    if not all(p in result["placements"] for p in locked):
        raise AssertionError("A locked placement changed")
    if elapsed > 11:
        raise AssertionError(f"One-second search exceeded budget plus ten seconds: {elapsed:.3f}s")
    evidence = {"wall_seconds": elapsed, "valid": report["ok"], "placements": 114,
                "locked_exact": True, "length_mm": result["length"],
                "efficiency": result["efficiency"], "validation": report,
                "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                "nest_stdout": completed.stdout, "nest_stderr": completed.stderr}
    args.output.with_suffix(".check.json").write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({k: v for k, v in evidence.items() if k not in {"validation", "nest_stdout", "nest_stderr"}}))


if __name__ == "__main__":
    main()
