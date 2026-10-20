"""Writes the labelled SAMPLE benchmark files from numbers already documented in
marker/docs/benchmarks.md and marker/bench/results/.

Usage (from marker/apps/site): python -I scripts/make-sample-data.py data/benchmark

Real exports from the benchmark runner replace these files (same ids, status "final").
"""
import json
import os
import sys

out = sys.argv[1]

LECTRA_REF = {
    "name": "Lectra recorded efficiency (Lectra's own nesting engine)",
    "type": "commercial-engine",
    "citation": "Lallier, Vezard, Pinaud and Blin (2022), Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation, Zenodo 7030786, version 1.1",
    "url": "https://doi.org/10.5281/zenodo.7030786",
    "licence": "CC BY 4.0",
}
LAPTOP = "Shared 16-thread Intel laptop (i9-11900H), other programs running"
ASSUME = [
    "Sheet type 0 is an ordinary open rectangular strip of the stated width (single layer). There is no public codebook for this code.",
    "Tier B keeps a subset of the rotations and mirrors that Lectra was allowed. BETA never gets more freedom than Lectra; sometimes it gets less (a handicap for BETA).",
    "flip_x = 1 is read as a reflection of the x coordinate before rotation.",
    "Lectra's engine version, hardware and worker count are unknown. Equal seconds do not mean equal computing power.",
    "The sample balances piece count and Lectra efficiency. Its means describe the sample, not all Lectra customers.",
]
REPRO_DOC = "marker/docs/benchmarks.md"
ENGINE = "sparrow 0.3.0 on jagua-rs 1.0 (unmodified), BETA garment rules, independent validator"
SAMPLE_NOTE = "Sample data copied from the documented round-2 results (marker/docs/benchmarks.md). The benchmark runner's own export replaces this file."


def g(label, a, v, mean, med, w, t, l, ms, mds, headline=False):
    d = {"label": label, "attempts": a, "valid": v, "mean_diff_pp": mean, "median_diff_pp": med,
         "win": w, "tie": t, "loss": l, "mean_length_saving_pct": ms, "median_length_saving_pct": mds,
         "other_failures": 0}
    if headline:
        d["headline"] = True
    return d


STRATA_60 = [
    ("pieces:1-25;lectra:<75", 4.5500, 1.3748, 21, 2, 2, 6.3733, 2.0585),
    ("pieces:1-25;lectra:75-85", -0.3028, -0.1289, 10, 1, 14, -0.4449, -0.1530),
    ("pieces:1-25;lectra:85+", -0.0037, -0.0594, 11, 2, 12, -0.0819, -0.0688),
    ("pieces:26-75;lectra:<75", 0.0134, -2.9510, 5, 0, 20, -1.2788, -4.5780),
    ("pieces:26-75;lectra:75-85", -2.1274, -2.0886, 4, 2, 19, -2.7822, -2.6149),
    ("pieces:26-75;lectra:85+", -2.0264, -1.7922, 3, 1, 21, -2.4553, -2.1045),
    ("pieces:76-150;lectra:<75", -5.7194, -3.7506, 2, 0, 23, -11.0919, -5.7988),
    ("pieces:76-150;lectra:75-85", -3.6653, -3.9784, 5, 0, 20, -5.1659, -4.9527),
    ("pieces:76-150;lectra:85+", -3.3574, -3.5227, 1, 1, 23, -4.0729, -4.0488),
    ("pieces:151+;lectra:<75", -3.1013, -4.8256, 5, 1, 19, -6.1018, -7.4405),
    ("pieces:151+;lectra:75-85", -6.8475, -6.8165, 0, 0, 25, -9.3122, -9.3965),
    ("pieces:151+;lectra:85+", -5.6796, -5.1593, 1, 0, 24, -7.0384, -6.2656),
]
STRATA_MATCH = [
    ("pieces:1-25;lectra:<75", 1.3350, 1.1210, 4, 1, 0, 2.5453, 1.9308),
    ("pieces:1-25;lectra:75-85", 0.7049, 0.6426, 3, 0, 2, 0.8071, 0.7994),
    ("pieces:1-25;lectra:85+", 0.3317, 0.4585, 3, 0, 2, 0.3814, 0.5306),
    ("pieces:26-75;lectra:<75", 3.1537, 0.9933, 3, 0, 2, 4.8198, 1.6849),
    ("pieces:26-75;lectra:75-85", -0.6032, -1.3196, 2, 0, 3, -0.7680, -1.7258),
    ("pieces:26-75;lectra:85+", -2.0005, -2.5339, 0, 1, 4, -2.3846, -2.9758),
    ("pieces:76-150;lectra:<75", -2.1202, -2.0439, 1, 0, 4, -3.2929, -3.7114),
    ("pieces:76-150;lectra:75-85", -2.2826, -1.5415, 2, 0, 3, -3.5033, -1.9072),
    ("pieces:76-150;lectra:85+", -2.2933, -2.6369, 0, 0, 5, -2.7029, -3.1333),
    ("pieces:151+;lectra:<75", -1.3219, -1.1884, 1, 0, 4, -2.0320, -1.6513),
    ("pieces:151+;lectra:75-85", -4.9176, -3.6367, 0, 0, 5, -6.6060, -4.7544),
    ("pieces:151+;lectra:85+", -3.0240, -3.9271, 1, 0, 4, -3.6655, -4.8184),
]

