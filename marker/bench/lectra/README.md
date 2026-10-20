# Lectra dataset audit and conversion

Source: Corentin Lallier, Laurent Vézard, Bruno Pinaud and Guillaume Blin (2022), *Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation*, Zenodo record [7030786](https://doi.org/10.5281/zenodo.7030786), version 1.1, CC BY 4.0. The record notes that this version swaps the previously incorrect `c8` and `g4` constraint labels. Do not substitute an older record silently.

The associated paper is [*Graph neural network comparison for 2D nesting efficiency estimation*](https://doi.org/10.1007/s10845-023-02084-6), published online in 2023 and in volume 35 (2024). The complete 28-page HAL manuscript was reviewed through [its file link](https://hal.science/hal-03952756/file/main.pdf). The author's [dataset card](https://huggingface.co/datasets/clallier/nesting-tasks-2d) was also checked. Only the original Zenodo pickle files are converted here.

## Safety and integrity

Downloads go to `$BETA_DATA_DIR/datasets/lectra-nesting-7030786/`. The four checksums below came from the Zenodo API and were verified against the actual downloads on 18 October 2026. The converter verifies them again before loading. MD5 detects download corruption; the records are fetched over HTTPS.

| File | Bytes | Zenodo MD5 |
|---|---:|---|
| tasks.gz | 1,208,474 | ac18fc58408a3fc832cfd6757b4b16ca |
| parts.gz | 22,476,589 | d8b51403f0cab79ec990b95a40911c1c |
| constraints.gz | 16,299,551 | e12581851bd2a357145a9dfccdad5363 |
| shapes.gz | 147,824,458 | ff1623f24adf031710450a30e72984f2 |

Run `lectra_convert.py` with Python 3.11 and `-I`, from outside the download folder. Restricted loading and classification used about 9 GB of working memory on this machine; compressed file sizes are not a memory estimate. The `inspect` step uses `pickletools.genops` and a symbolic stack/memo to resolve every GLOBAL and STACK_GLOBAL without executing them. Extension and persistent-id opcodes are refused. It writes `pickle-globals.json`. The loader checks every recorded global against an explicit allow-list and uses a restricted `pickle.Unpickler.find_class` for the actual load. It never uses pandas' pickle reader. The removed pandas `Int64Index` reconstruction global is mapped to the current integer `Index` class. The two legacy `numpy.core` reconstruction modules resolve to their installed `numpy._core` counterparts only after the original global passes the allow-list check. An existing audit can be reused only with the pinned files passing their checksum checks; the restricted loader remains active.

`self-test` checks refusal of an `os.system` pickle global, both forms of global opcode, exact rotation sets, rejected intervals and flips, units, ring-size errors, and repeatable sampling. No unsafe payload is executed.

## What the columns mean

| Table | Columns | Meaning and mapping |
|---|---|---|
| tasks | tasks_index | Integer join key; used as the public task id. |
| tasks | efficiency | Recorded Lectra utilization in percent. |
| tasks | duration | Seconds; metadata calls this convergence time. |
| tasks | sheet_width, sheet_length | Coordinates in 0.0001 m. Multiply by 0.1 for mm. A positive sheet length becomes max_length; negative length is treated as absent in diagnostic jobs. |
| tasks | sheet_type | Opaque nesting category. No public codebook was found; exact interpretation remains unresolved. |
| tasks | is_train, is_val, is_test | Supplied machine-learning split flags; sampling does not train a model or use these flags. |
| parts | part_id, shape_hash, tasks_index | One physical part per row, with shape and task join keys. The actual column is `part_id`, although both descriptions say `parts_id`. Repeated hashes are repeated copies, not duplicate rows to discard. |
| shapes | shape_hash, raw, sizes | Shape key; point coordinates; sub-shape scalar-coordinate counts (divide by two for vertices). Flat alternating x/y coordinates are paired. Observed `sizes` values sum to the flat raw-array length: for example, 112 scalar entries give 56 x/y points. Odd or inconsistent counts are rejected. A repeated closing vertex is removed. |
| constraints | type, tasks_index, parts_1, parts_2 | Opaque constraint class, task key, and lists of referenced part ids. |
| constraints | p1_x, p1_y, p2_x, p2_y | Anchors in each involved part's coordinates. |
| constraints | r1_start, r1_end, r1_flip_x | Parallel lists of allowed rotation intervals and associated flip settings. The paper confirms rotation ranges are in degrees. |
| constraints | y_min, y_max | Allowed placement intervals across the sheet. Not supported by Job. |
| constraints | x_offset, y_offset, motif_order, x_alignment_type, y_alignment_type | Pattern/motif/alignment parameters. Exact class codes are not specified in the public description. |
| constraints | proximity_type, max_distance | Proximity parameters. Not supported by Job. |
| constraints | groups_relative_orientation, is_frozen | Group orientation and frozen-position information. Not supported by the current Job contract. |

The paper's Introduction defines efficiency as the fraction of consumed material occupied by placed parts. For a rectangular strip this agrees with BETA's area/(width*length). It does not provide enough detail to certify identical hole accounting or finite-sheet denominator conventions. Section 3 says about 100 fashion customers supplied tasks, the same algorithm was used, and nominal nesting time was five minutes for every task. Actual `duration` values differ slightly; this likely includes overhead, but that is an inference. The Lectra algorithm version, hardware and thread count remain unknown.

## Exact mappings and exclusions

Only `s1` records containing rotation fields and no other populated parameters are candidates for mapping. The field structure is consistent with the paper's statement that every part has one rotation constraint. An exact point-angle set `{0}` maps to fixed, `{0,180}` to flip180, and the four cardinal directions to quarter. One full 360-degree interval maps to free. Other intervals and angle sets are excluded. Missing, repeated or orphan part references are excluded. A missing rotation constraint is not assumed to mean free rotation.

Nonzero `r1_flip_x` is excluded. The public description does not establish whether this reflects the x coordinate or about the x axis, nor how optional handedness should affect copy counts. BETA requires explicit normal and mirrored quantities. A guessed axis could change which physical pieces are cut.

All other constraint classes are excluded regardless of apparently default parameter values. The names `c*`, `g*`, and `s*` do not establish semantics. Group, matching, motif, proximity, y-range and frozen constraints must not be dropped from a headline comparison. Their observed type counts are reported below rather than assigned invented class names.

For a shape with several sub-shapes, the diagnostic converter keeps the first ring and records how many sub-shapes it omitted. It excludes the task because the public descriptions do not prove that the first ring is the exterior and all others are holes. Dropping verified holes would make geometric packing harder, while increasing outline area; the latter must be corrected when comparing efficiencies. The runner accepts `reference_area_ratio` for this purpose, but the current converter does not certify such tasks.

Every task is currently excluded from the **certified** set until `sheet_type` semantics are established. This is an evidence gap, not a claim that no task could ever be compared. The converter separately reports tasks for which this is the only remaining blocker and produces a stratified diagnostic sample with `comparable: false`. The runner refuses that file. It never silently relaxes eligibility to reach 300 tasks.

## Outputs and sampling

The converter writes to `$BETA_DATA_DIR/marker-bench/lectra/`:

- `pickle-globals.json`: exact globals found in each downloaded file.
- `profile.json`: local schema and examples. This contains geometry; do not commit or publish it.
- `eligibility.jsonl`: one metadata-only record per task, including all exclusion reasons.
- `comparable-task-ids.txt`: the full certified task-id list.
- `sample-300.jsonl`: up to 300 certified Job records, each with Lectra efficiency, duration, constraint summary, eligibility and stratum.
- `diagnostic-sample-300.jsonl`: up to 300 mechanically convertible jobs still blocked by the unknown sheet-type code. Each remains explicitly non-comparable.
- `conversion-report.json`: population counts, overlapping reasons, mutually exclusive primary reasons and sample strata.
- `conversion-summary.md`: geometry-free Markdown audit with counts, exclusions, globals and attribution.

Sampling uses seed 7030786, sorts task ids first, shuffles within each stratum, and allocates by round-robin across nonempty strata without replacement. Piece-count strata are 1–25, 26–75, 76–150 and 151+. Reference-efficiency strata are below 75%, 75–85% and 85%+. Empty strata receive no allocation. This deliberately balances strata; its unweighted average is not a population estimate.

Raw data and Job geometry stay on the data drive. Only the small audit tables and result summaries belong in git. See [benchmarks.md](../../docs/benchmarks.md) for commands and limitations.

## Pickle globals observed

The full opcode scan completed for all four files. There are 12 distinct globals; every file uses 11. Tasks use RangeIndex, while parts, constraints and shapes use Int64Index. The exact per-file lists are committed in [pickle-globals.json](pickle-globals.json).

- `builtins.slice`
- `numpy.dtype`
- `numpy.ndarray`
- `numpy.core.multiarray._reconstruct`
- `numpy.core.numeric._frombuffer`
- `pandas._libs.internals._unpickle_block`
- `pandas.core.frame.DataFrame`
- `pandas.core.indexes.base.Index`
- `pandas.core.indexes.base._new_Index`
- `pandas.core.indexes.numeric.Int64Index`
- `pandas.core.indexes.range.RangeIndex`
- `pandas.core.internals.managers.BlockManager`

## Observed table and constraint counts

Restricted loading found 100,000 tasks, 7,360,718 physical part rows, 7,394,464 constraint records and 399,746 distinct shape records. Shape records have `shape_hash` (int64), `raw` (object) and `sizes` (object) columns. Sheet types are: code 0, 99,403 tasks; code 2, 570 tasks; code 3, 27 tasks. The codes are deliberately not assigned guessed fabric meanings.

The table shows non-null parameter families observed in each class, beyond task and part references. A populated field is evidence about the stored values; it is not a complete specification of the constraint. Every non-s1 class remains unsupported for the headline set.

| Type | Records | Observed populated parameters |
|---|---:|---|
| s1 | 7,360,718 | Rotation interval starts/ends and flip settings. |
| g2 | 15,029 | y_min, y_max. |
| g1 | 7,303 | groups_relative_orientation. |
| g4 | 3,584 | Part references only; numeric meaning unverified. |
| g3 | 2,562 | Anchors, rotation ranges, flip settings, is_frozen. |
| c1 | 2,205 | Two-part anchors, offsets, x/y alignment, group relative orientation. |
| c8 | 1,853 | Anchors and x/y offsets. |
| c3 | 348 | Two-part anchors, offsets, motif_order, x/y alignment. |
| c2 | 296 | Anchors, offsets, motif_order. |
| t1 | 204 | p1_x, p1_y. |
| t2 | 201 | y_min, y_max. |
| c6 | 62 | Anchors, y_offset, y_alignment_type. |
| c5 | 57 | Anchors, y_min, y_max. |
| c7 | 32 | Two-part anchors, proximity_type, max_distance. |
| c4 | 10 | y_min, y_max. |

There are exactly as many s1 records as part rows, consistent with the paper's one rotation constraint per part. The converter still checks the actual joins and rejects duplicate or missing constraints instead of relying on that aggregate count.

## Final eligibility result

The corrected full conversion found **0 strictly comparable tasks** because sheet-type semantics remain unverified. It found **9,606 tasks whose only remaining exclusion reason is that sheet-type meaning**. A separate diagnostic sample contains 300 of those tasks, with exactly 25 from each of the 12 piece-count/efficiency strata, using seed 7030786. Every diagnostic record keeps `comparable: false`; it is not a commercial benchmark result.

Overlapping additional exclusions: nonzero flip settings occur in 79,874 tasks; unsupported point-angle sets in 49,962 tasks; unsupported continuous intervals in 1,049 tasks. Unsupported classes and their per-task counts are in the [complete conversion summary](../results/2026-10-18-lectra-audit.md). A task can appear in several counts. The corrected decoder found no referenced task with inconsistent sub-shape sizes or multiple sub-shapes. Support for omitting unverified sub-shapes remains conservative for future inputs, but no omission was needed in this dataset run.

`comparable-task-ids.txt` and `sample-300.jsonl` are deliberately empty. `diagnostic-sample-300.jsonl` contains valid Job JSON with explicit non-comparability, reference efficiency, duration, constraint counts and strata. The full 100,000-task eligibility file stays on the data drive. No unverified task was substituted to fill a headline sample.
