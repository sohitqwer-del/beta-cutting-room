# Three 10-minute starts

Open the [browser app](https://sohitqwer-del.github.io/beta-cutting-room/app/).
No account is needed. Use public samples first. These are time boxes for learning,
not promises of production readiness. For experimental 0.8 steps, use a matching
development build; the current public 0.7 app lacks those controls.

## Paper patterns on a phone

You need flat paper pieces, a whole A4 reference, good light and a tape measure.
One photographed set is one physical size; take separate photos for other sizes.

1. **Minutes 0–2:** choose **Digitize paper patterns** on Home (the Photo to marker flow). Put pieces apart on a contrasting surface. Put A4 beside them in the same plane. Photograph from above with the whole reference visible.
2. **Minutes 2–5:** review every outline. Remove bad detections and retake the photo. Confirm part name, size, copy count, left/right pairs and grain. Compare the part inventory with the paper garment set.
3. **Minutes 5–7, experimental 0.8:** choose a fabric group. Open a piece, select two clear outline points and measure the same straight distance on paper. Enter the measurement and a suitable tolerance. Each included photo needs a passing tape check; any included failing check blocks it. This checks one distance, not every curve.
4. **Minutes 7–9:** confirm width, open/folded/tube mode and gaps. In 0.8 each group keeps its construction/GSM/settings. Make one group's marker, then inspect counts, grain, useful yield and warnings. Repeat for other fabrics.
5. **Minute 10:** save the local pattern set in 0.8; keep the source photos separately. Reopening restarts review/checks. Print a small actual-size test and measure it before trusting the full output.

Expected result: reviewed pieces and a marker for one fabric, or a clear reason
to retake a photo. If detection fails, stop and retake; do not force acceptance.
See [paper workflow and limits](PAPER-PATTERNS-MSME-2026-10-26.md).

## Make a style

1. **Minutes 0–2:** choose **Make a style**, name a trial and select the **T-shirt** block. Keep the example chart for this first pass. It is not a certified size standard.
2. **Minutes 2–4:** in **Pattern**, inspect the separate pieces. Try one small point/curve edit, then Undo. Check quantities, grain and allowances.
3. **Minutes 4–6:** in **Grade**, select S/M/L. Inspect the size outlines, block checks and spec sheet. A green length check does not prove fit; edits that change topology need fresh seam review.
4. **Minutes 6–8:** choose **Lay** → open fabric and enter the actual usable width with units. Set the ratio, physical layers, nap and gaps. Leave stripes off for the first trial.
5. **Minutes 8–10:** run a short search in **Marker**. Inspect labels, quantities and validation. In **Cut**, export a PDF and save the Style document. Print at actual size and measure a known distance.

Expected result: a saved graded draft and checked sample marker. Next, try
**Stripes** with a measured repeat and inspect the overlay/report. Before cutting
orders, sew a sample and have a pattern master review fit and seam allowances.
See [block checks](../../pattern/docs/BLOCK-CHECKS.md).

## DXF to cut plan

You need a supported, permission-cleared DXF and known units, counts, fabric
width and order quantities. Start with a demo if these are unknown.

1. **Minutes 0–2:** choose **Marker from files** → **Open DXF**. Confirm units, sizes, names and warnings. Review the default left/right pairing; same-hand copies need an explicit decision. Add a supported RUL file only if needed.
2. **Minutes 2–4:** set usable width, layout, nap, gaps and required size ratio. Start open for the first trial. Folded/tube width is the doubled fabric's working table width, not its full open width.
3. **Minutes 4–6:** nest briefly. Inspect every validation issue, quantity and yield. Compare surplus copies on doubled fabric. Save the job/result before exporting.
4. **Minutes 6–8:** open **Cut plan** through More tools or the cutting workflow. Enter actual order quantities, maximum physical plies, table limits, end allowances and allowed overcut. Run planning. Stop may retain a partial plan: check shortages and overcut before using it.
5. **Minutes 8–10:** inspect lays, passes and total fabric. Open **Rolls** for inspection/allocation, then **Cutting floor → Spreading** for release work. Without inspected stock, this is a planning trial. Save a browser backup file.

Expected result: a reviewed quantity plan, not an automatic authorization to cut.
A folded/tube pass has two physical plies; see [plies and examples](PLIES.md).
Only release after stock, shrinkage, shade, defects and machine scale are checked.
The experimental 0.8 width/cost comparison is optional and currently open-fabric
only; read its [limits](WORKFLOW-IMPROVEMENTS-2026-10-25.md).
