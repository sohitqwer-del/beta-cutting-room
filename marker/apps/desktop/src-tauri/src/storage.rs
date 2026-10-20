//! Local project and recovery storage. No archive extraction or network paths.
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Settings {
    pub fabric_width_mm: f64,
    pub units: String,
    pub plotter_units_per_mm: f64,
    pub iso_profile: beta_marker_export::IsoProfile,
    pub worker_threads: usize,
    pub language: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            fabric_width_mm: 1500.0,
            units: "mm".into(),
            plotter_units_per_mm: 40.0,
            iso_profile: Default::default(),
            worker_threads: 2,
            language: "en".into(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            self.fabric_width_mm.is_finite() && self.fabric_width_mm > 0.0,
            "Fabric width must be positive"
        );
        ensure!(
            ["mm", "cm", "inch"].contains(&self.units.as_str()),
            "Choose mm, cm or inch"
        );
        ensure!(
            (0.01..=10000.0).contains(&self.plotter_units_per_mm),
            "Plotter units must be between 0.01 and 10000 per mm"
        );
        ensure!(
            (1..=64).contains(&self.worker_threads),
            "Choose 1 to 64 worker threads"
        );
        ensure!(self.language == "en", "Only English is available yet");
        Ok(())
    }
}
pub(crate) fn read(path: &Path) -> anyhow::Result<Value> {
    ensure!(path.is_file(), "File does not exist");
    ensure!(
        path.metadata()?.len() <= 50 * 1024 * 1024,
        "File exceeds 50 MB"
    );
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("beta-tmp");
    let backup = path.with_extension("beta-backup");
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    if path.exists() {
        std::fs::copy(path, &backup)?;
    }
    // Windows rename cannot replace an existing destination. The durable backup
    // survives an interrupted replacement, and is read by recovery on next boot.
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    std::fs::rename(&temp, path)?;
    Ok(())
}
pub(crate) fn project_file(folder: &Path) -> PathBuf {
    folder.join("project.json")
}
pub(crate) fn save_project(folder: &Path, value: &Value) -> anyhow::Result<()> {
    ensure!(
        folder.extension().is_some_and(|e| e == "betaproj"),
        "Choose a .betaproj folder"
    );
    validate_project(value)?;
    let bytes = serde_json::to_vec(value)?;
    ensure!(bytes.len() <= 50 * 1024 * 1024, "Project exceeds 50 MB");
    atomic_write(&project_file(folder), &bytes)
}
pub(crate) fn validate_project(value: &Value) -> anyhow::Result<()> {
    ensure!(
        value["format"] == "beta-cutting-room.project/v1",
        "Unsupported project format"
    );
    serde_json::from_value::<beta_marker_core::Job>(value["editor"]["job"].clone())
        .context("Project job")?;
    serde_json::from_value::<beta_marker_core::MarkerResult>(value["editor"]["result"].clone())
        .context("Project marker")?;
    ensure!(value["storage"].is_object(), "Project storage is missing");
    if let Some(settings) = value.get("settings") {
        serde_json::from_value::<Settings>(settings.clone())?.validate()?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn scratch() -> PathBuf {
        std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("desktop-tests")
    }
    #[test]
    fn settings_reject_invalid_units_and_threads() {
        let mut s = Settings::default();
        assert!(s.validate().is_ok());
        s.units = "feet".into();
        assert!(s.validate().is_err());
        s.units = "mm".into();
        s.worker_threads = 0;
        assert!(s.validate().is_err());
    }
    #[test]
    fn project_rejects_missing_state() {
        assert!(
            validate_project(&serde_json::json!({"format":"beta-cutting-room.project/v1"}))
                .is_err()
        );
    }
    #[test]
    fn atomic_save_keeps_last_good_copy() {
        let folder = scratch().join(format!("beta-desktop-test-{}", std::process::id()));
        let path = folder.join("state.json");
        atomic_write(&path, b"1").unwrap();
        atomic_write(&path, b"2").unwrap();
        assert_eq!(
            std::fs::read(path.with_extension("beta-backup")).unwrap(),
            b"1"
        );
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(path.with_extension("beta-backup")).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
    #[test]
    fn project_roundtrip_and_previous_copy_keep_the_marker_and_settings() {
        let fixture:Value=serde_json::from_str(include_str!("../../../../packages/marker-wasm/test/fixtures/cutting-room.json")).unwrap();
        let folder=scratch().join(format!("beta-project-test-{}.betaproj",std::process::id()));
        let mut project=serde_json::json!({"format":"beta-cutting-room.project/v1","editor":{"job":fixture["job"],"result":fixture["result"]},"storage":{"beta-cutting-room-v1":"{}"},"settings":Settings::default()});
        save_project(&folder,&project).unwrap();
        assert_eq!(read(&project_file(&folder)).unwrap(),project);
        project["settings"]["units"]=serde_json::json!("inch");save_project(&folder,&project).unwrap();
        assert_eq!(read(&folder.join("project.beta-backup")).unwrap()["settings"]["units"],"mm");
        assert_eq!(read(&project_file(&folder)).unwrap()["settings"]["units"],"inch");
        std::fs::remove_file(project_file(&folder)).unwrap();std::fs::remove_file(folder.join("project.beta-backup")).unwrap();std::fs::remove_dir(folder).unwrap();
    }

}
