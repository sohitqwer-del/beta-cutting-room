# BETA Pattern specification (version 0.1, draft)

BETA Pattern is the pattern-making and grading software of Project BETA. It competes with TUKAcad, Gerber AccuMark PDS, Lectra Modaris and Optitex PDS. It feeds BETA Marker directly. Code licence: Apache-2.0. The plan and the open-source review are in `../marker/docs/CUTTING-ROOM-PLAN.md`.

## 1. Hard rules

1. **Rust for everything that computes.** The browser UI (later) is TypeScript calling Rust through WebAssembly.
2. **Permissive licences only** (MIT, Apache-2.0, BSD, ISC, Zlib, Unicode). **No GPL code**: Seamly2D, Valentina and LibreCAD are GPL. Never copy their code, icons or texts. Reading and writing their file formats is allowed; build readers from our own test files and observation.
3. Never copy specifications, code or sample files from Tukatech, Gerber, Lectra or Optitex.
4. User patterns are private by default (`private-patterns/` is git-ignored). Never commit or publish them without written permission.
5. Units: millimetres; angles in degrees, counter-clockwise, y axis up (same as BETA Marker).
6. No network at runtime. No machine-specific paths in git (`CARGO_TARGET_DIR`, `BETA_DATA_DIR`).

## 2. Layout and ownership

| Path | Contents | Owner |
|---|---|---|
| `Cargo.toml`, `SPEC.md`, `README.md` | Workspace and contract | Lead |
| `crates/beta-pattern-core/` | Drafting model, formulas, pattern tools, pieces, seam allowance, grading, DXF-AAMA/ASTM with grade rules, Seamly2D import | Agent `pattern-engine` |
| `crates/beta-pattern-digitize/` | Photo/scan digitizing of paper patterns | Agent `digitize` |

Shared with BETA Marker (read-only for pattern agents): `../marker/crates/beta-marker-core` (the `Piece` contract) and `../marker/crates/beta-marker-dxf` (DXF reader/writer). Every BETA Pattern piece must convert to a valid `beta_marker_core::Piece`.

## 3. Commands

```sh
cd pattern
cargo test --workspace
cargo clippy --workspace --all-targets
```
