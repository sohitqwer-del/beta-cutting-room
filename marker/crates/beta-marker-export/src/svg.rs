use crate::scene::Scene;
use std::fmt::Write;
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub(super) fn render(s: &Scene) -> anyhow::Result<String> {
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}mm\" height=\"{}mm\" viewBox=\"0 0 {} {}\" role=\"img\">\n<title>Marker preview</title>\n<g transform=\"translate(0 {}) scale(1 -1)\" fill=\"none\" stroke-width=\"0.25\">\n<rect width=\"{}\" height=\"{}\" fill=\"#fafafa\" stroke=\"#333333\"/>\n",
        s.canvas_length, s.height, s.canvas_length, s.height, s.width, s.length, s.width
    );
    for path in &s.paths {
        let tag = if path.closed { "polygon" } else { "polyline" };
        write!(out, "<{tag} stroke=\"{}\" points=\"", path.colour)?;
        for p in &path.points {
            write!(out, "{},{} ", p[0], p[1])?;
        }
        writeln!(out, "\"/>")?;
    }
    for p in &s.circles {
        writeln!(
            out,
            "<circle cx=\"{}\" cy=\"{}\" r=\"0.75\" stroke=\"#555555\"/>",
            p[0], p[1]
        )?;
    }
    out.push_str("</g>\n<g font-family=\"sans-serif\" font-size=\"3\" fill=\"#222222\">\n");
    for text in &s.texts {
        writeln!(
            out,
            "<text x=\"{}\" y=\"{}\">{}</text>",
            text.point[0],
            if text.footer {
                s.width + s.height - text.point[1] + 3.0
            } else {
                s.width - text.point[1]
            },
            escape(&text.value)
        )?;
    }
    out.push_str("</g>\n</svg>\n");
    Ok(out)
}
