# Site benchmark data: schema `beta-site-benchmark/1`

The Benchmark page of the BETA Cutting Room website reads every `*.json` file in this
folder at build time. Vite inlines the data into the page, so the built site still works
when opened from disk. No file is fetched at runtime.

One file = one benchmark run (one dataset, one time budget). Example names:
`lectra-60s.json`, `lectra-matched.json`, `lectra-server-60s.json`, `academic-60s.json`,
`freesewing-60s.json`.

The loader is `marker/apps/site/src/benchmark.ts`. Run its tests (they also check every
file in this folder) from the repository root:

```sh
npx vitest run marker/apps/site
```

## Rules

- **Aggregates and task IDs only.** No geometry, outlines, coordinates or raw dataset rows.
  The loader rejects any object key named `geometry`, `points`, `polygon`, `polygons`,
  `outline`, `outlines`, `vertices`, `shape`, `shapes` or `coordinates`.
- **No private factory data.** Factory styles, buyers and factory names never go here.
- **Differences are BETA minus reference**, in percentage points (pp). Negative means BETA
  used more fabric than the reference.
- **Incomplete runs are `"status": "provisional"`** and must give `progress` and
  `status_note`. The page shows a clear "Provisional" badge and the completed count.
- **Sample data is `"status": "sample"`.** The page labels it "Sample data copied from
  documented results". Replace sample files with real exports when they exist.
- **Same `id` in several files:** if any file is `final`, only the newest `final` file
  (by `date`) is shown. Otherwise the page shows at most one `sample` and one
  `provisional` file for that id (the provisional one with the most completed tasks,
  then the newest). So a provisional server run appears next to the earlier complete
  result until the final file arrives.
- **Extensions are allowed.** Unknown fields are ignored by the page (but still checked
  for geometry keys). Put producer-specific data (runtime, load, provenance) in extra
  fields, preferably under an `x_` prefix or an `extensions` object.
- Numbers are plain JSON numbers (not strings). Use `null` for "not measured".
- Dates are ISO `YYYY-MM-DD`.

## Top-level object

| Field | Type | Required | Meaning |
|---|---|---|---|
| `schema` | `"beta-site-benchmark/1"` | yes | Version tag. Other values are rejected. |
| `id` | string, `[a-z0-9-]+` | yes | Stable ID, e.g. `lectra-60s`. |
| `title` | string | yes | Short plain-English title, e.g. "Lectra dataset, 60 seconds". |
| `kind` | `"lectra"`, `"academic"`, `"freesewing"`, `"other"` | yes | Which section of the page shows it. |
| `status` | `"final"`, `"provisional"`, `"sample"` | yes | See rules above. |
| `status_note` | string | if not `final` | Why it is not final, e.g. "Server run in progress: 126 of 300 tasks". |
| `order` | number | no | Sort key inside its section (low first). Default 100. |
| `date` | `YYYY-MM-DD` | yes | Date of the run (end date). |
| `summary_text` | string | no | One or two plain sentences shown above the table. |
| `progress` | `{ "completed": int, "planned": int }` | if `provisional` | Tasks finished so far and tasks planned. |
| `software` | object | yes | See below. |
| `reference` | object | yes | What BETA is compared with. See below. |
| `conditions` | object | yes | Time budget and machine. See below. |
| `assumptions` | string[] | yes (may be empty) | Each item is one plain sentence. |
| `groups` | Group[] | yes, 1 or more | Aggregate rows. See below. |
| `strata` | Group[] | no | Per-stratum rows (same shape as groups). |
| `items` | Item[] | no | Per-task or per-instance rows. Task IDs only. |
| `task_ids` | string[] | no | IDs of the tasks in this run (for reproduction), when per-task scores are not published. |
| `correlation` | object | no | `{ "pairs": int, "pearson": number, "spearman": number }` |
| `quantiles_pp` | object | no | `{ "min", "p05", "p50", "p95", "max" }`, all numbers, of the per-task difference. |
| `reproduce` | object | yes | `{ "commands": string[], "doc": string }` - `doc` is a repository-relative path or URL. |
| `sources` | `{ "label": string, "url": string }[]` | no | Reports and citations. |

### `software`

| Field | Type | Required |
|---|---|---|
| `beta_version` | string, e.g. `"0.3.0"` | yes |
| `engine_revision` | string (git short hash) or `null` | yes |
| `engine` | string, e.g. `"sparrow 0.3.0 on jagua-rs 1.0 (unmodified)"` | no |

### `reference`

