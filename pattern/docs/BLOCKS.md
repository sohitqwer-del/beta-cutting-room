# Original BETA basic blocks, round 0.7

These are original, editable **example blocks**, not fit-certified production
patterns. All dimensions are mm. No proprietary tables, drafting text, diagrams,
vendor patterns or private factory data were used. Sew a sample and have a
pattern master approve balance, fit, closures and the intended fabric.

## Charts and measurement terms

Both bundled S/M/L/XL/XXL charts are illustrative straight-body adult examples.
They are not population averages, an Indian national sizing system, a certified
ISO chart, or universal letter sizes. They do not represent bust shaping,
children, maternity, or all adult body proportions. Replace them with buyer data.

Public sources consulted by name: **ISO 8559-1, Size designation of clothes:
Anthropometric definitions for body measurement**, and **NIFT / Ministry of
Textiles, INDIAsize**. They inform terminology and the need for population-specific
measurement data. No measurements or grade tables were copied from them.

For size index i=0..4, the original examples retain these editable formulas:

| Dimension | Example value | Step | Meaning |
|---|---:|---:|---|
| Chest, indian_adult | 920+60i | 60 | Full body circumference |
| Chest, iso_example | 880+80i | 80 | Full body circumference; wider size interval |
| Waist | chest-140 | 60 or 80 | Full circumference |
| Hip | chest+40 | 60 or 80 | Full circumference |
| Neck | 360+15i | 15 | Full circumference |
| Shoulder | 125+5i | 5 | One shoulder seam construction length |
| Arm depth | 205+10i | 10 | Vertical HPS-to-underarm construction depth |
| Bicep | 300+15i | 15 | Upper arm circumference |
| Wrist | 170+5i | 5 | Wrist circumference |
| Ankle | 340+10i | 10 | Loose lower-leg opening reference circumference |
| Body length | 650+15i | 15 | Design length from high shoulder point (HPS) |
| Sleeve length | 580+10i | 10 | Design crown-to-finished-cuff length |
| Short sleeve | 215+5i | 5 | Design crown-to-hem length |
| Trouser length | 990+15i | 15 | Waist joining line to hem, excluding band |
| Inseam | 735+10i | 10 | Stitch-line leg length, excluding hem allowance |
| Skirt length | 600+10i | 10 | Waist joining line to hem, excluding band |
| Tunic length | 930+20i | 20 | Design length from HPS |

Length fields above are **design inputs**, not a claim that stature increases
with circumference. `ankle` is a loose opening reference, not a measured skin
ankle girth. Chart names are legacy API IDs; the display names say example.
The grade review checks non-decreasing values, steps up to 80 mm for major
girths and 25 mm for other dimensions, a 200-360 mm construction rise, and
broad neck/chest, bicep/chest and hip/waist plausibility bounds. These are BETA
advisory screens for these examples, not anthropometric standards. Both charts
pass all sizes. The two presets intentionally have different girth intervals;
other grade steps stay the same. No unsupported population-realism claim is made.

## Ease and construction assumptions

Ease below is added to **full circumference**, except cap ease (total seam
length), arm depth (vertical), and lengths (zero). User `BlockEase` overrides
chest, waist, hip and cap values. Bicep ease is the editable `ease_bicep` variable.

| Block | Chest | Waist | Hip | Bicep | Cap ease | Arm-depth allowance |
|---|---:|---:|---:|---:|---:|---:|
| T-shirt | 80 | not fitted | not fitted | 60 | 0 | 0 |
| Shirt | 100 | not fitted | not fitted | 80 | 12 | 20 |
| Trouser | n/a | 40 | 60 | n/a | n/a | n/a |
| A-line skirt | n/a | 40 | 60 | n/a | n/a | n/a |
| Kurta/tunic | 100 | not fitted | hem flare, see below | 80 | 12 | 20 |
| Polo | 80 | not fitted | not fitted | 60 | 0 | 0 |
| Track pant | n/a | hip-sized casing | 60 | n/a | n/a | n/a |

Knit cap ease must be -1% to +2% of armhole length; woven cap ease must be
+1% to +5%. These are declared BETA starting ranges, not a universal fabric
rule. The current create API permits nonnegative ease only; -1% is the lower
check bound for measured/edited knit curves. No stretch recovery is assumed
for shell pieces. Neck rib is **0.85 times the neckline**, 15% shorter; sewing
it to the neckline extends it about **17.65%**, not 15%. Test the actual rib.

