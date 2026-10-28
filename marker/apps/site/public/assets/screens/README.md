# Screenshots

## Home page paths (0.8)

Real BETA Cutting Room browser app (static build, real Rust engines in the browser), captured headlessly with sample
data only and saved as WebP in two widths each. Only these files are loaded by the website.

| File | Data and screen |
|---|---|
| `path-paper-390.webp`, `-600.webp` | Photo to marker on a 390 px phone (2x): the built-in practice sample photo with the found pieces outlined |
| `path-pattern-800.webp`, `-1200.webp` | Make a style · Pattern: the sample tour T-shirt block (sizes S–XL) on the drafting canvas |
| `path-marker-800.webp`, `-1200.webp` | Make a style · Marker: the sample tour T-shirt marker with piece names, all checks passed |

The launch QA tools (`site-shots.mjs`, `webp.mjs`, outside the repository) drive the sample tour and convert the
PNG captures with the browser's own WebP encoder. No factory or private data.

## Older release screenshots (0.7.0)


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
