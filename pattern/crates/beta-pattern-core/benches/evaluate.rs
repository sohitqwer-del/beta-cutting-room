//! Repeatable 30-piece / 1,000-object / six-size release benchmark. No extra dependency.
use beta_pattern_core::{evaluate, tools, *};
use std::{collections::BTreeMap, hint::black_box, time::Instant};
fn fixture() -> Document {
    let mut d = Document {
        sizes: ["XS", "S", "M", "L", "XL", "XXL"]
            .map(str::to_owned)
            .to_vec(),
        ..Document::default()
    };
    for (i, size) in d.sizes.iter().enumerate() {
        d.measurements.insert(
            size.clone(),
            BTreeMap::from([("growth".into(), i as f64 * 4.)]),
        );
    }
    for i in 0..30 {
        let id = format!("panel{i}");
        let x = i as f64 * 220.;
        tools::insert_polygon(
            &mut d,
            &id,
            &id,
            &[[x, 0.], [x + 180., 0.], [x + 180., 200.], [x, 200.]],
        )
        .unwrap();
        for (name, dx) in [("c1", 150.), ("c2", 30.)] {
            d.drawing.insert(
                format!("{id}.{name}"),
                Object::Point {
                    operation: PointOp::Absolute {
                        x: format!("{x}+{dx}+m.growth").as_str().into(),
                        y: 210.0.into(),
                    },
                },
            );
        }
        d.drawing.insert(
            format!("{id}.e2"),
            Object::Cubic {
                points: [
                    format!("{id}.p2"),
                    format!("{id}.c1"),
                    format!("{id}.c2"),
                    format!("{id}.p3"),
                ],
            },
        );
        for edge in &mut d.pieces[i].edges {
            edge.allowance = 10.0.into();
        }
    }
    let extra = 1000 - d.drawing.len();
    for i in 0..extra {
        d.drawing.insert(
            format!("measurement_point{i}"),
            Object::Point {
                operation: PointOp::Absolute {
                    x: format!("m.growth+{i}").as_str().into(),
                    y: (i as f64).into(),
                },
            },
        );
    }
    assert_eq!(d.drawing.len(), 1000);
    assert_eq!(d.pieces.len(), 30);
    d
}
fn main() {
    let d = fixture();
    let mut durations = vec![];
    for run in 0..12 {
        let begin = Instant::now();
        let mut count = 0;
        for size in &d.sizes {
            let result = evaluate::evaluate_pattern(black_box(&d), size, 0.1).unwrap();
            assert!(result.objects.iter().all(|o| o.error.is_none()));
            assert!(result.piece_errors.is_empty());
            count += result.pieces.len();
            black_box(result);
        }
        assert_eq!(count, 180);
        let ms = begin.elapsed().as_secs_f64() * 1000.;
        if run > 1 {
            durations.push(ms);
        }
    }
    durations.sort_by(f64::total_cmp);
    let median = durations[durations.len() / 2];
    println!(
        "30 pieces, 1000 objects (30 cubics), all 6 sizes, 0.1 mm tolerance: median {median:.3} ms; min {:.3}; max {:.3}; 10 timed runs after 2 warmups",
        durations[0],
        durations[durations.len() - 1]
    );
    assert!(median < 100., "release evaluation exceeds 100 ms");
}