Seam allowances default to 10 and hems to 20, on each specified edge. Folded
bands/cuffs contain their full unfolded depth and joining allowances. A shirt
cuff is 100 deep unfolded, 50 finished. The sleeve draft subtracts that 50 so
the finished crown-to-cuff length equals `sleeve_length`. Waistbands are 80
unfolded, 40 finished; lower-body design lengths explicitly exclude that 40.
There is no shrinkage compensation, fastening overlap or elastic-length solver.

## Upper body and sleeves

Body quarter width is `(chest+chest_ease)/4`. Neck width is `neck/6`. Shoulder
length is measured along its slope, using a horizontal run derived from that
length and the 30 mm drop (45 for shirt). Front neck depth is neck width+10;
back depth is 30. Front/back sides and armholes match. This symmetric loose
starter does not model separate front/back balance or a forward shoulder.

Armhole handles meet the side seam at right angles. The horizontal underarm
handle is 60% of the horizontal run from underarm to shoulder point. This keeps
the underarm radius at about 20-28 mm in the bundled charts, about twice the
10 mm allowance. A shorter handle tried during this round gave about 8 mm,
and the cut allowance folded over itself. Armhole and neckline handles meet each shoulder at right angles; the
two joined boundaries then have a common tangent. The neck meets its mirrored centre horizontally.
The side is straight on straight bodies. A tunic uses a smooth side with
vertical tangents at both ends, and half-piece hem width
`max(chest quarter, hip quarter)+20`. Its hip fit is not independently fitted.

The sleeve fixes bicep circumference at body bicep plus stated ease. A bounded
Rust bisection solves cap height while retaining that width and the measured
armhole length plus cap ease. It no longer scales bicep width to make a seam
match. Each half cap is a cubic from `(width/2,0)` through `(.27width,0)` and
`(.22width,height)` to `(0,height)`, translated to the crown. End tangents are
horizontal. The underarm seam is straight from cuff/hem to bicep and is squared
to the hem and the cap over its last 40 mm, so the joined underarm and hem have
no kink. Infeasible dimensions return a clear error instead of shrinking the
bicep or silently changing ease.

The shirt yoke seam is horizontal. It splits the back armhole curve exactly at
curve parameter 0.7, so the back armhole and yoke armhole together are the
original curve and the sleeve cap still matches. The yoke is 70-80 mm deep at
centre back for the bundled sizes (checked 50-90 mm); the seam meets the armhole about
60 mm below the shoulder point. The 0.6 yoke seam went through the shoulder points and was only 25 mm deep
at centre back. The yoke has independent linked drawing objects after
sheet arrangement. It is a straight casual-shirt yoke, not a shaped or
forward-shoulder production yoke. Collar, stand, placket, binding and cuff components are
simple rectangles. Collar/stand and binding joining lengths follow the actual
complete neckline; the T-shirt rib uses the stated ratio. Button extensions,
collar roll, polo/kurta openings, interfacing and sleeve plackets need drafting.

## Lower body

Hip quarter is `(hip+hip_ease)/4`; waist quarter is `(waist+waist_ease)/4`.
Garment hip H and garment waist W are the full circumferences with ease.

**Trouser and track pant (rewritten in round 0.7).** The 0.6 legs had almost no crotch
extension (0.01-0.02 of hip) and nearly identical front and back panels. Each leg is now drafted as two
panels in this original order:

1. Hip share: front panel H/4-10, back panel H/4+10, at the hip line 200 mm
   below the waist seam.
2. Crotch extension beyond the hip line: front H/20, back H/10. Together this is
   0.15 H; the back is twice the front so it covers the seat.
3. Waist reduction (H-W): half at the side seam, shared equally by front and
   back; the rest 30% at centre front and 70% at centre back. The track pant has
   W=H (hip-sized elastic casing), so there is no reduction.
4. The centre back is raised 25 mm and slants to the hip line. The back waist
   width is shortened so its sloped length still gives W in total.
5. Crotch depth: the inseam's vertical span is solved so its curved stitch
   length equals the chart inseam; rise = trouser length minus that span.
6. Each panel is centred on its crease at crotch level and at the hem. The crease
   is the grain line. Leg opening total is ankle+80. The back hem is wider than
   the front by exactly the amount that gives front and back legs the same
   lateral run, so front and back inseams and outseams are congruent and match.
7. Curves: the centre seam is straight from waist to hip line, then a cubic
   scoop that continues that direction and arrives horizontally at the crotch
   point. The waist curve leaves the side seam and arrives at the centre seam
   at right angles. Leg seams are S-cubics with vertical ends.

