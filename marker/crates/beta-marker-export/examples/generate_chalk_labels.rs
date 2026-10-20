//! Compile fixed Hindi/Tamil labels to vector paths. No font/shaper is loaded at runtime.
use rustybuzz::{
    Face, UnicodeBuffer,
    ttf_parser::{GlyphId, OutlineBuilder},
};
use std::fmt::Write;
#[derive(Default)]
struct Outline {
    path: String,
    current: [f32; 2],
}
impl OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        writeln!(self.path, "{x} {y} m").unwrap();
        self.current = [x, y];
    }
    fn line_to(&mut self, x: f32, y: f32) {
        writeln!(self.path, "{x} {y} l").unwrap();
        self.current = [x, y];
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let [a, b] = self.current;
        self.curve_to(
            a + (x1 - a) * 2. / 3.,
            b + (y1 - b) * 2. / 3.,
            x + (x1 - x) * 2. / 3.,
            y + (y1 - y) * 2. / 3.,
            x,
            y,
        );
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        writeln!(self.path, "{x1} {y1} {x2} {y2} {x} {y} c").unwrap();
        self.current = [x, y];
    }
    fn close(&mut self) {
        self.path.push_str("h\n");
    }
}
fn main() -> anyhow::Result<()> {
    let mut labels = serde_json::Map::new();
    let hi = include_bytes!("../fonts/DroidSansDevanagari-Regular.ttf").as_slice();
    let ta = include_bytes!("../fonts/DroidSansTamil-Regular.ttf").as_slice();
    for (lang, font, strings) in [
        (
            "hi",
            hi,
            [
                "चाक गाइड",
                "टुकड़ा",
                "आकार",
                "घुमाव",
                "पलटें",
                "शुरू से दूरी",
                "किनारे से दूरी",
                "मिमी",
            ],
        ),
        (
            "ta",
            ta,
            [
                "சுண்ணாம்பு வழிகாட்டி",
                "துண்டு",
                "அளவு",
                "சுழற்சி",
                "புரட்டு",
                "தொடக்கம்",
                "ஓரம்",
                "மிமீ",
            ],
        ),
    ] {
        let face = Face::from_slice(font, 0).unwrap();
        for (key, text) in ["title", "piece", "size", "rotation", "flip", "x", "y", "mm"]
            .iter()
            .zip(strings)
        {
            let mut buffer = UnicodeBuffer::new();
            buffer.push_str(text);
            buffer.guess_segment_properties();
            let shaped = rustybuzz::shape(&face, &[], buffer);
            let mut path = String::new();
            let mut advance = 0_f32;
            for (glyph, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                anyhow::ensure!(glyph.glyph_id != 0, "Missing glyph in {text}");
                let mut outline = Outline::default();
                face.outline_glyph(GlyphId(glyph.glyph_id as u16), &mut outline);
                writeln!(
                    path,
                    "q 1 0 0 1 {} {} cm\n{}f Q",
                    advance + pos.x_offset as f32,
                    pos.y_offset,
                    outline.path
                )?;
                advance += pos.x_advance as f32;
            }
            labels.insert(format!("{lang}.{key}"),serde_json::json!({"text":text,"units":face.units_per_em(),"width":advance,"path":path}));
        }
    }
    let output = std::env::args().nth(1).expect("output JSON path");
    std::fs::write(output, serde_json::to_string(&labels)?)?;
    Ok(())
}
