# Lectra conversion audit — 18 October 2026

Source: Lallier, Vézard, Pinaud and Blin (2022), *Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation*, [Zenodo 7030786](https://doi.org/10.5281/zenodo.7030786), version 1.1, CC BY 4.0. Downloads matched all four API MD5 checksums. No raw geometry is included here.

Tasks: **100,000**. Strictly comparable: **0**. Requested sample: 300; actual eligible sample: **0**.

There are 9,606 mechanically convertible candidates whose only remaining blocker is unknown sheet-type semantics. The separate diagnostic sample contains 300 tasks and remains `comparable: false`. The benchmark runner rejects it. No Lectra performance number is claimed.

No public codebook for sheet_type was found in the Zenodo description, author dataset card, or full HAL paper. A task is not treated as an exact match by assuming that code 0 means plain open-width fabric. Nonzero flip settings and unverified multi-ring topology are also excluded.

## Rows

| Table | Rows |
|---|---:|
| tasks | 100,000 |
| parts | 7,360,718 |
| constraints | 7,394,464 |
| shapes | 399,746 |

## Constraint types

The class labels below are observed values. Their vendor meanings are not invented. Only s1 records with rotation fields alone are candidates for exact mapping.

| Type | Records |
|---|---:|
| c1 | 2,205 |
| c2 | 296 |
| c3 | 348 |
| c4 | 10 |
| c5 | 57 |
| c6 | 62 |
| c7 | 32 |
| c8 | 1,853 |
| g1 | 7,303 |
| g2 | 15,029 |
| g3 | 2,562 |
| g4 | 3,584 |
| s1 | 7,360,718 |
| t1 | 204 |
| t2 | 201 |

## Sheet types

| Code | Tasks |
|---|---:|
| 0 | 99,403 |
| 2 | 570 |
| 3 | 27 |

## Exclusion reasons

Counts overlap: one task can have several reasons. Every task is blocked by the unverified sheet-type meaning.

| Reason | Tasks |
|---|---:|
| flip_semantics_unverified | 79,874 |
| rotation_interval_not_exactly_supported | 1,049 |
| rotation_set_not_exactly_supported | 49,962 |
| sheet_type_semantics_unverified | 100,000 |
| unsupported_constraint:c1 | 71 |
| unsupported_constraint:c2 | 10 |
| unsupported_constraint:c3 | 15 |
| unsupported_constraint:c4 | 9 |
| unsupported_constraint:c5 | 11 |
| unsupported_constraint:c6 | 14 |
| unsupported_constraint:c7 | 2 |
| unsupported_constraint:c8 | 124 |
| unsupported_constraint:g1 | 1,100 |
| unsupported_constraint:g2 | 438 |
| unsupported_constraint:g3 | 553 |
| unsupported_constraint:g4 | 307 |
| unsupported_constraint:t1 | 121 |
| unsupported_constraint:t2 | 201 |

Primary reasons are the lexicographically first reason per task; their counts are mutually exclusive.

| Primary reason | Tasks |
|---|---:|
| flip_semantics_unverified | 79,874 |
| rotation_interval_not_exactly_supported | 83 |
| rotation_set_not_exactly_supported | 10,059 |
| sheet_type_semantics_unverified | 9,984 |

## Pickle globals

All GLOBAL and STACK_GLOBAL opcodes were resolved with pickletools.genops before restricted loading. Only explicit pandas/numpy reconstruction globals and builtins.slice were accepted. Python ran with -I outside the download directory.

### tasks.gz

- `builtins.slice`
- `numpy.dtype`
- `numpy.ndarray`
- `numpy.core.multiarray._reconstruct`
- `numpy.core.numeric._frombuffer`
- `pandas._libs.internals._unpickle_block`
- `pandas.core.frame.DataFrame`
- `pandas.core.indexes.base.Index`
- `pandas.core.indexes.base._new_Index`
- `pandas.core.indexes.range.RangeIndex`
- `pandas.core.internals.managers.BlockManager`

### parts.gz

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
- `pandas.core.internals.managers.BlockManager`

### constraints.gz

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
- `pandas.core.internals.managers.BlockManager`

### shapes.gz

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
- `pandas.core.internals.managers.BlockManager`

## Sampling

Seed: 7030786. Equal round-robin allocation by piece count (1–25, 26–75, 76–150, 151+) and recorded efficiency (<75%, 75% to <85%, 85%+), without replacement. This is not a weighted population estimate.

| Diagnostic stratum | Sample tasks |
|---|---:|
| pieces:1-25;lectra:75-85 | 25 |
| pieces:1-25;lectra:85+ | 25 |
| pieces:1-25;lectra:<75 | 25 |
| pieces:151+;lectra:75-85 | 25 |
| pieces:151+;lectra:85+ | 25 |
| pieces:151+;lectra:<75 | 25 |
| pieces:26-75;lectra:75-85 | 25 |
| pieces:26-75;lectra:85+ | 25 |
| pieces:26-75;lectra:<75 | 25 |
| pieces:76-150;lectra:75-85 | 25 |
| pieces:76-150;lectra:85+ | 25 |
| pieces:76-150;lectra:<75 | 25 |

The complete comparable-task id list, eligibility metadata for all tasks and sampled Job JSON remain on the data drive. Multi-ring diagnostic jobs keep the first sub-shape and record the omission; none enters a headline comparison. Dropping holes changes both feasibility and the efficiency numerator, so any future inclusion must preserve net-area accounting.

The full paper states a nominal five-minute nesting budget using the same algorithm. Software version, hardware and worker count are unknown. The paper describes area utilization but does not settle hole accounting or all finite-sheet denominator conventions.

The final CLI checks rejected diagnostic task 43028 with `sheet_type_semantics_unverified` and rejected the empty strict sample with `input contains no tasks`. Both exited with code 1 before creating result CSVs. No Lectra nesting run or efficiency claim was made from the diagnostic sample.
