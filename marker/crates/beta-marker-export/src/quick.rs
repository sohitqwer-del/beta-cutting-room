//! Quick Marker outputs: fixed physical tiles, a measurement guide and projector JSON.
use crate::{ExportOptions, pdf, scene};
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Job, MarkerResult, Pt};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Write};
const PT: f64 = 72. / 25.4;
const MARGIN: f64 = 10.;
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum Paper {
    #[default]
    A4,
    A3,
}
impl Paper {
    fn dimensions(self) -> Pt {
        match self {
            Self::A4 => [210., 297.],
            Self::A3 => [297., 420.],
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TiledMarkerOptions {
    pub paper: Paper,
    pub overlap_mm: f64,
}
impl Default for TiledMarkerOptions {
    fn default() -> Self {
        Self {
            paper: Paper::A4,
            overlap_mm: 10.,
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ChalkGuideOptions {
    pub language: Language,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Hi,
    Ta,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectedPiece {
    pub piece_id: String,
    pub name: String,
    pub size: Option<String>,
    pub copy: u32,
    pub outline: Vec<Pt>,
    pub reference_point: Pt,
    pub rotation_deg: f64,
    pub mirrored: bool,
    pub on_fold: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectorLayout {
    pub format: String,
    pub units: String,
    pub length_mm: f64,
    pub width_mm: f64,
    pub calibration_square_mm: f64,
    pub pieces: Vec<ProjectedPiece>,
}
fn validated(job: &Job, result: &MarkerResult) -> Result<Job> {
    ensure!(
        job.pieces.len() <= 10000 && result.placements.len() <= 10000,
        "Too many placements for phone export"
    );
    ensure!(
        job.pieces.iter().map(|p| p.outline.len()).sum::<usize>() <= 500000,
        "Simplify this job before phone export"
    );
    let errors = beta_marker_core::check_job(job);
    ensure!(
        errors.is_empty(),
        "Invalid job: {}",
        serde_json::to_string(&errors)?
    );
    let report = beta_marker_core::validate(job, result);
    ensure!(
        report.ok,
        "Validate this marker before export: {}",
        serde_json::to_string(&report.errors)?
    );
    ensure!(!result.placements.is_empty(), "Place pieces before export");
    Ok(beta_marker_core::rules::effective_job(job))
}
/// Coordinates reflect actual cut geometry, including shrinkage, fold halves and blocks.
pub fn projector_layout(job: &Job, result: &MarkerResult) -> Result<ProjectorLayout> {
    let effective = validated(job, result)?;
    let map: BTreeMap<_, _> = effective
        .pieces
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect();
    let mut pieces = Vec::new();
    for pl in &result.placements {
        let p = map
            .get(pl.piece_id.as_str())
            .context("Unknown placed piece")?;
        let outline = beta_marker_core::rules::cut_outline(p, pl);
        let reference_point = *outline.first().context("Empty placed outline")?;
        pieces.push(ProjectedPiece {
            piece_id: p.id.clone(),
            name: p.name.clone(),
            size: p.size.clone(),
            copy: pl.copy,
            outline,
            reference_point,
            rotation_deg: pl.rotation_deg,
            mirrored: pl.mirrored,
            on_fold: pl.on_fold,
        });
    }
    Ok(ProjectorLayout {
        format: "beta-marker.projector/v1".into(),
        units: "mm".into(),
        length_mm: beta_marker_core::compute_metrics(&effective, &result.placements).length,
        width_mm: effective.fabric.width,
        calibration_square_mm: 100.,
        pieces,
    })
}
fn text(out: &mut String, x: f64, y: f64, value: &str, size: f64) -> Result<()> {
    writeln!(
        out,
        "BT /F1 {size} Tf 1 0 0 1 {} {} Tm ({}) Tj ET",
        x * PT,
        y * PT,
        pdf::escaped(value)
    )?;
    Ok(())
}
fn cross(out: &mut String, x: f64, y: f64) -> Result<()> {
    writeln!(
        out,
        "{} {} m {} {} l S {} {} m {} {} l S",
        (x - 2.) * PT,
        y * PT,
        (x + 2.) * PT,
        y * PT,
        x * PT,
        (y - 2.) * PT,
        x * PT,
        (y + 2.) * PT
    )?;
    Ok(())
}
fn tile_count(extent: f64, usable: f64, overlap: f64) -> usize {
    if extent <= usable {
        1
    } else {
        ((extent - usable) / (usable - overlap)).ceil() as usize + 1
    }
}
fn form_pages(contents: &[String], size: Pt, content: &str, bounds: Pt) -> Result<Vec<u8>> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R /ViewerPreferences << /PrintScaling /None >> >>".to_vec(),
        vec![],
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        pdf::stream(
            content,
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R >> >>",
                bounds[0] * PT,
                bounds[1] * PT
            ),
        ),
    ];
    let mut kids = String::new();
    for content in contents {
        let page = objects.len() + 1;
        write!(kids, "{page} 0 R ")?;
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /XObject << /Marker 4 0 R >> /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",size[0]*PT,size[1]*PT,page+1).into_bytes());
        objects.push(pdf::stream(content, ""));
    }
    objects[1] = format!(
        "<< /Type /Pages /Count {} /Kids [{kids}] >>",
        contents.len()
    )
    .into_bytes();
    pdf::finish(objects)
}
/// Full-size A4/A3 tiles. The overview is extra; every tile has the chosen paper size.
pub fn tiled_marker_pdf(
    job: &Job,
    result: &MarkerResult,
    options: &TiledMarkerOptions,
) -> Result<Vec<u8>> {
    let effective = validated(job, result)?;
    ensure!(
        options.overlap_mm.is_finite() && (5. ..=30.).contains(&options.overlap_mm),
        "Tile overlap must be 5..30 mm"
    );
    let mut scene = scene::build_without_guides(
        job,
        result,
        &ExportOptions {
            labels: false,
            ..Default::default()
        },
    )?;
    let by_id: BTreeMap<_, _> = effective
        .pieces
        .iter()
        .map(|p| (p.id.as_str(), p))
        .collect();
    for placement in &result.placements {
        let piece = by_id[placement.piece_id.as_str()];
        let outline = beta_marker_core::rules::cut_outline(piece, placement);
        let b = beta_marker_core::geometry::bbox(&outline).context("Empty placed outline")?;
        scene.texts.push(scene::Text {
            point: [b[0] + 1., (b[1] + b[3]) / 2.],
            footer: false,
            value: format!(
                "{} | {} | copy {}{}{}",
                piece.name,
                piece.size.as_deref().unwrap_or("unsized"),
                u64::from(placement.copy) + 1,
                if placement.mirrored {
                    " | face down"
                } else {
                    ""
                },
                if placement.on_fold { " | on fold" } else { "" }
            ),
        });
    }
    let [pw, ph] = options.paper.dimensions();
    let uw = pw - 2. * MARGIN;
    let uh = ph - 2. * MARGIN;
    let overlap = options.overlap_mm;
    let sx = uw - overlap;
    let sy = uh - overlap;
    let nx = tile_count(scene.length, uw, overlap);
    let ny = tile_count(scene.width, uh, overlap);
    ensure!(
        nx.checked_mul(ny).is_some_and(|n| n <= 2000),
        "This marker needs more than 2000 pages. Use A3 or a plotter"
    );
    let mut pages = Vec::with_capacity(nx * ny + 1);
    let mut map = String::from("0 0 0 RG 0 0 0 rg 0.4 w\n");
    text(
        &mut map,
        10.,
        ph - 18.,
        "BETA Quick Marker - assembly map",
        15.,
    )?;
    text(
        &mut map,
        10.,
        ph - 28.,
        "Print every page at 100% / actual size. Disable fit to page.",
        10.,
    )?;
    text(
        &mut map,
        10.,
        ph - 36.,
        &format!(
            "{} tiles: {} columns x {} rows. Overlap {:.0} mm.",
            nx * ny,
            nx,
            ny,
            overlap
        ),
        10.,
    )?;
    text(
        &mut map,
        10.,
        ph - 44.,
        "Trim one edge of each overlap. Match crosses and outline segments.",
        9.,
    )?;
    text(
        &mut map,
        10.,
        ph - 51.,
        "Column increases along fabric length. Row increases from the y=0 selvedge.",
        9.,
    )?;
    let factor = (uw / scene.length).min((ph - 115.) / scene.width);
    let mx = MARGIN + (uw - scene.length * factor) / 2.;
    let my = 55.;
    writeln!(
        map,
        "q {factor} 0 0 {factor} {} {} cm /Marker Do Q",
        mx * PT,
        my * PT
    )?;
    for row in 0..ny {
        for col in 0..nx {
            let x = mx + col as f64 * sx * factor;
            let y = my + row as f64 * sy * factor;
            let w = (scene.length - col as f64 * sx).min(uw) * factor;
            let h = (scene.width - row as f64 * sy).min(uh) * factor;
            writeln!(
                map,
                "0.6 0.6 0.6 RG {} {} {} {} re S",
                x * PT,
                y * PT,
                w * PT,
                h * PT
            )?;
            text(
                &mut map,
                x + 1.,
                y + 1.,
                &format!("{}", row * nx + col + 1),
                7_f64.min(w * PT / 3.),
            )?;
        }
    }
    text(
        &mut map,
        10.,
        40.,
        "Scale check: this line must measure exactly 100 mm.",
        10.,
    )?;
    writeln!(
        map,
        "0 0 0 RG 0.6 w {} {} m {} {} l S",
        10. * PT,
        31. * PT,
        110. * PT,
        31. * PT
    )?;
    cross(&mut map, 10., 31.)?;
    cross(&mut map, 110., 31.)?;
    text(
        &mut map,
        10.,
        19.,
        "Map is reduced. Only the numbered tile pages are 1:1.",
        9.,
    )?;
    pages.push(map);
    for row in 0..ny {
        for col in 0..nx {
            let x = col as f64 * sx;
            let y = row as f64 * sy;
            let mut out = format!(
                "q {} {} {} {} re W n\n1 0 0 1 {} {} cm\n/Marker Do Q\n0 0 0 RG 0 0 0 rg 0.4 w\n",
                MARGIN * PT,
                MARGIN * PT,
                uw * PT,
                uh * PT,
                (MARGIN - x) * PT,
                (MARGIN - y) * PT
            );
            // Opposite pages print crosses at identical world coordinates in each overlap.
            for cx in [
                if col > 0 {
                    Some(MARGIN + overlap / 2.)
                } else {
                    None
                },
                if col + 1 < nx {
                    Some(MARGIN + uw - overlap / 2.)
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            {
                for cy in [MARGIN + 20., MARGIN + uh - 20.] {
                    cross(&mut out, cx, cy)?;
                }
            }
            for cy in [
                if row > 0 {
                    Some(MARGIN + overlap / 2.)
                } else {
                    None
                },
                if row + 1 < ny {
                    Some(MARGIN + uh - overlap / 2.)
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            {
                for cx in [MARGIN + 20., MARGIN + uw - 20.] {
                    cross(&mut out, cx, cy)?;
                }
            }
            text(
                &mut out,
                10.,
                ph - 6.,
                &format!(
                    "Tile {}/{} - column {} row {} - PRINT 100%",
                    row * nx + col + 1,
                    nx * ny,
                    col + 1,
                    row + 1
                ),
                8.,
            )?;
            text(
                &mut out,
                10.,
                4.,
                &format!(
                    "Origin X {:.1} mm, Y {:.1} mm | overlap {:.0} mm | scale check on map",
                    x, y, overlap
                ),
                7.,
            )?;
            pages.push(out);
        }
    }
    form_pages(
        &pages,
        [pw, ph],
        &pdf::scene_content(&scene)?,
        [scene.length, scene.width],
    )
}
#[derive(Deserialize)]
struct GlyphLabel {
    text: String,
    units: f64,
    width: f64,
    path: String,
}
fn native_label(
    out: &mut String,
    labels: &BTreeMap<String, GlyphLabel>,
    language: Language,
    key: &str,
    point: Pt,
    max_width: f64,
    size: f64,
) -> Result<()> {
    let prefix = match language {
        Language::En => return Ok(()),
        Language::Hi => "hi",
        Language::Ta => "ta",
    };
    let label = labels
        .get(&format!("{prefix}.{key}"))
        .context("Missing translated label")?;
    let scale = (size / label.units).min(max_width * PT / label.width);
    // ActualText retains searchable Unicode while the pre-shaped glyph paths render offline.
    let actual = label
        .text
        .encode_utf16()
        .map(|u| format!("{u:04X}"))
        .collect::<String>();
    writeln!(
        out,
        "/Span << /ActualText <FEFF{actual}> >> BDC q {scale} 0 0 {scale} {} {} cm\n{}Q EMC",
        point[0] * PT,
        point[1] * PT,
        label.path
    )?;
    Ok(())
}
fn clipped_text(
    out: &mut String,
    x: f64,
    y: f64,
    width: f64,
    value: &str,
    size: f64,
) -> Result<()> {
    let limit = (width * PT / (size * 0.55)).floor() as usize;
    let value = scene::ascii(value);
    let display = if value.len() > limit {
        format!(
            "{}...",
            value
                .chars()
                .take(limit.saturating_sub(3))
                .collect::<String>()
        )
    } else {
        value
    };
    text(out, x, y, &display, size)
}
/// A4 placement measurements. Reference points are explicit vertices, not bounding-box corners.
pub fn chalk_guide_pdf(
    job: &Job,
    result: &MarkerResult,
    options: &ChalkGuideOptions,
) -> Result<Vec<u8>> {
    let layout = projector_layout(job, result)?;
    let labels: BTreeMap<String, GlyphLabel> =
        serde_json::from_str(include_str!("../fonts/chalk-labels.json"))?;
    let scene = scene::build_without_guides(
        job,
        result,
        &ExportOptions {
            labels: false,
            ..Default::default()
        },
    )?;
    let content = pdf::scene_content(&scene)?;
    let per_page = 12;
    let count = layout.pieces.len().div_ceil(per_page);
    let mut pages = Vec::new();
    for (page, chunk) in layout.pieces.chunks(per_page).enumerate() {
        let mut out = String::from("0 0 0 RG 0 0 0 rg 0.4 w\n");
        text(&mut out, 10., 282., "BETA Quick Marker - chalk guide", 15.)?;
        native_label(
            &mut out,
            &labels,
            options.language,
            "title",
            [10., 271.],
            155.,
            12.,
        )?;
        let review = if matches!(options.language, Language::En) {
            "Placement measurements; check every piece before chalking."
        } else {
            "Hindi/Tamil labels: needs native review. English labels are retained."
        };
        text(&mut out, 10., 262., review, 9.)?;
        text(
            &mut out,
            10.,
            254.,
            &format!(
                "Length {:.1} mm | width {:.1} mm | guide page {}/{}",
                layout.length_mm,
                layout.width_mm,
                page + 1,
                count
            ),
            10.,
        )?;
        let factor = (185. / layout.length_mm).min(60. / layout.width_mm);
        let map_x = 10. + (185. - layout.length_mm * factor) / 2.;
        writeln!(
            out,
            "q {factor} 0 0 {factor} {} {} cm /Marker Do Q",
            map_x * PT,
            184. * PT
        )?;
        for (index, p) in layout.pieces.iter().enumerate() {
            let [x, y] = [
                map_x + p.reference_point[0] * factor,
                184. + p.reference_point[1] * factor,
            ];
            cross(&mut out, x, y)?;
            text(&mut out, x + 1., y + 1., &(index + 1).to_string(), 6.)?;
        }
        text(
            &mut out,
            10.,
            176.,
            "X: from marker start. Y: from the y=0 selvedge. All distances in mm.",
            9.,
        )?;
        text(
            &mut out,
            10.,
            169.,
            "Cross = first vertex of the placed outline. Use the matching paper piece.",
            9.,
        )?;
        let columns = [
            (10., 8., "#", ""),
            (18., 58., "Piece", "piece"),
            (76., 22., "Size", "size"),
            (98., 23., "Turn deg", "rotation"),
            (121., 18., "Flip", "flip"),
            (139., 30., "X mm", "x"),
            (169., 31., "Y mm", "y"),
        ];
        for (x, width, title, key) in columns {
            text(&mut out, x, 157., title, 8.)?;
            if !key.is_empty() {
                native_label(
                    &mut out,
                    &labels,
                    options.language,
                    key,
                    [x, 150.],
                    width - 1.,
                    8.,
                )?;
            }
        }
        for (row, p) in chunk.iter().enumerate() {
            let y = 138. - row as f64 * 9.;
            let index = page * per_page + row + 1;
            writeln!(
                out,
                "0.8 0.8 0.8 RG {} {} m {} {} l S 0 0 0 RG",
                10. * PT,
                (y - 3.) * PT,
                200. * PT,
                (y - 3.) * PT
            )?;
            let vals = [
                index.to_string(),
                format!("{} [{}]", p.name, p.copy + 1),
                p.size.clone().unwrap_or_else(|| "unsized".into()),
                format!("{:.1}", p.rotation_deg),
                if p.mirrored { "Yes" } else { "No" }.into(),
                format!("{:.1}", p.reference_point[0]),
                format!("{:.1}", p.reference_point[1]),
            ];
            for ((x, width, _, _), value) in columns.iter().zip(vals) {
                clipped_text(&mut out, *x, y, *width - 1., &value, 8.)?;
            }
        }
        text(
            &mut out,
            10.,
            21.,
            "Turn is counter-clockwise from the accepted photo outline. Flip means face down.",
            8.,
        )?;
        text(
            &mut out,
            10.,
            14.,
            "Map is reduced, not a full-size cutting template. Mark the reference vertex on paper.",
            8.,
        )?;
        pages.push(out);
    }
    form_pages(&pages, [210., 297.], &content, [scene.length, scene.width])
}
#[cfg(test)]
mod tests {
    use super::*;
    use beta_marker_core::{Fabric, JOB_FORMAT, Piece, Placement, RESULT_FORMAT};
    fn fixture() -> (Job, MarkerResult) {
        let job = Job {
            format: JOB_FORMAT.into(),
            name: "test".into(),
            fabric: Fabric {
                width: 600.,
                ..Default::default()
            },
            pieces: vec![Piece {
                id: "p".into(),
                name: "Panel".into(),
                size: Some("M".into()),
                outline: vec![[0., 0.], [200., 0.], [200., 300.], [0., 300.]],
                quantity: 1,
                ..Default::default()
            }],
            ..Default::default()
        };
        let result = MarkerResult {
            format: RESULT_FORMAT.into(),
            job_name: "test".into(),
            width: 600.,
            length: 200.,
            piece_area: 60000.,
            efficiency: 0.5,
            engine: "test".into(),
            seed: None,
            runtime_s: 0.,
            warnings: vec![],
            placements: vec![Placement {
                piece_id: "p".into(),
                ..Default::default()
            }],
        };
        (job, result)
    }
    #[test]
    fn a4_a3_page_counts_physical_scale_and_overlap() {
        let (job, result) = fixture();
        let a4 = tiled_marker_pdf(&job, &result, &TiledMarkerOptions::default()).unwrap();
        let a4 = String::from_utf8_lossy(&a4);
        assert!(a4.contains("/Count 7 "));
        assert_eq!(
            a4.matches(&format!("/MediaBox [0 0 {} {}]", 210. * PT, 297. * PT))
                .count(),
            7
        );
        assert!(a4.contains("PrintScaling /None"));
        assert!(a4.contains("100 mm"));
        assert!(a4.contains(&format!("{} {} l", 200. * PT, 300. * PT)));
        // Same world X=185 mm cross on adjacent columns: page 1 x=195,
        // page 2 x=15 with the form translated by -170 mm (including margin).
        assert!(a4.contains(&format!(
            "{} {} m {} {} l S",
            193. * PT,
            30. * PT,
            197. * PT,
            30. * PT
        )));
        assert!(a4.contains(&format!(
            "{} {} m {} {} l S",
            13. * PT,
            30. * PT,
            17. * PT,
            30. * PT
        )));
        let a3 = tiled_marker_pdf(
            &job,
            &result,
            &TiledMarkerOptions {
                paper: Paper::A3,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(String::from_utf8_lossy(&a3).contains("/Count 3 "));
    }
    #[test]
    fn output_rejects_bad_overlap_missing_pieces_and_nonfinite_input() {
        let (mut job, mut result) = fixture();
        assert!(
            tiled_marker_pdf(
                &job,
                &result,
                &TiledMarkerOptions {
                    overlap_mm: f64::NAN,
                    ..Default::default()
                }
            )
            .is_err()
        );
        result.placements.clear();
        assert!(chalk_guide_pdf(&job, &result, &ChalkGuideOptions::default()).is_err());
        assert!(projector_layout(&job, &result).is_err());
        job.fabric.width = f64::INFINITY;
        assert!(tiled_marker_pdf(&job, &result, &TiledMarkerOptions::default()).is_err());
    }
    #[test]
    fn projector_recomputes_length_and_identifies_actual_reference_vertex() {
        let (job, mut result) = fixture();
        result.length = 999.;
        let p = projector_layout(&job, &result).unwrap();
        assert_eq!(p.length_mm, 200.);
        assert_eq!(p.calibration_square_mm, 100.);
        assert_eq!(p.pieces[0].reference_point, p.pieces[0].outline[0]);
    }
    #[test]
    fn hindi_and_tamil_are_vector_glyphs_with_unicode_actual_text() {
        let (job, result) = fixture();
        for language in [Language::Hi, Language::Ta] {
            let bytes = chalk_guide_pdf(&job, &result, &ChalkGuideOptions { language }).unwrap();
            let pdf = String::from_utf8_lossy(&bytes);
            assert!(pdf.contains("/ActualText <FEFF"));
            assert!(pdf.contains("needs native review"));
            assert!(pdf.contains("/Count 1 "));
            assert!(pdf.contains(" c\n"));
            assert!(!pdf.contains("????"));
        }
    }
}
