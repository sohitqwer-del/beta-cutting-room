//! BETA Cutting Room native shell. All manufacturing operations call Rust crates.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod api;
mod pipeline;
mod storage;
mod worker;
#[cfg(test)]
mod opener_tests;
use api::{Method, Reply, Route};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

struct Desktop {
    root: PathBuf,
    child: Mutex<Option<Child>>,
    busy: AtomicBool,
    status: Mutex<Value>,
    recent: Mutex<Vec<String>>,
    pending: Mutex<Vec<String>>,
    pending_project: Mutex<Option<Value>>,
    settings: Mutex<storage::Settings>,
    pdfs: Mutex<BTreeSet<PathBuf>>,
    started: Instant,
    smoke: Option<PathBuf>,
}
fn lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|e| e.into_inner())
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn recent(state: &Desktop, path: &Path) {
    let name = path.to_string_lossy().into_owned();
    let mut items = lock(&state.recent);
    items.retain(|p| p != &name);
    items.insert(0, name);
    items.truncate(15);
    let _ = storage::atomic_write(
        &state.root.join("recent.json"),
        json!(*items).to_string().as_bytes(),
    );
}
#[tauri::command]
async fn desktop_request(
    app: tauri::AppHandle,
    state: State<'_, Arc<Desktop>>,
    method: String,
    path: String,
    body: Vec<u8>,
) -> Result<Reply, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || request(&app, &state, &method, &path, &body))
        .await
        .map_err(err)?
}
fn request(
    app: &tauri::AppHandle,
    state: &Arc<Desktop>,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<Reply, String> {
    if path == "/api/nest/cancel" {
        return Ok(if method == "POST" { cancel(state) } else { Reply::error(405,"Use POST to cancel") });
    }
    if method == "GET" && path == "/api/nest/status" {
        return Ok(Reply::json(lock(&state.status).clone()));
    }
    let method = match method {
        "GET" => Method::Get,
        "POST" => Method::Post,
        _ => return Ok(Reply::error(405, "Use GET or POST")),
    };
    if ["/api/pipeline", "/api/plan/pipeline", "/api/plan/cost"].contains(&path) {
        if method != Method::Post {return Ok(Reply::error(405,"Use POST for pipeline"));}
        if body.len() > 50 * 1024 * 1024 {
            return Ok(Reply::error(413, "Request exceeds 50 MB"));
        }
        return search(app, state, path, body);
    }
    match api::route(&method, path, body) {
        Route::Reply(mut reply) => {
            if reply.status == 200 && path.starts_with("/api/export?format=") {
                let input: Value = serde_json::from_slice(body).map_err(err)?;
                let settings = lock(&state.settings).clone();
                if path == "/api/export?format=iso" {
                    let job = serde_json::from_value(input["job"].clone()).map_err(err)?;
                    let result = serde_json::from_value(input["result"].clone()).map_err(err)?;
                    reply.body = beta_marker_export::to_iso(&job, &result, &settings.iso_profile)
                        .map_err(err)?
                        .into_bytes();
                }
            }
            Ok(reply)
        }
        Route::Status => Ok(Reply::json(lock(&state.status).clone())),
        Route::Nest(_) | Route::Plan(_) => search(app, state, path, body),
    }
}
fn scale_hpgl(bytes: &[u8], units: f64) -> anyhow::Result<Vec<u8>> {
    let source = std::str::from_utf8(bytes)?;
    anyhow::ensure!(
        units.is_finite() && units > 0.0,
        "Plotter units must be positive"
    );
    if units == 40.0 {
        return Ok(bytes.to_vec());
    }
    let mut output = String::new();
    let mut remaining = source;
    while !remaining.is_empty() {
        let clean = remaining.trim_start();
        output.push_str(&remaining[..remaining.len() - clean.len()]);
        if clean.is_empty() {
            break;
        }
        // Labels end at ETX, not at semicolons inside user-visible text.
        let end = if clean.starts_with("LB") {
            clean
                .find('\u{3}')
                .map(|index| index + 1)
                .unwrap_or(clean.len())
        } else {
            clean
                .find(';')
                .map(|index| index + 1)
                .unwrap_or(clean.len())
        };
        let token = &clean[..end];
        remaining = &clean[end..];
        if matches!(token.get(..2), Some("PU" | "PD" | "PA")) {
            output.push_str(&token[..2]);
            let coordinates = token[2..].trim_end_matches(';');
            if !coordinates.is_empty() {
                let values = coordinates
                    .split(',')
                    .map(|number| -> anyhow::Result<String> {
                        let value = number.trim().parse::<f64>()? * units / 40.0;
                        anyhow::ensure!(
                            value.is_finite() && value.abs() <= i32::MAX as f64,
                            "Scaled coordinates exceed HPGL range"
                        );
                        Ok(format!("{value:.0}"))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                output.push_str(&values.join(","));
            }
            if token.ends_with(';') {
                output.push(';');
            }
        } else {
            output.push_str(token);
        }
    }
    Ok(output.into_bytes())
}

struct SearchGuard<'a> {
    state: &'a Desktop,
    app: &'a tauri::AppHandle,
    started: Instant,
}
impl Drop for SearchGuard<'_> {
    fn drop(&mut self) {
        if let Some(mut child) = lock(&self.state.child).take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let mut status = lock(&self.state.status);
        status["running"] = json!(false);
        status["elapsed_s"] = json!(self.started.elapsed().as_secs_f64());
        let _ = self.app.emit("nest-progress", &*status);
        self.state.busy.store(false, Ordering::Release);
    }
}
fn search(
    app: &tauri::AppHandle,
    state: &Arc<Desktop>,
    path: &str,
    body: &[u8],
) -> Result<Reply, String> {
    if state
        .busy
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(Reply::error(409, "A search is already running"));
    }
    let guard = SearchGuard {
        state,
        app,
        started: Instant::now(),
    };
    *lock(&state.status) = json!({"running":true,"elapsed_s":0.0,"best_length_mm":null,"best_efficiency":null,"cancelled":false});
    let mut input: Value = serde_json::from_slice(body).map_err(err)?;
    if path == "/api/nest" {
        input["options"]["workers"] = json!(lock(&state.settings).worker_threads);
    }
    let mut cmd = Command::new(std::env::current_exe().map_err(err)?);
    cmd.arg("--beta-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd.spawn().map_err(err)?;
    #[cfg(windows)]
    let _process_job = match worker::ProcessJob::attach(&child) {
        Ok(job) => job,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(err(error));
        }
    };
    let mut stdin = child.stdin.take().ok_or("Worker stdin is unavailable")?;
    let stdout = child.stdout.take().ok_or("Worker stdout is unavailable")?;
    *lock(&state.child) = Some(child);
    writeln!(stdin, "{}", json!({"path":path,"body":input})).map_err(err)?;
    drop(stdin);
    let finished = Arc::new(AtomicBool::new(false));
    let ticker = Arc::clone(&finished);
    let ticker_state = Arc::clone(state);
    let handle = app.clone();
    let tick = std::thread::spawn(move || {
        let started = Instant::now();
        while !ticker.load(Ordering::Acquire) {
            std::thread::sleep(std::time::Duration::from_millis(200));
            let mut status = lock(&ticker_state.status);
            status["elapsed_s"] = json!(started.elapsed().as_secs_f64());
            let _ = handle.emit("nest-progress", &*status);
        }
    });
    let mut reply = None;
    for line in BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(p) = value.get("progress") {
            let mut status = lock(&state.status);
            if status["best_length_mm"]
                .as_f64()
                .is_none_or(|best| p["best_length_mm"].as_f64().is_some_and(|n| n < best))
            {
                status["best_length_mm"] = p["best_length_mm"].clone();
                status["best_efficiency"] = p["best_efficiency"].clone();
                status["result"] = p["result"].clone();
            }
        }
        if let Some(r) = value.get("reply") {
            reply = serde_json::from_value::<Reply>(r.clone()).ok();
        }
    }
    finished.store(true, Ordering::Release);
    let _ = tick.join();
    let cancelled = lock(&state.status)["cancelled"] == true;
    if let Some(r) = &reply
        && r.status == 200
        && path == "/api/nest"
        && let Ok(result) = serde_json::from_slice::<Value>(&r.body)
    {
        let mut status = lock(&state.status);
        status["best_length_mm"] = result["length"].clone();
        status["best_efficiency"] = result["efficiency"].clone();
        status["result"] = result;
    }
    drop(guard);
    Ok(if cancelled {
        Reply::error(499, "Nesting cancelled. The worker has stopped.")
    } else {
        reply.unwrap_or_else(|| Reply::error(500, "Search worker stopped unexpectedly"))
    })
}
fn cancel(state: &Desktop) -> Reply {
    let mut child = lock(&state.child);
    if let Some(child) = child.as_mut() {
        lock(&state.status)["cancelled"] = json!(true);
        if let Err(e) = child.kill() {
            return Reply::error(500, &e.to_string());
        }
        let _ = child.wait();
    }
    Reply::json(json!({"cancelled":state.busy.load(Ordering::Acquire)}))
}
#[tauri::command]
fn desktop_bootstrap(state: State<'_, Arc<Desktop>>) -> Value { bootstrap(&state) }
fn bootstrap(state: &Desktop) -> Value {
    let project = lock(&state.pending_project).take();
    let recovery = if project.is_some() { None } else {
        storage::read(&state.root.join("recovery.json")).or_else(|_| storage::read(&state.root.join("recovery.beta-backup"))).ok()
    };
    json!({"settings":*lock(&state.settings),"recent":*lock(&state.recent),"pending":std::mem::take(&mut *lock(&state.pending)),"project":project,"recovery":recovery,"smoke":state.smoke.is_some(),"rust_start_ms":state.started.elapsed().as_secs_f64()*1000.0})
}
fn stage_project(state: &Desktop, mut value: Value) -> Result<(), String> {
    if state.busy.load(Ordering::Acquire) {return Err("Wait for the current search.".into());}
    storage::validate_project(&value).map_err(err)?;
    let settings = if let Some(settings) = value.get("settings") {
        serde_json::from_value::<storage::Settings>(settings.clone()).map_err(err)?
    } else { lock(&state.settings).clone() };
    settings.validate().map_err(err)?;
    value["settings"] = serde_json::to_value(&settings).map_err(err)?;
    let bytes = serde_json::to_vec(&value).map_err(err)?;
    if bytes.len()>50*1024*1024 {return Err("Project exceeds 50 MB".into());}
    storage::atomic_write(&state.root.join("recovery.json"), &bytes).map_err(err)?;
    storage::atomic_write(&state.root.join("settings.json"), &serde_json::to_vec(&settings).map_err(err)?).map_err(err)?;
    *lock(&state.settings)=settings;
    *lock(&state.pending_project)=Some(value);
    Ok(())
}
#[tauri::command]
async fn desktop_stage_project(state: State<'_, Arc<Desktop>>, value: Value) -> Result<(), String> {
    let state=Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || stage_project(&state,value)).await.map_err(err)?
}

