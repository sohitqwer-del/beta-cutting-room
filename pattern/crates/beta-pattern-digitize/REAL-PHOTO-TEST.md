# Ten real photos for a first factory check

Use scrap paper. Do not cut production cloth from these scans yet. Keep photos
and patterns local. Share only synthetic or explicitly approved public patterns.
Use the phone's normal camera, no portrait blur or ultra-wide lens. Use full
resolution JPEG/PNG up to 12 MP; flatten curled paper with small weights away
from edges. Do not let the weights hide any edge.

## Prepare once

- Lay a dark, matte cloth on a flat floor. Smooth the cloth and the paper.
- Use one untrimmed, plain white A4 sheet. Check its sides with a steel ruler:
  210 x 297 mm. Reject damaged, curled or inaccurately cut sheets.
- Cut three scrap pieces: a 100 x 200 mm rectangle, an asymmetric sleeve-like
  shape, and its mirror. Label the underside. Draw one long grain line inside
  each. Add one V notch and one 3 mm dot to the asymmetric shape.
- Record each side, two diagonals and three widths of the shapes to the nearest
  0.5 mm. Trace their outlines on squared paper or use a checked 1:1 print from
  the public FreeSewing fixture. Keep that reference independent of the scan.
- Record phone model, resolution, reference dimensions, lighting and the distance
  from each piece to the nearest A4 edge. Leave at least 10 mm between objects.

## Take these ten photos

| Photo | Setup |
|---|---|
| 1 | One white piece, 50 mm from the A4 sheet. Bright indirect daylight. Phone overhead, about 0.8 m high. |
| 2 | Same setup, piece 200 mm from the sheet. Same height and lighting. |
| 3 | Same piece 500 mm from the sheet. Raise the phone only enough to include all edges. |
| 4 | Same as 3, with a second A4 sheet near the distant piece. Keep both sheets separate and flat. |
| 5 | All three pieces on dark cloth, 10-30 mm apart. Overhead, indirect daylight. |
| 6 | Same as 5, phone tilted about 25 degrees from overhead. Record the approximate tilt. |
| 7 | Same as 5, indoors with light from one side and a soft shadow across one piece. |
| 8 | One brown-paper or card copy with the same measured outline. Keep it flat. Include the white A4 sheet. |
| 9 | Intentionally degraded: slight hand motion and two pieces touching at an edge. Check blur/touch warnings; separate them before acceptance. |
| 10 | Accurate mode: use a printed BETA A3 target checked at 100% (ten cells = 50 mm, within 0.2 mm). Fit the same small pieces inside the clear border. |

Do not crop after taking the photos. Keep all originals. Note wrinkles, glare,
missing edges and any warnings. A warning's absence does not prove a good photo.

## Record and compare

Save each overlay, scale report and accepted outline JSON, without manual geometry
changes first. Check that the yellow rectangle is the A4 sheet. Record a failed
scan as a failure; do not silently omit it from the accuracy results.

Compare the recovered lengths, diagonals and widths with the ruler record.
For full outline error, digitize the independent squared-paper/ruler reference,
then sample both outlines in mm and measure nearest-edge distances. Record mean,
maximum, area difference, missed/false pieces, marks and grain angle. Only a known
reference-frame translation/rotation is allowed; never rescale to improve the
score. For A4 scans use the reported sheet origin and axis; for BETA target use
the board coordinates. Report A4 and target results separately.

Repeat photographs 1 and 5 on another phone before any production trial. A useful
first acceptance goal is mean contour error below 1 mm and maximum below 2 mm,
with no missed or joined pieces, on these physical tests. This is a proposed
factory gate, **not an accuracy claim that this engine has already met**.
