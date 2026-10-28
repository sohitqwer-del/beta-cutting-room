# Real photo-reader teaching sample

`sample-tshirt.png` is original project material, Apache-2.0. It contains no
factory or vendor data. Generate it with `deploy/video/make-paper-sample.mjs`.
That script calls the real Rust Pattern WASM T-shirt block and exports its cut
outlines. It presents a front, back, two sleeves and neck rib at **30% teaching
scale**, beside a full 210 x 297 mm A4 reference on contrasting matte backing.
Even lighting and slight perspective are rendered locally, without AI image
generation, downloaded textures, fonts or third-party photos.

The strip's drawn arrow runs along its long edge. This makes its grain visible
in the teaching image; the fixture is not a garment fit or rib-cutting recipe.
The sample width is 450 mm (the same 30% scale applied to a 1500 mm fabric width)
and its default order is four teaching garments. Pattern and fabric are scaled
together; the example does not increase fabric use by changing their ratio.
The sample is read and nested by the real Rust engines. Explicit `quick-mock=1`
remains test-only and is never used in launch recording.

The known long straight body side edge is **143.8546 mm** in the generated
geometry. The app gives the rounded 14.39 cm value for the sample's tape check.
This is an independent fixture dimension, not a measurement inferred from the
scan. Real paper requires a real tape check. A4-only confidence remains medium;
a successful independent distance check enables the green checked status.

`sample-scan.mjs`, called by `scan-status.test.ts`, uses the actual Rust WASM
scanner. It asserts five pieces, five detected grain lines, no quality warnings,
and estimated error below 2.2 mm. Routine outline/grain review stays visible as
guidance. Missing grain, low confidence, blur and other quality warnings remain
warnings. The launch recorder also gates the sample marker at 65% fabric use.

Regenerate with the WASM modules built from this checkout. Compare the resulting
PNG and run the scanner regression before replacing the asset. The generated
SVG, original block job and truth manifest stay in the external fixture folder.
