# Paper patterns for a small factory

The local branch `marker/paper-patterns-msme` extends Photo to marker. It starts from existing paper patterns. It does not require the operator to draft with CAD formulas.

## Operator workflow

1. Choose **Digitize paper patterns** on Home. Name the pattern set for one garment style.
2. Lay the paper flat on a contrasting background. Keep the whole A4 reference beside the pieces, on the same plane. Leave gaps between pieces and photograph from above. Photograph each physical size; this flow does not grade sizes.
3. Check each outline. Name the part and confirm size, copies, mirrored pairs and grain direction. Assign main fabric, lining, rib, fusing or another fabric group. The inventory shows parts and cut counts by fabric and size. Compare it with the paper set; missing garment parts cannot be inferred reliably.
4. Open a piece and choose two clear outline points. Compare their straight digital distance with a tape measurement of the same points on the paper. The operator sets a tolerance between 0.1 and 10 mm. At least one passing check is required for each source photo used by the selected marker. Any recorded failing check on an included piece also blocks that photo. One distance does not validate every curve or the garment's fit.
5. Choose a fabric group. Record its construction, GSM, width, open/folded/tube layout, one-way direction, piece gap and edge margin. Each group keeps separate settings. GSM is recorded as weight information; it does not resize the pattern or infer stretch or shrinkage.
6. Make a marker for that fabric group. Continue with the existing print, plotter, projector or marker-editor tools. Make separate markers for the other fabrics.
7. Choose **Save pattern set** to keep reusable outlines and fabric settings in a local `.paper-set.json` file. Reopening checks the geometry in Rust and restarts piece review and tape checks. Source photos and old marker layouts are excluded. Keep the paper patterns or original photos separately for reference.

The grain section also accepts two selected points parallel to the arrow on the paper. It stores the chosen direction without changing the outline. A4 scanning, outline extraction, snapping, tape distances, marker construction and validation use Rust. TypeScript handles forms, drawing and file transport.

## Validation

- 268 web/package tests passed, including archive round trips, fresh-review rules, failed checks, material isolation, per-fabric settings, malformed inputs and deleting the last piece in a group.
- All 9 native tests in `beta-marker-wasm` passed. The new tests cover tape snapping, distance failures, invalid measurements, material separation, spacing and GSM metadata without geometry changes.
- Web typecheck, production static build and strict Clippy for all targets of `beta-marker-wasm` passed.
- Browser functional checks used the built app and its real local WASM. Opening a two-piece synthetic archive produced separate main/lining inventory rows. Nesting stayed blocked until a paper check passed. A 100 mm edge failed against 95 mm at a 3 mm tolerance and passed against 100 mm.
- Switching groups restored main fabric at 180 GSM / 1000 mm and lining at 80 GSM / 1100 mm. The lining group still required its own check. The resulting main-fabric marker contained one main piece, retained 2 mm piece spacing and 5 mm edge spacing, and passed a separate Rust validation with zero errors and warnings. Its 205 mm length is a synthetic functional fixture, not an efficiency benchmark.
- Saving through the UI produced `beta-marker.paper-set/v1`, two pieces and two fabric profiles, without scan images.
- The final static build reopened the synthetic set with two saved-outline previews. Keyboard point A/B selections remained visible after redraw. Browser downloads recorded a 4,532-byte archive. Reopening it restarted at zero accepted pieces.
- A generated grayscale A4 scene went through the real browser photo worker. It found one piece, produced an overlay and retained its low-confidence scale and unconfirmed-grain warnings. This is a synthetic detection check, not a factory-camera accuracy measurement.

T3's snapshot and click tools timed out. Browser evaluation and file upload remained available, so functional checks used DOM controls and the normal UI handlers. The other browser connection reported no available browsers. Screenshot and mobile visual acceptance checks remain unverified.

## Limits of this version

- Real factory photos, lighting, lens distortion, paper curl and tape repeatability still need a measured acceptance trial. No factory accuracy guarantee is claimed.
- Incorrect outlines are removed and photographed again. Curve repair, reliable notch recovery and automatic seam-allowance interpretation are not added here.
- The inventory helps the operator organize parts. It does not certify a complete garment set or a correct garment fit. The same shape needed in two fabrics needs a separate scanned copy in each group.
- This is a single reusable style file, not a searchable multi-style pattern library. The archive is bounded to 8 MB, 300 pieces, 100 photos and 40 fabric groups.
- New inventory, fabric and tape panels use English. Existing Hindi/Tamil screens remain draft translations; the new blocked-step messages have draft translations.
- No release, publication, installer acceptance test or production deployment was performed. Private factory files were not used.

Synthetic fixtures and build logs are retained in the task's external evidence
folder. They are not release assets or a reproducible public input bundle.
