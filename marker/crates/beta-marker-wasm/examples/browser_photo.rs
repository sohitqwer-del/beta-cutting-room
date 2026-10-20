//! Public synthetic A4 photograph for the offline phone-flow browser test.
//! No camera image or factory geometry is used.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("pass the output PNG path")?;
    let piece = beta_marker_core::Piece {
        id: "test-panel".into(),
        name: "Synthetic panel".into(),
        outline: vec![[0., 0.], [200., 0.], [200., 100.], [150., 180.], [0., 180.]],
        ..Default::default()
    };
    let options = beta_pattern_digitize::synthetic::PhotoOptions {
        dimensions: [1200, 900],
        perspective: false,
        k1: 0.,
        blur_px: 0.,
        noise: 0.,
        lighting: 0.,
        ..Default::default()
    };
    beta_pattern_digitize::quick_synthetic::photograph(&piece, &options, false)?
        .image
        .save(path)?;
    Ok(())
}
