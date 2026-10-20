//! Human lay sheets and generic, versioned interchange files; no machine driver.
use crate::{
    floor::{FloorLay, FloorState},
    intake::csv_table,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Generic forward-only spreading instruction. All coordinates use metres.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineRow {
    /// One-based physical execution order.
    pub sequence: usize,
    /// Stable lay identity.
    pub lay: String,
    /// Factory cutting-table identifier.
    pub table: String,
    /// One-based spread pass within the lay.
    pub ply: u32,
    /// Source received roll identity.
    pub roll: String,
    /// Receiving lot identity.
    pub lot: String,
    /// Dye batch identity within the receiving lot.
    pub dye_lot: String,
    /// Inventory shade identity, never a geometric shade band.
    pub shade: String,
    /// Measured starting offset on the received roll, in metres.
    pub roll_start_m: f64,
    /// Start from the table head before marker allowance, in metres.
    pub table_start_m: f64,
    /// Consumed segment length including overlap, in metres.
    pub length_m: f64,
    /// Marker head allowance per spread pass, in millimetres.
    pub head_allowance_mm: f64,
    /// Marker tail allowance per spread pass, in millimetres.
    pub tail_allowance_mm: f64,
    /// Extra physical cloth at a join, in metres.
    pub splice_overlap_m: f64,
    /// Forward spreading only in version 1.
    pub direction: String,
    /// Face up in version 1.
    pub face: String,
}
/// Versioned envelope deliberately identifies this as an exchange, not controller code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineFile {
    /// Versioned format identifier or requested file encoding.
    pub format: String,
    /// Operator note or explicit format limitation.
    pub note: String,
    /// Source data rows or emitted instruction sequence.
    pub rows: Vec<MachineRow>,
}
/// Produce instructions only from a complete, independently audited floor lay.
pub fn machine(l: &FloorLay) -> Result<MachineFile> {
    beta_marker_roll::validate_plan(&l.stock, &l.spread)?;
    anyhow::ensure!(l.spread.complete, "Incomplete spread cannot be released.");
    anyhow::ensure!(
        l.actuals
            .as_ref()
            .is_none_or(|a| a.plies == l.spread.lays[0].allocated_plies),
        "The actual lay was shorter. Its old spreading release is retained for audit but cannot be re-exported as a machine instruction."
    );
    let spread = &l.spread.lays[0];
    let mut rows = vec![];
    for (i, s) in spread.segments.iter().enumerate() {
        let r = l
            .stock
            .rolls
            .iter()
            .find(|r| r.id == s.roll_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown roll."))?;
        let overlap = spread
            .splices
            .iter()
            .find(|j| {
                j.ply == s.ply
                    && j.to_roll == s.roll_id
                    && (j.marker_position_m - s.marker_start_m).abs() < 1e-8
            })
            .map_or(0.0, |j| j.overlap_m);
        rows.push(MachineRow {
            sequence: i + 1,
            lay: l.id.clone(),
            table: l.table.clone(),
            ply: s.ply,
            roll: r.id.clone(),
            lot: r.lot.clone(),
            dye_lot: r.dye_lot.clone(),
            shade: r.shade_group.clone(),
            roll_start_m: s.roll_start_m,
            table_start_m: s.marker_start_m,
            length_m: s.length_m,
            head_allowance_mm: l.end_allowance_mm,
            tail_allowance_mm: l.end_allowance_mm,
            splice_overlap_m: overlap,
            direction: "forward".into(),
            face: "up".into(),
        });
    }
    Ok(MachineFile {format:"beta-marker.spreader/v1".into(),note:"Generic exchange only. A factory-approved adapter and dry run are required for a machine. No speed, tension or controller commands are supplied.".into(),rows})
}
/// CSV representation of the same generic rows, with explicit schema and units.
pub fn machine_csv(l: &FloorLay) -> Result<Vec<u8>> {
    let file = machine(l)?;
    csv_table(
        &[
            "schema_version",
            "sequence",
            "lay",
            "table",
            "ply",
            "roll",
            "lot",
            "dye_lot",
            "shade",
            "roll_start_m",
            "table_start_m",
            "length_m",
            "head_allowance_mm",
            "tail_allowance_mm",
            "splice_overlap_m",
            "direction",
            "face",
        ],
        &file
            .rows
            .iter()
            .map(|r| {
                vec![
                    "1".into(),
                    r.sequence.to_string(),
                    r.lay.clone(),
                    r.table.clone(),
                    r.ply.to_string(),
                    r.roll.clone(),
                    r.lot.clone(),
                    r.dye_lot.clone(),
                    r.shade.clone(),
                    r.roll_start_m.to_string(),
                    r.table_start_m.to_string(),
                    r.length_m.to_string(),
                    r.head_allowance_mm.to_string(),
                    r.tail_allowance_mm.to_string(),
                    r.splice_overlap_m.to_string(),
                    r.direction.clone(),
                    r.face.clone(),
                ]
            })
            .collect::<Vec<_>>(),
    )
}
/// Literal HTML escaping for factory-supplied identifiers.
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn table(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    format!(
        "<table><thead><tr>{}</tr></thead><tbody>{}</tbody></table>",
        headers
            .iter()
            .map(|s| format!("<th scope=\"col\">{}</th>", escape(s)))
            .collect::<String>(),
        rows.iter()
            .map(|r| format!(
                "<tr>{}</tr>",
                r.iter()
                    .map(|s| format!("<td>{}</td>", escape(s)))
                    .collect::<String>()
            ))
            .collect::<String>()
    )
}
/// Printable A4 instructions, including physical roll sequence and sticker ordering.
pub fn lay_html(l: &FloorLay) -> Result<Vec<u8>> {
    let file = machine(l)?;
    let rows = file
        .rows
        .iter()
        .map(|r| {
            vec![
                r.sequence.to_string(),
                r.ply.to_string(),
                r.roll.clone(),
                format!("{} / {} / {}", r.lot, r.dye_lot, r.shade),
                format!("{:.3}", r.roll_start_m),
                format!("{:.3}", r.table_start_m),
                format!("{:.3}", r.length_m),
            ]
        })
        .collect();
    let splices = l.spread.lays[0]
        .splices
        .iter()
        .map(|s| {
            vec![
                s.ply.to_string(),
                s.from_roll.clone(),
                s.to_roll.clone(),
                format!("{:.3}", s.marker_position_m),
                format!("{:.3}", s.overlap_m),
            ]
        })
        .collect();
    let rolls = l.spread.lays[0]
        .allocations
        .iter()
        .map(|r| {
            vec![
                r.roll_id.clone(),
                r.plies.to_string(),
                r.full_plies.to_string(),
                format!("{:.3}", r.used_m),
                format!("{:.3}", r.end_bit_m),
            ]
        })
        .collect();
    let content = format!(
        "<h1>Spreading instructions · {}</h1><p>Order {} · {} · {} · Table {}</p><p><b>{} spread passes</b>; {:?} fabric. Marker {:.3} m × {:.0} mm. Allowance at EACH end: {:.1} mm per pass. Planned cloth: {:.3} m, including {:.3} m splice overlap.</p><p>Spread forward, face up, with left usable edges aligned. Follow sequence below. Check shade, shrinkage and safe zones against the released marker. Keep these instructions with the lay.</p><h2>Roll issue summary</h2>{}<h2>Execution sequence</h2>{}<h2>Splices</h2>{}<p>An outgoing segment ends at splice position + overlap. Incoming cloth starts at splice position. Coordinates start at the table head, before the marker head allowance.</p><h2>Ply numbering and stickers</h2><p>Print tickets/stickers from this saved lay. Sort the sticker-order CSV by its supplied sequence: ply, size, garment, part, slot, handedness. Every part of one garment and ply has the same printed number. A folded pass can cut both handed parts. Keep order + lay + bundle + physical piece identity; the printed number alone is not unique.</p><p>Spread by: __________ Checked by: __________ Actual plies: ______ Date: __________</p><p>Record issued and returned metres for every roll in Floor → Tracking. End bits are reusable returns, not automatically scrap. Register verified remnants under new stock IDs before reuse.</p><p class=\"note\">{}</p>",
        escape(&l.id),
        escape(&l.order),
        escape(&l.colour),
        escape(&l.date),
        escape(&l.table),
        l.spread.lays[0].allocated_plies,
        l.job.fabric.layout,
        l.marker.length / 1000.0,
        l.job.fabric.width,
        l.end_allowance_mm,
        l.spread.total_used_m,
        l.spread.splice_overlap_waste_m,
        table(
            &[
                "Roll",
                "Contributing plies",
                "Whole plies",
                "Used m",
                "End bit m"
            ],
            rolls
        ),
        table(
            &[
                "Seq",
                "Ply",
                "Roll",
                "Lot / dye / shade",
                "Roll start m",
                "Table start m",
                "Length m"
            ],
            rows
        ),
        table(
            &["Ply", "From roll", "To roll", "Position m", "Overlap m"],
            splices
        ),
        escape(&file.note)
    );
    Ok(format!("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>Lay sheet {}</title><style>body{{font:14px/1.5 system-ui,sans-serif;max-width:1050px;margin:24px auto;padding:16px;color:#172e26}}table{{border-collapse:collapse;width:100%;font-size:12px}}th,td{{padding:6px;border:1px solid #a7b8ae;text-align:left;overflow-wrap:anywhere}}thead{{display:table-header-group}}tr{{break-inside:avoid}}h2{{break-after:avoid}}.note{{font-size:12px}}@page{{size:A4 landscape;margin:12mm}}@media print{{body{{margin:0;padding:0}}}}</style>{content}</html>",escape(&l.id)).into_bytes())
}
/// Deterministic ply-first numbering print manifest, preserving exact piece identity.
pub fn stickers_csv(l: &FloorLay) -> Result<Vec<u8>> {
    let mut rows = l
        .bundles
        .iter()
        .flat_map(|b| b.stickers.iter().map(move |s| (b, s)))
        .collect::<Vec<_>>();
    rows.sort_by(|(a, x), (b, y)| {
        (
            x.piece.ply,
            &a.size,
            x.piece.garment,
            &x.piece.piece_id,
            x.piece.slot,
            x.piece.mirrored,
        )
            .cmp(&(
                y.piece.ply,
                &b.size,
                y.piece.garment,
                &y.piece.piece_id,
                y.piece.slot,
                y.piece.mirrored,
            ))
    });
    csv_table(
        &[
            "sequence",
            "order",
            "lay",
            "bundle",
            "ply",
            "size",
            "garment",
            "part",
            "slot",
            "mirrored",
            "number",
            "last_number",
            "shade",
        ],
        &rows
            .iter()
            .enumerate()
            .map(|(i, (b, s))| {
                vec![
                    (i + 1).to_string(),
                    b.order.clone(),
                    b.lay.clone(),
                    b.number.to_string(),
                    s.piece.ply.to_string(),
                    b.size.clone(),
                    s.piece.garment.to_string(),
                    s.piece.piece_id.clone(),
                    s.piece.slot.to_string(),
                    s.piece.mirrored.to_string(),
                    s.number.to_string(),
                    s.last_number.to_string(),
                    b.shade_group.clone(),
                ]
            })
            .collect::<Vec<_>>(),
    )
}
/// BETA Time exchange counts parts, not completed garments or accounting postings.
pub fn wip_csv(s: &FloorState) -> Result<Vec<u8>> {
    csv_table(
        &[
            "schema_version",
            "event_id",
            "date",
            "line",
            "operator",
            "order",
            "style",
            "colour",
            "lay",
            "bundle",
            "size",
            "part",
            "mirrored",
            "shade",
            "first_ply",
            "last_ply",
            "issued_parts",
            "status",
        ],
        &s.issues
            .iter()
            .map(|i| {
                let p = &i.payload;
                vec![
                    "1".into(),
                    i.event_id.clone(),
                    i.date.clone(),
                    i.line.clone(),
                    i.operator.clone(),
                    p.o.clone(),
                    p.s.clone(),
                    p.c.clone(),
                    p.l.clone(),
                    p.b.to_string(),
                    p.z.clone(),
                    p.part.clone(),
                    p.m.to_string(),
                    p.h.clone(),
                    p.p[0].to_string(),
                    p.p[1].to_string(),
                    p.q.to_string(),
                    "issued".into(),
                ]
            })
            .collect::<Vec<_>>(),
    )
}
/// Planned-versus-measured roll reconciliation for ERP review.
pub fn consumption_csv(s: &FloorState) -> Result<Vec<u8>> {
    crate::intake::csv_numeric_table(
        &[
            "schema_version",
            "order",
            "lay",
            "date",
            "status",
            "planned_m",
            "actual_used_m",
            "returned_m",
            "variance_m",
            "actual_plies",
        ],
        &s.lays
            .iter()
            .map(|l| {
                let actual = l.actuals.as_ref().map(|a| {
                    a.rolls
                        .iter()
                        .map(|r| r.issued_m - r.returned_m)
                        .sum::<f64>()
                });
                vec![
                    "1".into(),
                    l.order.clone(),
                    l.id.clone(),
                    l.date.clone(),
                    format!("{:?}", l.status).to_lowercase(),
                    l.spread.total_used_m.to_string(),
                    actual.map(|x| x.to_string()).unwrap_or_default(),
                    l.actuals
                        .as_ref()
                        .map(|a| {
                            a.rolls
                                .iter()
                                .map(|r| r.returned_m)
                                .sum::<f64>()
                                .to_string()
                        })
                        .unwrap_or_default(),
                    actual
                        .map(|x| (x - l.spread.total_used_m).to_string())
                        .unwrap_or_default(),
                    l.actuals
                        .as_ref()
                        .map(|a| a.plies.to_string())
                        .unwrap_or_default(),
                ]
            })
            .collect::<Vec<_>>(),
        &[0, 5, 6, 7, 8, 9],
    )
}
