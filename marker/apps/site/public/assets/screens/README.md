# Release screenshots

Real BETA Cutting Room 0.7.0 browser app, captured headlessly against the local
Rust server and the local Rust WASM. All seven files are 1920 x 1200 PNGs in
the light theme, with device scale 1. No visible windows open.

| File | Data and screen |
|---|---|
| `marker-editor.png` | Built-in original synthetic shirt, nested and checked |
| `style-pattern.png` | Editable original T-shirt block in the Style pattern canvas |
| `pattern.png` | Built-in BETA teaching bodice with four graded sizes |
| `cut-plan.png` | Same synthetic shirt, 40 L and 40 M garments |
| `rolls.png` | Original sample stock DEMO-001 and DEMO-002; inspected locally |
| `spreading.png` | Checked sample shirt; 20 plies released from sample stock |
| `bundles.png` | Eight physical-part bundles from the same sample marker |

These samples and captures are original BETA material under Apache-2.0.
No factory, buyer, private pattern, vendor geometry or production record is used.

Reproduce after building the CLI and all three WASM modules from the repository
root. Set a temporary `BETA_TOKEN`, then run:

```sh
node marker/deploy/app-screenshots.mjs http://127.0.0.1:47881 <output-dir> <CLI-binary>
```

The script owns and stops its loopback server and headless browser. Review the
seven images, then copy only the PNG files here. Keep its diagnostic JSON outside
the repository. Rebuild with `npm run build -w @beta/marker-site`.
