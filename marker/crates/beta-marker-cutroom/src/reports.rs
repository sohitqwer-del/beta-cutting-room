//! Cutting-room reconciliation. Fabric issued minus returned end bits equals actual use.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::Write;

/// One spreading record, summed across rolls. All lengths are physical roll lengths.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SpreadingRecord {
    /// Length removed from roll inventory, including returned remnants.
    pub issued_mm: f64,
    /// Reusable end bits returned to inventory; excluded from actual use.
    pub end_bits_mm: f64,
}
/// Measurements for one lay; actual cut quantities come from the cutting record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayActuals {
    /// Unique lay id.
    pub lay: String,
    /// Order id.
    pub order: String,
    /// Physical fabric width, mm.
    pub width_mm: f64,
    /// Planned marker length, mm.
    pub marker_length_mm: f64,
    /// Planned physical plies (expand folded-layer accounting in the adapter).
    pub plies: u32,
    /// Total head plus tail allowance on EACH ply, mm.
    pub end_allowance_per_ply_mm: f64,
    /// Planned outline area in a single physical ply, mm².
    pub piece_area_per_ply_mm2: f64,
    /// Planned physical pieces per ply.
    pub pieces_per_ply: u32,
    /// Actual outline area cut, including replacement pieces if included in actual pieces.
    pub actual_piece_area_mm2: f64,
    /// Actual physical pieces cut; state consistently whether replacements are included.
    pub actual_pieces: u64,
    /// Number of replacement pieces requested (a subset, not added to actual pieces).
    pub recut_count: u64,
    /// Roll withdrawals and end-bit returns.
    pub spreading: Vec<SpreadingRecord>,
}
/// Additive report metrics, usable for one lay or a whole order with mixed widths.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CuttingReport {
    /// `lay` or `order`.
    pub scope: String,
    /// Lay/order id.
    pub id: String,
    /// Order id.
    pub order: String,
    /// Fabric including planned end allowances, mm.
    pub planned_mm: f64,
    /// Issued minus end bits, mm.
    pub actual_mm: f64,
    /// Total issued length, mm.
    pub issued_mm: f64,
    /// Returned reusable remnants, mm.
    pub end_bits_mm: f64,
    /// Actual minus planned length, mm; positive is overuse.
    pub variance_mm: f64,
    /// Actual outline area divided by actual fabric area, percent; absent before spreading.
    pub cutting_efficiency_pct: Option<f64>,
    /// Planned outline area divided by marker fabric area, percent (excludes end allowances).
    pub marker_efficiency_pct: f64,
    /// Planned physical piece count.
    pub planned_pieces: u64,
    /// Actual pieces cut.
    pub pieces_cut: u64,
    /// Requested replacements.
    pub recut_count: u64,
    /// Planned outline area, retained for area-weighted order reporting.
    pub planned_piece_area_mm2: f64,
    /// Planned width × marker length × plies, excluding end allowance.
    pub marker_fabric_area_mm2: f64,
    /// Actual fabric area; sum of width × actual length, for mixed-width orders.
    pub actual_fabric_area_mm2: f64,
    /// Actual outline area.
    pub actual_piece_area_mm2: f64,
}
fn finite_nonnegative(v: f64) -> bool {
    v.is_finite() && v >= 0.0
}
/// Reconcile one lay. Reject non-finite values, impossible areas and excessive returns.
pub fn lay_report(a: &LayActuals) -> Result<CuttingReport> {
    ensure!(
        !a.lay.trim().is_empty() && !a.order.trim().is_empty(),
        "lay and order are required"
    );
    ensure!(
        a.plies > 0
            && a.pieces_per_ply > 0
            && a.width_mm.is_finite()
            && a.width_mm > 0.0
            && a.marker_length_mm.is_finite()
            && a.marker_length_mm > 0.0,
        "width, marker length, pieces and plies must be positive"
    );
    ensure!(
        [
            a.end_allowance_per_ply_mm,
            a.piece_area_per_ply_mm2,
            a.actual_piece_area_mm2
        ]
        .iter()
        .all(|v| finite_nonnegative(*v)),
        "allowances and areas must be finite and nonnegative"
    );
    ensure!(
        a.piece_area_per_ply_mm2 > 0.0
            && a.piece_area_per_ply_mm2 <= a.width_mm * a.marker_length_mm + 0.01,
        "planned piece area exceeds marker fabric area"
    );
    let mut issued = 0.0;
    let mut ends = 0.0;
    for s in &a.spreading {
        ensure!(
            finite_nonnegative(s.issued_mm)
                && finite_nonnegative(s.end_bits_mm)
                && s.end_bits_mm <= s.issued_mm,
            "invalid spreading withdrawal or end-bit return"
        );
        issued += s.issued_mm;
        ends += s.end_bits_mm;
    }
    let actual = issued - ends;
    let area = actual * a.width_mm;
    ensure!(
        a.actual_piece_area_mm2 <= area + 0.01
            && (a.actual_pieces == 0) == (a.actual_piece_area_mm2 == 0.0),
        "actual pieces/area do not agree with fabric use"
    );
    let marker_area = a.width_mm * a.marker_length_mm * f64::from(a.plies);
    let planned_piece_area = a.piece_area_per_ply_mm2 * f64::from(a.plies);
    let planned = (a.marker_length_mm + a.end_allowance_per_ply_mm) * f64::from(a.plies);
    ensure!(
        [
            issued,
            ends,
            actual,
            area,
            marker_area,
            planned_piece_area,
            planned
        ]
        .iter()
        .all(|v| v.is_finite()),
        "report numeric overflow"
    );
    Ok(CuttingReport {
        scope: "lay".into(),
        id: a.lay.clone(),
        order: a.order.clone(),
        planned_mm: planned,
        actual_mm: actual,
        issued_mm: issued,
        end_bits_mm: ends,
        variance_mm: actual - planned,
        cutting_efficiency_pct: if area > 0.0 {
            Some(a.actual_piece_area_mm2 / area * 100.0)
        } else {
            None
        },
        marker_efficiency_pct: planned_piece_area / marker_area * 100.0,
        planned_pieces: u64::from(a.pieces_per_ply) * u64::from(a.plies),
        pieces_cut: a.actual_pieces,
        recut_count: a.recut_count,
        planned_piece_area_mm2: planned_piece_area,
        marker_fabric_area_mm2: marker_area,
        actual_fabric_area_mm2: area,
        actual_piece_area_mm2: a.actual_piece_area_mm2,
    })
}
/// Aggregate original lay measurements, rejecting duplicate lays and mixed order ids.
/// Efficiencies use total areas, never the average of lay percentages.
pub fn order_report(lays: &[LayActuals]) -> Result<CuttingReport> {
    ensure!(!lays.is_empty(), "order report needs at least one lay");
    let mut seen = BTreeSet::new();
    let reports = lays
        .iter()
        .map(|a| {
            ensure!(
                a.order == lays[0].order && seen.insert(&a.lay),
                "mixed orders or duplicate lays"
            );
            lay_report(a)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut r = reports[0].clone();
    r.scope = "order".into();
    r.id = r.order.clone();
    for a in reports.iter().skip(1) {
        r.planned_mm += a.planned_mm;
        r.actual_mm += a.actual_mm;
        r.issued_mm += a.issued_mm;
        r.end_bits_mm += a.end_bits_mm;
        r.planned_piece_area_mm2 += a.planned_piece_area_mm2;
        r.marker_fabric_area_mm2 += a.marker_fabric_area_mm2;
        r.actual_fabric_area_mm2 += a.actual_fabric_area_mm2;
        r.actual_piece_area_mm2 += a.actual_piece_area_mm2;
        r.planned_pieces = r
            .planned_pieces
            .checked_add(a.planned_pieces)
            .ok_or_else(|| anyhow::anyhow!("piece count overflow"))?;
        r.pieces_cut = r
            .pieces_cut
            .checked_add(a.pieces_cut)
            .ok_or_else(|| anyhow::anyhow!("piece count overflow"))?;
        r.recut_count = r
            .recut_count
            .checked_add(a.recut_count)
            .ok_or_else(|| anyhow::anyhow!("re-cut count overflow"))?;
    }
    ensure!(
        [
            r.planned_mm,
            r.actual_mm,
            r.issued_mm,
            r.end_bits_mm,
            r.planned_piece_area_mm2,
            r.marker_fabric_area_mm2,
            r.actual_fabric_area_mm2,
            r.actual_piece_area_mm2
        ]
        .iter()
        .all(|v| v.is_finite()),
        "order numeric overflow"
    );
    r.variance_mm = r.actual_mm - r.planned_mm;
    r.marker_efficiency_pct = r.planned_piece_area_mm2 / r.marker_fabric_area_mm2 * 100.0;
    r.cutting_efficiency_pct = if r.actual_fabric_area_mm2 > 0.0 {
        Some(r.actual_piece_area_mm2 / r.actual_fabric_area_mm2 * 100.0)
    } else {
        None
    };
    Ok(r)
}
fn csv_field(s: &str) -> String {
    // CSV quoting alone does not stop a spreadsheet from evaluating a formula.
    let prefix =
        if s.trim_start().starts_with(['=', '+', '-', '@']) || s.starts_with(['\t', '\r', '\n']) {
            "'"
        } else {
            ""
        };
    format!("\"{prefix}{}\"", s.replace('"', "\"\""))
}
/// RFC 4180 CSV with explicit units and CRLF records. Formula-leading text is
/// prefixed with an apostrophe for spreadsheet safety; numeric values stay numeric.
pub fn reports_csv(reports: &[CuttingReport]) -> String {
    let mut out = String::from(
        "scope,id,order,planned_mm,actual_mm,issued_mm,end_bits_mm,variance_mm,cutting_efficiency_pct,marker_efficiency_pct,planned_pieces,pieces_cut,recut_count\r\n",
    );
    for r in reports {
        let efficiency = r
            .cutting_efficiency_pct
            .map(|v| format!("{v:.4}"))
            .unwrap_or_default();
        // Formatting into a String cannot fail.
        let _ = write!(
            out,
            "{},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{},{:.4},{},{},{}\r\n",
            csv_field(&r.scope),
            csv_field(&r.id),
            csv_field(&r.order),
            r.planned_mm,
            r.actual_mm,
            r.issued_mm,
            r.end_bits_mm,
            r.variance_mm,
            efficiency,
            r.marker_efficiency_pct,
            r.planned_pieces,
            r.pieces_cut,
            r.recut_count
        );
    }
    out
}
/// Plain-English report. Lengths are shown in metres; percentages retain their meaning.
pub fn report_text(r: &CuttingReport) -> String {
    format!(
        "{} {} (order {})\nFabric planned: {:.3} m; actual: {:.3} m; variance: {:+.3} m\nIssued: {:.3} m; returned end bits: {:.3} m\nMarker efficiency: {:.2}%; cutting efficiency: {}\nPieces cut: {} / {}; re-cuts requested: {}\n",
        r.scope,
        r.id,
        r.order,
        r.planned_mm / 1000.0,
        r.actual_mm / 1000.0,
        r.variance_mm / 1000.0,
        r.issued_mm / 1000.0,
        r.end_bits_mm / 1000.0,
        r.marker_efficiency_pct,
        r.cutting_efficiency_pct
            .map_or_else(|| "not recorded".into(), |v| format!("{v:.2}%")),
        r.pieces_cut,
        r.planned_pieces,
        r.recut_count
    )
}
