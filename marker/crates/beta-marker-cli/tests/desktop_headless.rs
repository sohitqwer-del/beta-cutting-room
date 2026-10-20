//! Run the desktop's real storage and pipeline regressions without Tauri,
//! WebView2, an installer, or a desktop window. Keep these modules unchanged.
//!
//! This covers project validation, atomic saves/backups, settings and a complete
//! small native pipeline. It does not establish desktop IPC/installer acceptance.

#[allow(dead_code)]
#[path = "../../../apps/desktop/src-tauri/src/storage.rs"]
mod storage;

#[allow(dead_code)]
#[path = "../../../apps/desktop/src-tauri/src/pipeline.rs"]
mod pipeline;
