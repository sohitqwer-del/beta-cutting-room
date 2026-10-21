# Original BETA basic blocks

These are editable starting drafts made for this project. No Seamly, vendor
code, proprietary drafting instructions, or factory patterns are used. The
formulas below are BETA's own simple constructions. They are not fit-certified
production patterns. Make a sample and replace the example chart with the
buyer's measurements before production.

## Measurements and chart

All values are millimetres. Circumferences are full body circumferences, not
flat garment widths. `shoulder` is one shoulder seam length. `arm_depth` is the
vertical shoulder-to-underarm depth. `body_length`, `tunic_length`,
`trouser_length`, `inseam`, `sleeve_length`, `short_sleeve` and `skirt_length`
are vertical construction lengths. The examples use S, M, L, XL and XXL.

Let `i = 0..4` for these sizes. The `indian_adult` example uses chest
`920 + 60i`; `iso_example` uses `880 + 80i`. Both use waist `chest - 140`, hip
`chest + 40`, neck `360 + 15i`, shoulder `125 + 5i`, arm depth `205 + 10i`,
body length `650 + 15i`, sleeve `580 + 10i`, short sleeve `215 + 5i`, wrist
`170 + 5i`, bicep `300 + 15i`, trouser length `990 + 15i`, inseam `735 + 10i`,
ankle `340 + 10i`, skirt length `600 + 10i`, tunic length `930 + 20i`.

These are original illustrative adult charts for factory data entry. The
names describe measurement conventions and the intended example use. They
are not Indian population survey data, a certified ISO chart, or a mapping
from ISO body measurements to universal S–XXL labels. No kids preset is included.

Default ease is 80 chest, 40 waist, 60 hip and zero sleeve-cap ease.
Seam allowance defaults to 10; hem allowance to 20. Editable variables and
measurement formulas remain in the document after generation. The document's
base size is M. Grading reevaluates these formulas in each chart column.

## Upper-body construction

The half-body width is `(chest + chest_ease)/4`. Neck width is `neck/6`.
The shoulder endpoint is offset horizontally from the neck by
`sqrt(shoulder² - shoulder_drop²)`, so the actual sloped shoulder has the
specified length. Shoulder drop is 30 (45 for the shirt). The underarm is at
`body_length - arm_depth`. Front and back use the same side and armhole
construction so the side and shoulder seams match exactly.

The armhole is a cubic from underarm C to shoulder D. Its first handle is
`(body_width - 20, body_length - arm_depth + 30)`. The second is
`(D.x - 15, D.y - 40)`. Front neck depth is `neck_width + 10`; back depth is
20. The neck cubic runs from E at neck/shoulder to centre F. Its handles are
`(neck_width, body_length - .9*neck_depth)` and
`(.45*neck_width, body_length - neck_depth)`. This gives a horizontal tangent
at the centre and avoids a cusp after reflection. Front/back full outlines
are linked reflections, with an explicit full-piece fold axis.

The sleeve starts with a cubic profile from `(-bicep/2, 0)` to
`(0, .4*arm_depth)`, with handles `(-.45*bicep, .2*arm_depth)` and
`(-.2*bicep, .4*arm_depth)`. Scale this profile uniformly by

`k = (front_armhole_length + back_armhole_length + cap_ease) / (2 * profile_length)`.

Reflect the scaled half to make the full cap. Cap height is
`.4*arm_depth*k`; its crown is at sleeve length. Thus cap length follows actual
armhole curves after measurement edits. The sleeve bicep width is `bicep*k`,
not a claim of an independently solved exact finished bicep measurement.
Sleeve cuff width is `wrist+60` for long sleeves or `bicep+40` for short sleeves.
Straight underarm seams are symmetric. Curves are sampled at 0.1 mm for normal
editing and 0.05 mm for sewing reports; a small sampling difference can remain.