Result for the bundled M (hip 1020): front extension 54 mm, back 108 mm, back
crotch seam about 1.24 times the front, thigh 25 mm below the crotch about
0.65 H (about 700 mm). Mirrored front and back waists, the crotch across the
inseam, and both hem corners join without a kink. This is a loose, straight-
waisted casual block. It has **no darts, pleats, fly extension, pocket shaping,
seat-angle fitting or body-balance model**. Pocket bags remain rectangular
lining starters. Relaxed waist and elastic cut length are not specified. The
waistband length follows the actual shaped waist seam.

The A-line skirt retains exact waist and hip sections. Cubic sides share a
common tangent at the hip and meet waist and hem at right angles. Half-piece
hem width is hip quarter+60. Front/back side lengths match. A closure is still
needed to get the garment over the hips; no closure slit or darts are supplied.

## Checks, marks and edits

`block_checks` measures actual graded stitch geometry in every size. Default
seam and engineering POM residuals must be within 1 mm; joined-corner kinks
must be within 2 degrees. Native path tangents are exact; factory-deformed
paths use the end segments of their 0.05 mm sampling. Reports cover front/back
side and shoulder joins, inseam/outseam, waistband/waist, neckline components,
cuffs, cap ease and front/back allocation, finished dimensions, grain, notches,
edge allowances, and simple stitch/cut boundaries. Failed geometry remains a
failed size with its error text.

Round 0.7 adds checks that catch the 0.6 drafting faults (category
`proportion` unless noted). Ranges are BETA starting values, not standards:

| Check | Range | What it catches |
|---|---|---|
| `crotch_extension_total` | 0.12-0.18 of garment hip | A leg with no room between the legs |
| `crotch_extension_back_front` | 1.6-2.4 | Back extension not longer than front |
| `back_front_rise` | 1.1-1.5 | Back crotch seam not longer than front |
| `thigh_hip` | 0.58-0.72 | Thigh too tight or too loose for a basic leg |
| `<leg>.crease_offset` (grain) | at most 1 mm | Grain off the crease at crotch or hem: twisting leg |
| `waist_at_side` (tangent) | at most 2 degrees | Kink where front and back waists meet |
| `yoke_depth_cb` | 50-90 mm | A sliver yoke |
| `armhole_at_yoke` (tangent) | at most 2 degrees | Armhole kink across the yoke seam |
| `<piece>.concave_radius_<edge>` (geometry) | at least 1.25 times the allowance | Allowance that folds over itself in a tight curve |

`spec_sheet` uses the same actual geometry. Its tolerances are proposed factory
QC tolerances, separate from the 1 mm drafting checks: 10 mm for long lengths
(body, long sleeve, trouser, inseam, outseam), 6 mm for half chest, waist, hip,
hem sweep and curved armhole, 5 mm for armhole depth, across shoulder, short
sleeve and rises, 4 mm for half bicep, half thigh and leg opening, 3 mm for neck
width, neck drops, shoulder and sleeve opening, 2 mm for band depth. They need
buyer approval. Shirts saved by 0.6 (5-edge yoke) are still measured.

Assembly notches include side midpoints, single front and double back armhole
thirds, sleeve crown, component centres, and waistband quarters. Sleeve thirds
allocate the stated cap ease proportionately. Back double marks are 6 mm apart.
Marks are projected from stitch paths to allowance edges; depth is 3 mm and
must stay inside the allowance. Required reference formulas are kept in
`meta.block_notches` so moving/removing a mark does not erase its check.
Cutting the same shape mirrored does not change the front/back cap reference.

Main pieces have vertical grain. Woven collars, stands, cuffs and waistbands
have grain along their joining length; the woven neck binding has 45-degree
bias grain. Knit rib/collar grain permits cross-grain stretch along the join.
Every grain endpoint is checked inside the stitch boundary. Pieces have at
least 50 mm sheet clearance across all sizes. Layout variables do not affect lengths or reports. Grade nests are
a separate view. Pattern geometry and grain survive the marker handoff.

A topology-changing Style piece action defaults to **replacement**: the source
keeps its drawing references but its cut and mirror counts become zero. Set
`request.production_mode:'additional'` explicitly to retain both. Original
block seam/POM mappings are not valid for darts/splits/pleats: `block_seams`
returns no stale green rows; `block_checks` returns review errors per size;
`spec_sheet` refuses an obsolete spec. `meta.block_review_required` explains the
required review. Add custom seam checks for the actual production pieces.

See `BLOCK-CHECKS.md` for every before/after value. Passing arithmetic and
geometry checks is evidence of internal consistency, not production fit approval.