runs = {}
runs["lectra-60s.sample.json"] = {
    "schema": "beta-site-benchmark/1", "id": "lectra-60s", "title": "Lectra dataset, 60 seconds per task",
    "kind": "lectra", "status": "sample", "status_note": SAMPLE_NOTE, "order": 10, "date": "2026-10-18",
    "summary_text": "300 real client tasks. BETA had 60 seconds per task. Lectra's figure comes from its own engine, which ran for about five minutes.",
    "software": {"beta_version": "0.2 (round 2)", "engine_revision": "3022b1285ff7", "engine": ENGINE},
    "reference": LECTRA_REF,
    "conditions": {"budget_label": "60 s per task", "budget_s": 60, "machine": LAPTOP, "jobs": 5,
                   "workers_per_task": 3, "seeds": [1], "background_load": "not controlled",
                   "notes": "Sampling seed 7030786; 25 tasks in each of 12 strata. Import, construction, repair and validation add time outside the 60 s search."},
    "assumptions": ASSUME,
    "groups": [g("Tier B (includes Tier A)", 300, 300, -2.3556, -2.9653, 68, 10, 222, -3.6211, -3.6829, True),
               g("Tier A (exact orientation sets)", 293, 293, -2.3328, -2.9024, 68, 10, 215, -3.6110, -3.6557)],
    "strata": [g(s[0], 25, 25, *s[1:]) for s in STRATA_60],
    "correlation": {"pairs": 300, "pearson": 0.8517, "spearman": 0.8559},
    "quantiles_pp": {"min": -28.8761, "p05": -9.7790, "p50": -2.9653, "p95": 5.6572, "max": 37.1418},
    "reproduce": {"commands": [
        "python -I bench/tools/lectra_convert.py download",
        "python -I bench/tools/lectra_convert.py convert --sample-size 300 --seed 7030786",
        "cargo run --release -p beta-marker-cli -- bench lectra --input <data>/lectra/sample-300.jsonl --time 60 --limit 300 --threads-per-task 3 --jobs 5 --out <new-folder>"],
        "doc": REPRO_DOC},
    "sources": [{"label": "Round-2 60-second report", "url": "marker/bench/results/2026-10-18-lectra-60s.md"},
                {"label": "Round-2 policy audit", "url": "marker/bench/results/2026-10-18-lectra-round2-policy.md"}],
}
runs["lectra-matched.sample.json"] = {
    "schema": "beta-site-benchmark/1", "id": "lectra-matched", "title": "Lectra dataset, Lectra's own time per task",
    "kind": "lectra", "status": "sample", "status_note": SAMPLE_NOTE, "order": 20, "date": "2026-10-18",
    "summary_text": "60 tasks chosen before the run. BETA had the same number of seconds that Lectra's record shows for each task (244 to 608 s, median 304 s).",
    "software": {"beta_version": "0.2 (round 2)", "engine_revision": "3022b1285ff7", "engine": ENGINE},
    "reference": LECTRA_REF,
    "conditions": {"budget_label": "Lectra's recorded time (244-608 s, median 304 s)", "budget_s": None, "machine": LAPTOP,
                   "jobs": 5, "workers_per_task": 3, "seeds": [1], "background_load": "not controlled",
                   "notes": "On the same 60 tasks at 60 s the mean was -2.5276 pp. The extra time added 1.4411 pp on average."},
    "assumptions": ASSUME,
    "groups": [g("Tier B (includes Tier A)", 60, 60, -1.0865, -1.1278, 20, 2, 38, -1.3668, -1.6003, True),
               g("Tier A (exact orientation sets)", 59, 59, -1.0788, -1.0672, 20, 2, 37, -1.3576, -1.5492),
               g("Same 60 tasks at 60 s", 60, 60, -2.5276, -2.9054, 18, 1, 41, -3.5077, -3.6243)],
    "strata": [g(s[0], 5, 5, *s[1:]) for s in STRATA_MATCH],
    "correlation": {"pairs": 60, "pearson": 0.9592, "spearman": 0.9285},
    "reproduce": {"commands": [
        "cargo run --release -p beta-marker-cli -- bench lectra --input <data>/lectra/sample-match-60.jsonl --time match --match-cap 10000 --limit 60 --threads-per-task 3 --jobs 5 --out <new-folder>"],
        "doc": REPRO_DOC},
    "sources": [{"label": "Round-2 matched-duration report", "url": "marker/bench/results/2026-10-18-lectra-match.md"}],
}
PAPER = {  # instance: (paper mean %, mean diff, median diff, mean saving %, median saving %)
    "albano": (89.47, -1.5552, -1.8648, -1.7730, -2.1287),
    "dagli": (89.26, -3.8187, -4.3730, -4.4805, -5.1516),
    "mao": (86.14, -2.3822, -2.3875, -2.8443, -2.8507),
    "marques": (90.93, -1.3011, -1.8033, -1.4595, -2.0233),
    "shirts": (89.66, -2.4101, -2.3621, -2.7624, -2.7058),
    "swim": (78.26, -3.5178, -3.6224, -4.7075, -4.8534),
    "trousers": (91.73, -1.6258, -1.5825, -1.8045, -1.7555),
}
acad_groups = [g("All 7 instances, 3 seeds each", 21, 21, -2.3730, -2.3398, 0, 0, 21, -2.8331, -2.7058, True)]
for name, (ref, mean, med, ms, mds) in PAPER.items():
    row = g(name, 3, 3, mean, med, 0, 0, 3, ms, mds)
    row["mean_reference_eff_pct"] = ref
    row["mean_beta_eff_pct"] = round(ref + mean, 4)
    acad_groups.append(row)
