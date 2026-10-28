# Reproduce the public benchmark

Run from `marker/`. Rust 1.90+, Cargo and a C/C++ toolchain are required. The
one-time Lectra conversion also needs Python 3.11 and about 9 GB of free RAM.
Keep `BETA_DATA_DIR` and `CARGO_TARGET_DIR` outside the checkout. Set
`CARGO_BUILD_JOBS=3` when building on a shared machine.

The recorded server engine is commit `a2d5de276e5d679acd00a023476ed84209924c57`.
The final public runner is pinned at `0ecfd71`, whose CPU nesting, matching,
validator, benchmark runner and selected search settings are identical to that
server revision. Use a fresh checkout at `0ecfd71` to reproduce that baseline.
Later commits add the report and website data. Running the command
on newer engine code measures that newer revision; it is not a replay of the
recorded executable. The historical laptop results used a different, locally
modified round-2 source and the earlier converter.

## Paired 0.8 candidate versus 0.7.0

The [28 October paired report](../results/2026-10-28-public-0.8.md) uses
`beta-launch8-bench` in this package. It compares the frozen 300-task, 60-second
Lectra suite and 18 public garment variants on both release engines. It keeps
the older results and their version labels intact. Build each engine in its
own fresh target directory: Cargo can reuse stale artifacts across source
archives with preserved timestamps if they share a target directory.

The tool checks the frozen input checksum, writes common replay inputs before
searching, alternates engine order in 20-task Lectra blocks, and exports only
after all stages finish. Export checks every scored marker against its saved
independent validator report and recomputed metrics. See the report for exact
commands, whole-call timing, effective process priority and limitations.

## Prepare public inputs once

This preparation downloads public data. The benchmark itself makes no network
requests after its executable and inputs have been prepared.

Create a virtual environment outside the checkout, then run its Python with `-I`:

```sh
python -m venv "$BETA_DATA_DIR/venvs/marker-bench"
PYTHON="$BETA_DATA_DIR/venvs/marker-bench/bin/python"
"$PYTHON" -I -m pip install --no-cache-dir -r bench/tools/requirements.txt
"$PYTHON" -I bench/tools/lectra_convert.py download
"$PYTHON" -I bench/tools/lectra_convert.py inspect
"$PYTHON" -I bench/tools/lectra_convert.py self-test
"$PYTHON" -I bench/tools/lectra_convert.py convert --sample-size 300 --seed 7030786 \
  --reference-sample bench/public/sample-300.ids.jsonl \
  --reference-match bench/public/sample-match-60.ids.jsonl \
  --out "$BETA_DATA_DIR/marker-bench/public-inputs"
```

On Windows use `Scripts/python.exe` in the venv and PowerShell's `&` call
operator. Use backticks for continued commands, or put each command on one line.
The two reference files contain only public task IDs. They preserve the
confirmation sets from round 2; re-sampling the v0.3 eligible population would
produce a different suite. Conversion retains optional mirror states, uses
per-piece tilt and converts coordinates to mm. The four source MD5 checksums
and restricted unpickler are documented in [the audit](../lectra/README.md).

Download the seven academic JSON inputs listed in [inputs.json](inputs.json)
to `$BETA_DATA_DIR/datasets/esicup-sparrow/<id>.json`. Each URL points to
Sparrow tag v0.3.0. Check each SHA-256 against that manifest. For example:

```sh
mkdir -p "$BETA_DATA_DIR/datasets/esicup-sparrow"
for id in shirts trousers swim albano dagli mao marques; do
  curl --fail --location "https://raw.githubusercontent.com/JeroenGar/sparrow/v0.3.0/data/input/$id.json" \
    --output "$BETA_DATA_DIR/datasets/esicup-sparrow/$id.json"
done
```

The runner verifies these academic hashes before starting. FreeSewing Simon,
Teagan and Titan ratio jobs are already in `testdata/open/freesewing/`, under
MIT. They use open, 1500 mm fabric, sizes S1 M2 L2 XL1, zero gap and no nap.

## One command

```sh
cargo run --release --manifest-path bench/public/Cargo.toml -- --out "$BETA_DATA_DIR/marker-bench/my-public-run"
```

The tool builds `beta-marker-cli` first, unless `--executable <path>` is supplied.
It runs 300 Lectra tasks at 60 seconds, the fixed 60-task subset at each recorded
duration (244-608 seconds, no cap), seven academic instances at seeds 1/2/3 and
60 seconds, then Simon/Teagan/Titan at 60 and 300 seconds. Lectra and garments
use seed 1. All stages run once; no best-of-several selection is applied.

The default worker count is `min(3, logical CPUs)`. Task concurrency defaults
to `min(4, logical CPUs / workers)`; override with `--jobs` and `--workers`.
The server protocol used four jobs with three search workers, at nice 10.
Worker counts exclude coordinator/helper threads. Avoid competing builds.
On Linux prefix the command with `nice -n 10` for the same priority policy.

Use `--quick` for the first 20 frozen Lectra IDs, each at 30 seconds. This
checks the pipeline; it is neither a balanced sample nor a launch score.
`--time-scale 0.5` halves every budget, including the recorded matched times.
Changed budgets are labelled, and must not replace the launch numbers.
`--suite extras` runs only academic and FreeSewing stages.
`--input-dir <dir>` selects an existing conversion directory.

Each stage writes the existing runner's CSV, summary, job, result and independent
validation evidence. These files include geometry and stay outside git. The
tool writes geometry-free JSON files to `<out>/site/` using the website's
`beta-site-benchmark/1` schema, plus `<out>/analysis.md`. It verifies every
scored marker against its saved independent validation report. Failures stay
in the counts. `too_long` failures count as losses in the report, and are
separate from valid-layout losses in website JSON to satisfy its count rule.

To export collected results without rerunning searches:

```sh
cargo run --release --manifest-path bench/public/Cargo.toml -- --out "$BETA_DATA_DIR/marker-bench/my-public-run" --summarize
```

All five stages must be complete. `--allow-partial` exports only existing
stages and labels incomplete stages `provisional`. CSV completion order is
not the sampling order and partial means can be biased toward easier tasks.
Never substitute partial numbers for final results.

## Meaning and limits

Efficiencies and differences use valid markers only. Difference is BETA minus
reference, in percentage points. Ties are within 0.1 point. Length saving is
`100 * (1 - reference efficiency / BETA efficiency)`, averaged per task.
Tier B includes Tier A. Both depend on the unverified assumption that
sheet type 0 means an ordinary open rectangular strip and flip_x means
x-coordinate reflection before rotation. Unsupported source constraints and
unverified rings stay excluded. See [benchmark policy](../../docs/benchmarks.md).

The stratified sample is unweighted, not a population estimate. Equal seconds
do not equal equal compute. Lectra's hardware, version, thread count and precise
stopping rule are unknown. Academic references are Sparrow paper means from
100 runs at 1200 seconds on a Ryzen 9 7950X; BETA uses Sparrow itself.
FreeSewing has no commercial comparison. Wall-clock search cutoffs mean seeds
do not make results bit-for-bit deterministic across machines.

Lectra attribution: Corentin Lallier, Laurent Vezard, Bruno Pinaud and Guillaume
Blin (2022), *Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation*,
[Zenodo 7030786](https://doi.org/10.5281/zenodo.7030786), version 1.1,
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Conversion changes
units and sometimes restricts orientations. No geometry is published here.

The orchestration tool adds no product dependencies. Its dependencies are
anyhow (MIT/Apache-2.0), clap (MIT/Apache-2.0), serde_json (MIT/Apache-2.0), and
sha2 (MIT/Apache-2.0), all permissive.