| Block | Pieces and construction |
|---|---|
| T-shirt | Full front/back on fold, mirror sleeve pair, neck rib of `0.85 * neckline` by 40. Rib uses an explicit 15% stretch assumption; it is not a zero-ease seam match. |
| Shirt | Mirrored half fronts, full back, narrow straight yoke, long sleeve pair, two cuffs, two collars, two stands and two plackets. The yoke/back split is a straight line through the shoulder/armhole junction. Cuffs are `wrist+60` by 100; collar and stand joining edges equal the complete neckline. Plackets are 30 wide and follow the front centre length. |
| Polo | Full front/back, short sleeve pair, two collar pieces of neckline length by 70, two plackets of 30 by 180. |
| Kurta / tunic | Full front/back, long sleeve pair, neck binding and two 30 by 250 plackets. Body length uses tunic length. Half hem width is `max(chest quarter, hip quarter)+20`. |

The rectangular collar/stand/placket drafts are simple components, not an
automatic tailored collar shaping or button extension solver. Polo/kurta
placket opening, fastening choice, interfacing, and finishing details require
operator construction. Neck facing is a straight binding, not a curved facing.
The shirt has no fitted waist darts or automatic sleeve placket slash. No claim
is made that these details are already production-finished.

## Lower-body construction

Hip quarter `H = (hip + hip_ease)/4`, waist quarter
`W = (waist + waist_ease)/4`. Trouser rise is `trouser_length - inseam`.
Leg hem width is `(ankle+80)/2`. The front and back outer seams and inseams
share endpoints; separate crotch cubics create front/back scoop differences.
The waist edge is W long. Two normal/mirrored leg pairs make one garment.
Waistband length is `4W`, height 80, without an extra closure extension.

The crotch runs from `(H-W, trouser_length)` to `(0, inseam)`.
Its front controls are `(H-W, inseam+.3*rise)` and `(-25, inseam+.05*rise)`;
back controls are `(H-W-15, inseam+.3*rise)` and `(-40, inseam+.05*rise)`.
This is a simple loose base with identical matched leg seams, not a tailored
seat or balance model. Pocket bags are rectangular starters, `hip/6` by
`hip/4`, in lining, with two normal and two mirrored copies. Pocket entry,
fly, darts, closure extension and pocket shaping remain operator decisions.

Track pants use the same leg construction but set the waist quarter to H,
giving a hip-sized elastic casing. The 80 mm casing width is unfolded; choose
elastic width and turnings before cutting. It is not elastic-length grading.

The A-line skirt has full symmetric front/back pieces. Half waist is W;
half hip is H at 200 below the waist; half hem is `H+60`. Straight connected
side segments match between front/back. A `4W` by 80 waistband is included.
The block has no darts or closure slit by default.

## Draft sheet arrangement

Generated pieces arrive in separate columns, with vertical grain and at least
50 mm of cut-outline clearance across every size in the chart. The layout uses
reserved `layout_<piece>_x/y` variables inside the point formulas, so direct
handle edits still retain measurement grading. Updating the chart recomputes
the column offsets. The shirt yoke has its own editable construction objects
under `yoke.source.*`, instead of sharing the back's drawing coordinates.

This is a drafting-sheet arrangement. The aligned grade nest remains a
separate view, and marker nesting still happens only after the lay handoff.

## Assembly checks and limits

Generated `meta.block_seams` checks compare real graded stitch edges:
side seams, shoulders, both sleeve-cap halves (half the requested cap ease),
sleeve underarm seams, shirt back/yoke, cuff/sleeve, collar/stand, and trouser
inseams. Every default check uses 1 mm tolerance. After edits, `block_seams`
recomputes the report; mismatches remain visible. A report is not a fit test.

Material handoff requires one fabric per marker. Choose `material:'shell'`
or `'lining'` for trouser/track-pant jobs. Folded width is the flat width;
the body outline remains full, and the existing layer allocator chooses
halves or paired full cuts. Pattern grain and symmetry survive the handoff.

All seven blocks, both charts and five sizes are exercised in the native
geometry/sewing test. Edited four-size T-shirt export, matching and folded
nesting have a separate 30-second end-to-end test. See `STYLE-REPORT.md` for
the executed results and remaining integration limits.
