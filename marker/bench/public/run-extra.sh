#!/usr/bin/env bash
# Run only the extra suites after the existing Lectra process has completed.
# Arguments: marker checkout, built CLI, external data root, fresh output, STATUS file.
set -euo pipefail
if [[ $# != 5 ]]; then
  echo 'usage: run-extra.sh <marker-dir> <executable> <data-dir> <fresh-out> <lectra-STATUS>' >&2
  exit 2
fi
marker=$(cd "$1" && pwd)
exe=$(realpath "$2")
export BETA_DATA_DIR=$(cd "$3" && pwd)
out=$4
status=$5
[[ $(cat "$status") == done ]] || { echo 'Lectra STATUS is not done; no search started.' >&2; exit 1; }
[[ ! -e "$out" ]] || { echo 'Use a fresh output folder.' >&2; exit 1; }
for id in shirts trousers swim albano dagli mao marques; do
  test -f "$BETA_DATA_DIR/datasets/esicup-sparrow/$id.json"
done
mkdir -p "$out"
out=$(cd "$out" && pwd)
cd "$marker"
echo running > "$out/STATUS"
(
  while true; do
    date -u +'%Y-%m-%dT%H:%M:%SZ' >> "$out/load.log"
    uptime >> "$out/load.log"
    sleep 60
  done
) &
loadpid=$!
finish() {
  result=$?
  kill "$loadpid" 2>/dev/null || true
  if [[ $result != 0 ]]; then echo failed > "$out/STATUS"; fi
}
trap finish EXIT
common=(--jobs 4 --threads-per-task 3)
nice -n 10 "$exe" bench academic --time 60 --seeds 1,2,3 \
  --search-config bench/experiments/v03/selected.json --runs 1 "${common[@]}" \
  --out "$out/academic-60s" > "$out/academic-60s.log" 2>&1
inputs="$marker/testdata/open/freesewing/simon-ratio.job.json,$marker/testdata/open/freesewing/teagan-ratio.job.json,$marker/testdata/open/freesewing/titan-ratio.job.json"
for seconds in 60 300; do
  nice -n 10 "$exe" bench jobs --inputs "$inputs" --time "$seconds" --seed 1 \
    "${common[@]}" --out "$out/freesewing-${seconds}s" > "$out/freesewing-${seconds}s.log" 2>&1
done
echo done > "$out/STATUS"
