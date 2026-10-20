//! Deterministic full-size A4 scenes for regression, not physical-camera validation.
use crate::{
    calibration::{Radial, sample},
    geometry::{Homography, fit_homography},
    synthetic::PhotoOptions,
};
use anyhow::Result;
use beta_marker_core::{Piece, Pt};
use image::{GrayImage, Luma};
pub struct Photo {
    pub image: GrayImage,
    pub truth: Piece,
    pub source_scale: f64,
    pub camera: Homography,
}
pub fn photograph(source: &Piece, options: &PhotoOptions, brown: bool) -> Result<Photo> {
    let bounds = beta_marker_core::geometry::bbox(&source.outline).unwrap();
    let scale = (330. / (bounds[2] - bounds[0])).min(420. / (bounds[3] - bounds[1]));
    let poly: Vec<Pt> = source
        .outline
        .iter()
        .map(|p| {
            [
                350. + (p[0] - bounds[0]) * scale,
                80. + (bounds[3] - p[1]) * scale,
            ]
        })
        .collect();
    let ppm = 4.;
    let mut board = GrayImage::from_fn(3200, 2400, |x, y| {
        let texture = if options.noise > 0. {
            5. * (x as f64 * 0.6).sin()
                + 4. * (y as f64 * 0.7).sin()
                + 3. * ((x + y) as f64 * 0.09).sin()
        } else {
            0.
        };
        Luma([(38. + texture).round() as u8])
    });
    for y in 320..1508 {
        for x in 120..960 {
            board[(x, y)] = Luma([242]);
        }
    }
    for y in 0..board.height() {
        let py = (y as f64 + 0.5) / ppm;
        let mut xs = Vec::new();
        for i in 0..poly.len() {
            let a = poly[i];
            let b = poly[(i + 1) % poly.len()];
            if (a[1] > py) != (b[1] > py) {
                xs.push(a[0] + (b[0] - a[0]) * (py - a[1]) / (b[1] - a[1]));
            }
        }
        xs.sort_by(f64::total_cmp);
        for pair in xs.as_chunks::<2>().0 {
            let lo = (pair[0] * ppm - 0.5).ceil().max(0.) as u32;
            let hi = (pair[1] * ppm - 0.5).ceil().min(board.width() as f64) as u32;
            for x in lo..hi {
                board[(x, y)] = Luma([if brown { 185 } else { 231 }]);
            }
        }
    }
    let [w, h] = options.dimensions;
    let quad = if options.perspective {
        [[0.05, 0.10], [0.93, 0.04], [0.97, 0.92], [0.08, 0.96]]
    } else {
        [[0.05, 0.05], [0.95, 0.05], [0.95, 0.95], [0.05, 0.95]]
    };
    let world = [[0., 0.], [800., 0.], [800., 600.], [0., 600.]];
    let camera = fit_homography(
        &world
            .into_iter()
            .zip(quad.map(|p| [p[0] * w as f64, p[1] * h as f64]))
            .collect::<Vec<_>>(),
    )?;
    let inv = camera.inverse()?;
    let radial = Radial {
        k1: options.k1,
        centre: [w as f64 / 2., h as f64 / 2.],
        scale: w.max(h) as f64 / 2.,
    };
    let mut rng = options.seed.max(1);
    let mut image = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let q = radial.undistort([x as f64 + 0.5, y as f64 + 0.5]);
            let value = inv
                .map(q)
                .and_then(|p| sample(&board, [p[0] * ppm, p[1] * ppm]))
                .unwrap_or(35.);
            let nx = x as f64 / w as f64;
            let ny = y as f64 / h as f64;
            let light = 1.
                - options.lighting
                    * (0.4 * nx + 0.3 * ny + 0.3 * (-((nx - 0.67) / 0.12).powi(2)).exp());
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let noise = ((rng & 65535) as f64 / 32767.5 - 1.) * options.noise;
            image[(x, y)] = Luma([(value * light + noise).round().clamp(0., 255.) as u8]);
        }
    }
    if options.blur_px > 0. {
        image = image::imageops::blur(&image, options.blur_px);
    }
    let truth = Piece {
        id: source.id.clone(),
        name: source.name.clone(),
        outline: poly.into_iter().map(|p| [p[0] - 30., 80. - p[1]]).collect(),
        quantity: 1,
        ..Default::default()
    };
    Ok(Photo {
        image,
        truth,
        source_scale: scale,
        camera,
    })
}
