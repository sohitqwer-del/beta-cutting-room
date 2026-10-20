//! Exercise factory-facing import choices and tolerance propagation through the CLI.
use std::{
    path::PathBuf,
    process::{Command, Output},
};

struct Scratch {
    path: PathBuf,
    root: PathBuf,
}
impl Drop for Scratch {
    fn drop(&mut self) {
        if let Ok(path) = self.path.canonicalize()
            && path.parent() == Some(self.root.as_path())
            && path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("seamly-cli-"))
        {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_beta-pattern"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn copy_rules_and_tolerance_work_through_the_cli() {
    let base = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let base = base.canonicalize().unwrap();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = Scratch {
        path: base.join(format!("seamly-cli-{}-{nonce}", std::process::id())),
        root: base,
    };
    std::fs::create_dir_all(&dir.path).unwrap();
    let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/seamly");
    let pattern = input.join("shirt-front-back.sm2d");
    let ms = input.join("original-multisize.smms");
    let rules = input.join("shirt-front-back.copy-rules.json");
    let doc = dir.path.join("doc.json");
    let output = run(&[
        "import-seamly",
        pattern.to_str().unwrap(),
        "--measurements",
        ms.to_str().unwrap(),
        "--copy-rules",
        rules.to_str().unwrap(),
        "-o",
        doc.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("BP-004"));
    assert!(!stderr.contains("mirrored-pair annotation"));
    let document: beta_pattern_core::Document =
        serde_json::from_slice(&std::fs::read(&doc).unwrap()).unwrap();
    assert_eq!(document.sizes, ["48", "50", "52"]);
    assert!(
        document
            .pieces
            .iter()
            .all(|p| (p.quantity, p.mirror_quantity) == (1, 1))
    );
    let mut counts = vec![];
    for (name, tolerance) in [("coarse", "0.1"), ("fine", "0.01")] {
        let dest = dir.path.join(name);
        let output = run(&[
            "grade",
            doc.to_str().unwrap(),
            "--tolerance",
            tolerance,
            "-o",
            dest.to_str().unwrap(),
        ]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("BP-004"));
        let pieces: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dest.join("50.pieces.json")).unwrap()).unwrap();
        counts.push(
            pieces
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["seam"].as_array().unwrap().len())
                .sum::<usize>(),
        );
    }
    assert!(counts[1] > counts[0]);
    for command in ["export-dxf", "export-plt", "export-pdf", "to-job"] {
        let dest = dir.path.join(format!("{command}.out"));
        let mut args = vec![
            command,
            doc.to_str().unwrap(),
            "--tolerance",
            "0.01",
            "-o",
            dest.to_str().unwrap(),
        ];
        if command == "to-job" {
            args.extend(["--width", "1500", "--ratio", "48=1,50=1,52=1"]);
        }
        let output = run(&args);
        assert!(
            output.status.success(),
            "{command}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("BP-004"));
        assert!(std::fs::metadata(dest).unwrap().len() > 0);
    }
    for tolerance in ["0", "-1", "NaN", "inf", "text"] {
        let dest = dir.path.join("invalid.dxf");
        let output = run(&[
            "export-dxf",
            doc.to_str().unwrap(),
            "--tolerance",
            tolerance,
            "-o",
            dest.to_str().unwrap(),
        ]);
        assert!(!output.status.success());
        assert!(!dest.exists());
    }
    let dest = dir.path.join("individual.json");
    let individual = input.join("original-size-50.smis");
    let output = run(&[
        "import-seamly",
        pattern.to_str().unwrap(),
        "--measurements",
        individual.to_str().unwrap(),
        "--copy-rules",
        rules.to_str().unwrap(),
        "-o",
        dest.to_str().unwrap(),
    ]);
    assert!(output.status.success());
    let document: beta_pattern_core::Document =
        serde_json::from_slice(&std::fs::read(dest).unwrap()).unwrap();
    assert_eq!(document.sizes, ["M"]);
}
