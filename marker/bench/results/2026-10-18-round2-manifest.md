# Round-2 evidence manifest

**Assumption:** `sheet_type=0` means an ordinary rectangular strip of `sheet_width`, with open single-layer fabric. No public codebook verifies this. `flip_x=1` is interpreted as x → −x before rotation. Tier A is exact conditional on these assumptions. Tier B includes Tier A and restricts each part to a subset of Lectra’s rotations and mirror states.

Base engine revision: `3022b12`. Benchmark tooling has local changes. Only summaries are committed, as requested. These SHA-256 hashes identify the final tooling files; the code changes remain in this worktree for the lead.

| File | SHA-256 |
|---|---|
| `bench/tools/lectra_convert.py` | `52702a3d34fb55c5841ab9676414e1ae2bebfe990b1000a1a982890e63f7ce85` |
| `bench/tools/summarize_round2.py` | `cb4ea7130864122aefa04e62474b86fb0bbbc2fe561000c875f5772af002b50d` |
| `bench/tools/lectra_geometry_check.py` | `c1d29a28bc931ec2763fecb2ff79e515426e1c7020cee3ae2ade0ea5727e1268` |
| `crates/beta-marker-cli/src/cmd/bench.rs` | `f42eff260c6c3e98977c8666721f4c3c2de91fcadcb378c7ebb283f39926f2d1` |
| `crates/beta-marker-cli/src/cmd/bench/runner.rs` | `c3e6143fb19cb91b9e7eb84eaa7ce231c3572407d3a8053e082112c03f6c6c54` |

| Input | SHA-256 |
|---|---|
| `sample-300.jsonl` | `d93776b94277eb3095d93d08978407e1fcdce62a360adb6b0e750a96bb5347b0` |
| `sample-match-60.jsonl` | `cb76643a3a755cbe043c7673b75ad55babb157931c3805f41902654bfc89eff2` |
| `diagnostic-code-2.jsonl` | `1eb3d749b090b0c257ad137f9bc46490688a059dd2d4faf58552e09526e96c32` |
| `diagnostic-code-3.jsonl` | `9dcbfa0c6eeead1369c2e54abd294f4e04e82e2c6057839667d3827af979a8ef` |

Input hashes identify external files; inputs contain geometry and are not committed.
