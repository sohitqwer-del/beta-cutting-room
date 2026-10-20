//! Convert a public synthetic grayscale photo to phone-like RGB JPEG for memory profiling.
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let rgb = image::open(args.get(1).expect("input image"))?.into_rgb8();
    rgb.save(args.get(2).expect("output JPEG"))?;
    Ok(())
}