| Field | Type | Required | Meaning |
|---|---|---|---|
| `name` | string | yes | e.g. "Lectra recorded efficiency", "sparrow paper means (1200 s)". |
| `type` | `"commercial-engine"`, `"published-paper"`, `"none"` | yes | `none` = BETA results only (no comparison). |
| `citation` | string | no | Full citation. |
| `url` | string | no | DOI or link. |
| `licence` | string | no | e.g. `"CC BY 4.0"`. |

### `conditions`

| Field | Type | Required | Meaning |
|---|---|---|---|
| `budget_label` | string | yes | e.g. "60 s", "Lectra's recorded time (244-608 s, median 304 s)". |
| `budget_s` | number or `null` | yes | Fixed search budget in seconds; `null` if per-task. |
| `machine` | string | yes | e.g. "Shared 16-thread Intel laptop (i9-11900H)", "Shared 24-core Linux server". No host names or IP addresses. |
| `jobs` | int | no | Tasks run at the same time. |
| `workers_per_task` | int | no | Search threads per task. |
| `seeds` | int[] | no | Random seeds. |
| `background_load` | string | no | e.g. "not controlled". |
| `notes` | string | no | Anything else a reader needs. |

### Group (rows in `groups` and `strata`)

| Field | Type | Required | Meaning |
|---|---|---|---|
| `label` | string | yes | e.g. "Tier B (includes Tier A)". For strata use `pieces:1-25;lectra:<75`. |
| `task_ids` | string[] | no | Task IDs in this group. |
| `headline` | boolean | no | At most one group per file may be `true`. Shown large. |
| `attempts` | int | yes | Tasks attempted. |
| `valid` | int | yes | Layouts that passed the independent validator. `valid <= attempts`. |
| `mean_diff_pp` | number or `null` | yes | Mean of BETA minus reference, valid tasks only. `null` if `reference.type` is `none`. |
| `median_diff_pp` | number or `null` | yes | Median of the same. |
| `win` / `tie` / `loss` | int or `null` | yes | Valid layouts only. Tie = within 0.1 pp. When not null, `win + tie + loss = valid`. |
| `too_long_losses` | int | no | Failed `too_long` layouts (marker exceeds a finite sheet length). They count as losses in the Markdown report; the page shows them next to `loss`. Default 0. |
| `mean_length_saving_pct` | number or `null` | no | 100 x (1 - reference eff / BETA eff), mean per task. |
| `median_length_saving_pct` | number or `null` | no | Median of the same. |
| `other_failures` | int | no | Failed layouts other than `too_long`. Default 0. |
| `mean_beta_eff_pct` | number or `null` | no | Mean BETA efficiency, %. |
| `mean_reference_eff_pct` | number or `null` | no | Mean reference efficiency, %. |

### Item (rows in `items`)

| Field | Type | Required | Meaning |
|---|---|---|---|
| `id` | string | yes | Task ID (e.g. Lectra task id `"63488"`) or instance name (`"shirts"`). |
| `group` | string | no | Stratum label or instance family. |
| `seed` | int | no | Seed for this row. |
| `valid` | boolean | yes | Passed the independent validator. |
| `beta_eff_pct` | number or `null` | yes | `null` if not valid. |
| `reference_eff_pct` | number or `null` | no | Reference efficiency, %. |
| `diff_pp` | number or `null` | no | BETA minus reference. |
| `runtime_s` | number or `null` | no | Whole nesting-call time. |
| `failure` | string | no | e.g. `"too_long"`, `"construction_failed"`. |

## Minimal example

```json
{
  "schema": "beta-site-benchmark/1",
  "id": "lectra-server-60s",
  "title": "Lectra dataset, 60 seconds (server)",
  "kind": "lectra",
  "status": "provisional",
  "status_note": "Server run in progress. Numbers will change.",
  "progress": { "completed": 126, "planned": 300 },
  "date": "2026-10-19",
  "software": { "beta_version": "0.4.0", "engine_revision": "9461f54" },
  "reference": { "name": "Lectra recorded efficiency", "type": "commercial-engine",
                 "url": "https://doi.org/10.5281/zenodo.7030786", "licence": "CC BY 4.0" },
  "conditions": { "budget_label": "60 s", "budget_s": 60, "machine": "Shared 24-core Linux server" },
  "assumptions": ["Sheet type 0 is an ordinary open rectangular strip."],
  "groups": [
    { "label": "Tier B (includes Tier A)", "headline": true, "attempts": 126, "valid": 126,
      "mean_diff_pp": -0.71, "median_diff_pp": null, "win": null, "tie": null, "loss": null }
  ],
  "reproduce": { "commands": ["see doc"], "doc": "marker/docs/benchmarks.md" }
}
```

(The numbers above illustrate the format only.)