runs["academic-60s.sample.json"] = {
    "schema": "beta-site-benchmark/1", "id": "academic-60s", "title": "Academic nesting instances, 60 seconds",
    "kind": "academic", "status": "sample", "status_note": SAMPLE_NOTE, "order": 30, "date": "2026-10-18",
    "summary_text": "Seven standard research instances, including shirts, trousers and swimwear. BETA had 60 seconds. The paper's means used 1200 seconds on a faster desktop CPU.",
    "software": {"beta_version": "0.2 (round 2)", "engine_revision": "3022b1285ff7", "engine": "sparrow 0.3.0 on jagua-rs 1.0 (unmodified)"},
    "reference": {"name": "sparrow paper means (100 runs, 1200 s, 3 threads, AMD Ryzen 9 7950X)", "type": "published-paper",
                  "citation": "Gardeyn, Vanden Berghe and Wauters, An open-source heuristic to reboot 2D nesting research, arXiv:2509.13329v4, Tables 2-3",
                  "url": "https://arxiv.org/abs/2509.13329"},
    "conditions": {"budget_label": "60 s per run (paper: 1200 s)", "budget_s": 60, "machine": LAPTOP, "jobs": 5,
                   "workers_per_task": 3, "seeds": [1, 2, 3], "background_load": "not controlled"},
    "assumptions": ["BETA uses sparrow itself, so this measures BETA's adapter and the shorter time. It is not an independent algorithm comparison.",
                    "No garment rules or gaps are added. Instance files are the MIT-licensed sparrow v0.3.0 files."],
    "groups": acad_groups,
    "reproduce": {"commands": ["cargo run --release -p beta-marker-cli -- bench academic --time 60 --seeds 1,2,3 --threads-per-task 3 --jobs 5 --out <new-folder>"],
                  "doc": REPRO_DOC},
    "sources": [{"label": "Round-2 academic report", "url": "marker/bench/results/2026-10-18-academic-round2-60s.md"},
                {"label": "Reference values (CSV)", "url": "marker/bench/academic-references.csv"}],
}
FS = [("simon", 88.1897, 64.2858), ("teagan", 76.5062, 62.2702), ("titan", 75.3237, 62.7144)]
runs["freesewing-60s.sample.json"] = {
    "schema": "beta-site-benchmark/1", "id": "freesewing-60s", "title": "FreeSewing garments, 60 seconds",
    "kind": "freesewing", "status": "sample",
    "status_note": "Sample data copied from marker/bench/results/2026-10-19-v03-freesewing.md. The benchmark runner's own export replaces this file.",
    "order": 40, "date": "2026-10-19",
    "summary_text": "Three open-source garment patterns (Simon shirt, Teagan T-shirt, Titan trousers) on open fabric 1500 mm wide. There is no commercial reference for these jobs. They show what BETA reaches on whole garments with garment rules.",
    "software": {"beta_version": "0.3 (development)", "engine_revision": None, "engine": ENGINE},
    "reference": {"name": "None (BETA results only)", "type": "none"},
    "conditions": {"budget_label": "60 s per job", "budget_s": 60, "machine": LAPTOP, "jobs": 2, "workers_per_task": 3,
                   "seeds": [1], "background_load": "CPU shared and saturated"},
    "assumptions": ["Fabric is open, single layer, 1500 mm wide. Size ratio as in the job files."],
    "groups": [{"label": "All three jobs", "headline": True, "attempts": 3, "valid": 3, "mean_diff_pp": None,
                "median_diff_pp": None, "win": None, "tie": None, "loss": None,
                "mean_beta_eff_pct": round(sum(i[1] for i in FS) / 3, 4)}],
    "items": [{"id": n, "seed": 1, "valid": True, "beta_eff_pct": e, "runtime_s": r} for n, e, r in FS],
    "reproduce": {"commands": ["./bench/tools/quality_confirmation.ps1 -Executable <beta-marker.exe> -EvidenceRoot <new-folder> -Jobs 2 -Workers 3"],
                  "doc": "marker/bench/results/2026-10-19-v03-handoff.md"},
    "sources": [{"label": "FreeSewing 60-second report", "url": "marker/bench/results/2026-10-19-v03-freesewing.md"}],
}

for name, run in runs.items():
    with open(os.path.join(out, name), "w", encoding="utf-8", newline="\n") as f:
        json.dump(run, f, indent=2, ensure_ascii=False)
        f.write("\n")
print("wrote", len(runs), "files")