#[tauri::command]
async fn desktop_open(
    app: tauri::AppHandle,
    state: State<'_, Arc<Desktop>>,
    path: Option<String>,
    project: bool,
) -> Result<Option<Value>, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || {
        let path = if let Some(path) = path {
            PathBuf::from(path)
        } else {
            let dialog = app.dialog().file();
            let selected = if project {
                dialog
                    .set_title("Open .betaproj folder")
                    .blocking_pick_folder()
            } else {
                dialog
                    .add_filter(
                        "BETA files",
                        &["dxf", "plt", "betajob", "betamarker", "betapattern", "json"],
                    )
                    .blocking_pick_file()
            };
            let Some(selected) = selected else {
                return Ok(None);
            };
            selected.into_path().map_err(err)?
        };
        if path.to_string_lossy().starts_with("\\\\") {
            return Err("Network paths are disabled".into());
        }
        if path.is_dir() {
            let value = storage::read(&storage::project_file(&path))
                .or_else(|_| storage::read(&path.join("project.beta-backup")))
                .map_err(err)?;
            storage::validate_project(&value).map_err(err)?;
            recent(&state, &path);
            return Ok(Some(json!({"kind":"project","path":path,"value":value})));
        }
        if path.metadata().map_err(err)?.len() > 50 * 1024 * 1024 {
            return Err("File exceeds 50 MB".into());
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !["dxf", "plt", "betajob", "betamarker", "betapattern", "json"].contains(&ext.as_str()) {
            return Err("Unsupported BETA file type".into());
        }
        let bytes = std::fs::read(&path).map_err(err)?;
        recent(&state, &path);
        Ok(Some(
            json!({"kind":"file","path":path.to_string_lossy(),"name":path.file_name().map(|name|name.to_string_lossy()),"bytes":bytes}),
        ))
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn desktop_save(
    app: tauri::AppHandle,
    state: State<'_, Arc<Desktop>>,
    name: String,
    mut bytes: Vec<u8>,
) -> Result<Option<String>, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || {
        if bytes.len() > 50 * 1024 * 1024 {
            return Err("File exceeds 50 MB".into());
        }
        let name = Path::new(&name)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid filename")?
            .to_string();
        let path = if let Some(folder) = &state.smoke {
            folder.join(&name)
        } else {
            let Some(path) = app
                .dialog()
                .file()
                .set_file_name(&name)
                .blocking_save_file()
            else {
                return Ok(None);
            };
            path.into_path().map_err(err)?
        };
        if path.to_string_lossy().starts_with("\\\\") {
            return Err("Network paths are disabled".into());
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("plt"))
        {
            bytes = scale_hpgl(&bytes, lock(&state.settings).plotter_units_per_mm).map_err(err)?;
        }
        storage::atomic_write(&path, &bytes).map_err(err)?;
        if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")) {
            lock(&state.pdfs).insert(path.clone());
        }
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn desktop_project_save(
    app: tauri::AppHandle,
    state: State<'_, Arc<Desktop>>,
    value: Value,
) -> Result<Option<String>, String> {
    let state = Arc::clone(&state);
    tauri::async_runtime::spawn_blocking(move || {
        let Some(folder) = app
            .dialog()
            .file()
            .set_title("Choose parent folder for Cutting Room.betaproj")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        let path = folder
            .into_path()
            .map_err(err)?
            .join("Cutting Room.betaproj");
        if path.to_string_lossy().starts_with("\\\\") {
            return Err("Network paths are disabled".into());
        }
        storage::save_project(&path, &value).map_err(err)?;
        recent(&state, &path);
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(err)?
}
#[tauri::command]
async fn desktop_autosave(state: State<'_, Arc<Desktop>>, value: Value) -> Result<(), String> {
    let root = state.root.clone();
    tauri::async_runtime::spawn_blocking(move || {
        storage::validate_project(&value).map_err(err)?;
        let bytes = serde_json::to_vec(&value).map_err(err)?;
        if bytes.len() > 50 * 1024 * 1024 {
            return Err("Project exceeds 50 MB".into());
        };
        storage::atomic_write(&root.join("recovery.json"), &bytes).map_err(err)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
fn desktop_settings(
    state: State<'_, Arc<Desktop>>,
    settings: storage::Settings,
) -> Result<(), String> {
    settings.validate().map_err(err)?;
    storage::atomic_write(
        &state.root.join("settings.json"),
        &serde_json::to_vec_pretty(&settings).map_err(err)?,
    )
    .map_err(err)?;
    *lock(&state.settings) = settings;
    Ok(())
}
#[tauri::command]
fn desktop_view_pdf(state: State<'_, Arc<Desktop>>, path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !lock(&state.pdfs).contains(&path) {
        return Err("Save a PDF in this session first".into());
    }
    // File association launch, never a shell command or arbitrary URI.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        Command::new("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg(&path)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(err)?;
    }
    Ok(())
}
#[tauri::command]
fn desktop_smoke_finish(
    app: tauri::AppHandle,
    state: State<'_, Arc<Desktop>>,
    result: Value,
) -> Result<(), String> {
    let folder = state.smoke.as_ref().ok_or("Smoke mode is disabled")?;
    storage::atomic_write(
        &folder.join("smoke-result.json"),
        &serde_json::to_vec_pretty(&result).map_err(err)?,
    )
    .map_err(err)?;
    app.exit(if result["ok"] == true { 0 } else { 1 });
    Ok(())
}
fn main() {
    let started = Instant::now();
    if std::env::args().any(|a| a == "--beta-worker") {
        if let Err(e) = worker::run() {
            eprintln!("{e:#}");
            std::process::exit(1)
        }
        return;
    }
    let pending: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app,args,_cwd| {
            let paths:Vec<_>=args.into_iter().skip(1).filter(|a|!a.starts_with('-')).collect();
            let _=app.emit("desktop-open",paths);
            if let Some(window)=app.get_webview_window("main") {let _=window.unminimize();let _=window.show();let _=window.set_focus();}
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::Builder::new().open_js_links_on_click(false).build())
        .setup(move |app| {
            let root=std::env::var_os("BETA_DATA_DIR").map(PathBuf::from).map(|p|p.join("desktop")).unwrap_or(app.path().app_local_data_dir()?);
            std::fs::create_dir_all(&root)?;
            let webview_data = root.join("webview");
            let settings=storage::read(&root.join("settings.json")).ok().and_then(|v|serde_json::from_value::<storage::Settings>(v).ok()).filter(|s|s.validate().is_ok()).unwrap_or_default();
            let recent=storage::read(&root.join("recent.json")).ok().and_then(|v|serde_json::from_value(v).ok()).unwrap_or_default();
            app.manage(Arc::new(Desktop {root,child:Mutex::new(None),busy:AtomicBool::new(false),status:Mutex::new(json!({"running":false,"elapsed_s":0.0,"best_length_mm":null,"best_efficiency":null})),recent:Mutex::new(recent),pending:Mutex::new(pending),pending_project:Mutex::new(None),settings:Mutex::new(settings),pdfs:Mutex::new(BTreeSet::new()),started,smoke:std::env::var_os("BETA_DESKTOP_SMOKE_DIR").map(PathBuf::from)}));
            let config = app.config().app.windows.first().ok_or("Missing main window configuration")?;
            tauri::WebviewWindowBuilder::from_config(app, config)?
                .data_directory(webview_data)
                .on_navigation(|url| url.scheme() == "tauri" || (url.scheme() == "http" && url.host_str() == Some("tauri.localhost")) || (cfg!(debug_assertions) && url.host_str() == Some("localhost")))
                .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![desktop_request,desktop_bootstrap,desktop_stage_project,desktop_open,desktop_save,desktop_project_save,desktop_autosave,desktop_settings,desktop_view_pdf,desktop_smoke_finish])
        .build(tauri::generate_context!()).expect("BETA desktop initialization failed")
        .run(|app,event| {if let tauri::RunEvent::Exit=event { let state=app.state::<Arc<Desktop>>(); let _=cancel(&state); }});
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plotter_scaling_changes_coordinates_only() {
        assert_eq!(
            scale_hpgl(b"IN;SP1;PU400,800;PD0,-400;SP0;", 20.0).unwrap(),
            b"IN;SP1;PU200,400;PD0,-200;SP0;"
        );
    }
    #[test]
    fn plotter_scaling_preserves_labels_and_rejects_overflow() {
        assert_eq!(
            scale_hpgl(b"PU40,80;LBx;PD400;\x03;PD80,40;", 20.0).unwrap(),
            b"PU20,40;LBx;PD400;\x03;PD40,20;"
        );
        assert!(scale_hpgl(b"PU9999999999999,0;", 80.0).is_err());
        assert!(scale_hpgl("\u{1f600};".as_bytes(), 20.0).is_ok());
    }
    #[test]
    fn command_functions_match_current_api() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../packages/marker-wasm/test/fixtures/cutting-room.json"
        ))
        .unwrap();
        for path in [
            "/api/validate",
            "/api/export?format=svg",
            "/api/export?format=plt",
            "/api/export?format=pdf",
            "/api/export?format=iso",
            "/api/export?format=dxf",
            "/api/export?format=xlsx",
            "/api/export?format=labels",
            "/api/export?format=mini",
            "/api/export?format=report",
        ] {
            match api::route(&Method::Post, path, fixture.to_string().as_bytes()) {
                Route::Reply(r) => assert_eq!(r.status, 200, "{path}: {:?}", r.body),
                _ => panic!("Expected reply"),
            }
        }
        for path in [
            "/api/pattern/starter",
            "/api/pattern/catalog",
            "/api/digitize/target",
        ] {
            match api::route(&Method::Post, path, b"{}") {
                Route::Reply(r) => assert_eq!(r.status, 200),
                _ => panic!("Expected reply"),
            }
        }
        match api::route(&Method::Post, "/api/nest", b"{}") {
            Route::Reply(r) => assert_eq!(r.status, 400),
            _ => panic!("Expected error"),
        }
    }
    #[cfg(windows)]
    #[test]
    fn cancel_command_stops_and_reaps_its_owned_background_process() {
        use std::os::windows::process::CommandExt;
        let mut child=Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-Command","Start-Sleep -Seconds 3"]).creation_flags(0x08000000).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().expect("background test process");
        let job=match worker::ProcessJob::attach(&child) {Ok(job)=>job,Err(error)=>{let _=child.kill();let _=child.wait();panic!("Windows job attachment: {error}");}};
        let state=Desktop {root:PathBuf::new(),child:Mutex::new(Some(child)),busy:AtomicBool::new(true),status:Mutex::new(json!({"running":true})),recent:Mutex::new(Vec::new()),pending:Mutex::new(Vec::new()),pending_project:Mutex::new(None),settings:Mutex::new(Default::default()),pdfs:Mutex::new(BTreeSet::new()),started:Instant::now(),smoke:None};
        let reply=cancel(&state);assert_eq!(reply.status,200);assert_eq!(lock(&state.status)["cancelled"],true);
        assert!(lock(&state.child).as_mut().unwrap().try_wait().unwrap().is_some());
        drop(job);
    }

    #[test]
    fn project_staging_survives_a_renderer_reload_without_session_storage() {
        let fixture:Value=serde_json::from_str(include_str!("../../../../packages/marker-wasm/test/fixtures/cutting-room.json")).unwrap();
        let root=PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").expect("assigned Cargo output")).join(format!("desktop-stage-test-{}",std::process::id()));
        let state=Desktop {root:root.clone(),child:Mutex::new(None),busy:AtomicBool::new(false),status:Mutex::new(json!({"running":false})),recent:Mutex::new(Vec::new()),pending:Mutex::new(Vec::new()),pending_project:Mutex::new(None),settings:Mutex::new(Default::default()),pdfs:Mutex::new(BTreeSet::new()),started:Instant::now(),smoke:None};
        let project=json!({"format":"beta-cutting-room.project/v1","editor":{"job":fixture["job"],"result":fixture["result"]},"storage":{},"settings":storage::Settings::default(),"planning":{"cuttingRoom":{"large_test_note":"x".repeat(6*1024*1024)}}});
        stage_project(&state,project.clone()).expect("stage large project");
        assert_eq!(bootstrap(&state)["project"],project);
        assert!(bootstrap(&state)["project"].is_null());
        assert_eq!(storage::read(&root.join("recovery.json")).unwrap(),project);
        std::fs::remove_file(root.join("recovery.json")).unwrap();std::fs::remove_file(root.join("settings.json")).unwrap();std::fs::remove_dir(root).unwrap();
    }

}
