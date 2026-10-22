//! Offline HTTP API and static web server. Network access is opt-in and authenticated.
// Pure adapters are shared with WASM so offline and HTTP editing use identical checks.
#[path = "../../../../../pattern/crates/beta-pattern-wasm/src/editor.rs"]
mod pattern_editor;
#[path = "../../../../../pattern/crates/beta-pattern-wasm/src/operations.rs"]
mod pattern_operations;
#[path = "../../../../../pattern/crates/beta-pattern-wasm/src/output.rs"]
mod pattern_output;
#[path = "../../../../../pattern/crates/beta-pattern-wasm/src/style.rs"]
mod pattern_style;

use anyhow::Context;
use beta_marker_core::{Job, MarkerResult};
use beta_marker_dxf::{ImportOptions, Units};
use beta_marker_export::ExportOptions;
use beta_marker_nest::NestOptions;
use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::{
    Method, Request, Response, StatusCode,
    body::{Bytes, Incoming},
    header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::{TokioIo, TokioTimer};
use serde::Deserialize;
use serde_json::json;
use std::{
    convert::Infallible,
    future::Future,
    io,
    net::IpAddr,
    path::{Component, Path, PathBuf},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context as TaskContext, Poll},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf},
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    time::{Sleep, timeout},
};

#[cfg(test)]
use std::io::Read;

const MAX_BODY: usize = 50 * 1024 * 1024;
const PHOTO_BODY: usize = 30 * 1024 * 1024;

/// Options for the local or self-hosted server.
#[derive(clap::Args)]
pub struct Args {
    /// HTTP port. Use a reverse proxy for HTTPS.
    #[arg(long, default_value_t = 7878)]
    pub port: u16,
    /// IP address to listen on. Non-loopback addresses require an access token.
    #[arg(long, default_value = "127.0.0.1")]
    pub bind: IpAddr,
    /// API bearer token (at least 32 visible ASCII characters). Prefer BETA_TOKEN.
    #[arg(long)]
    pub token: Option<String>,
    /// Built Vite app directory, relative to the current directory.
    #[arg(long, default_value = "apps/web/dist")]
    pub web_dir: PathBuf,
}

fn access_token(args: &Args) -> anyhow::Result<Option<String>> {
    let token = match &args.token {
        Some(token) => Some(token.clone()),
        None => std::env::var("BETA_TOKEN")
            .map(Some)
            .or_else(|error| match error {
                std::env::VarError::NotPresent => Ok(None),
                _ => Err(anyhow::anyhow!("BETA_TOKEN must contain valid text")),
            })?,
    };
    validate_access(args.bind, token.as_deref())?;
    Ok(token)
}

fn validate_access(bind: IpAddr, token: Option<&str>) -> anyhow::Result<()> {
    // IPv4-mapped IPv6 loopback addresses are local too.
    let loopback = bind.is_loopback()
        || matches!(bind, IpAddr::V6(ip) if ip.to_ipv4_mapped().is_some_and(|ip| ip.is_loopback()));
    anyhow::ensure!(
        loopback || token.is_some(),
        "Non-loopback --bind requires --token or BETA_TOKEN. Use a random token of at least 32 characters and an HTTPS reverse proxy"
    );
    if let Some(token) = token {
        anyhow::ensure!(
            token.len() >= 32 && token.bytes().all(|b| b.is_ascii_graphic()),
            "The API token must contain at least 32 visible ASCII characters without spaces"
        );
    }
    Ok(())
}

fn check_access(url: &str, headers: &HeaderMap, token: Option<&str>) -> Result<(), Reply> {
    let path = url.split('?').next().unwrap_or(url);
    let Some(expected) = token.filter(|_| path == "/api" || path.starts_with("/api/")) else {
        return Ok(());
    };
    let mut authorization = headers.get_all(AUTHORIZATION).iter();
    let supplied = authorization.next().and_then(|h| {
        let (scheme, value) = h.to_str().ok()?.split_once(' ')?;
        scheme.eq_ignore_ascii_case("Bearer").then_some(value)
    });
    let Some(supplied) = supplied.filter(|_| authorization.next().is_none()) else {
        return Err(Reply::error(
            401,
            "API access requires Authorization: Bearer <token>",
        ));
    };
    // Compare every expected byte, including after a mismatch. Do not log tokens.
    let difference = expected.bytes().enumerate().fold(
        expected.len() ^ supplied.len(),
        |difference, (i, byte)| {
            difference | usize::from(byte ^ supplied.as_bytes().get(i).copied().unwrap_or(0))
        },
    );
    if difference != 0 {
        return Err(Reply::error(401, "Invalid API access token"));
    }
    Ok(())
}

struct Reply {
    status: u16,
    mime: &'static str,
    extension: Option<&'static str>,
    body: Vec<u8>,
}
impl Reply {
    fn bytes(mime: &'static str, body: Vec<u8>) -> Self {
        Self {
            status: 200,
            mime,
            extension: None,
            body,
        }
    }
    fn json(value: impl serde::Serialize) -> Self {
        match serde_json::to_vec(&value) {
            Ok(body) => Self::bytes("application/json; charset=utf-8", body),
            Err(error) => Self::error(500, &error.to_string()),
        }
    }
    fn from_output(output: beta_marker_api::Output) -> Self {
        match output {
            beta_marker_api::Output::Json(body) => {
                Self::bytes("application/json; charset=utf-8", body)
            }
            beta_marker_api::Output::File {
                mime,
                extension,
                body,
            } => Self {
                extension,
                ..Self::bytes(mime, body)
            },
        }
    }
    fn error(status: u16, message: &str) -> Self {
        Self {
            status,
            mime: "application/json; charset=utf-8",
            extension: None,
            body: json!({"error": message}).to_string().into_bytes(),
        }
    }
    fn response(self) -> Response<Full<Bytes>> {
        let mut response = Response::new(Full::new(Bytes::from(self.body)));
        *response.status_mut() =
            StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let headers = response.headers_mut();
        headers.insert("content-type", HeaderValue::from_static(self.mime));
        headers.insert(
            "x-content-type-options",
            HeaderValue::from_static("nosniff"),
        );
        headers.insert("cache-control", HeaderValue::from_static("no-store"));
        headers.insert("connection", HeaderValue::from_static("close"));
        if self.status == 401 {
            headers.insert(
                "www-authenticate",
                HeaderValue::from_static("Bearer realm=\"BETA Cutting Room\""),
            );
        }
        if let Some(extension) = self.extension {
            if let Ok(value) = HeaderValue::from_str(extension) {
                headers.insert("x-beta-extension", value);
            }
            if let Ok(value) =
                HeaderValue::from_str(&format!("attachment; filename=\"marker.{extension}\""))
            {
                headers.insert("content-disposition", value);
            }
        }
        response
    }
}

#[derive(Deserialize)]
struct MarkerRequest {
    job: Job,
    result: MarkerResult,
}
#[derive(Deserialize)]
struct NestRequest {
    job: Job,
    options: NestOptions,
}
use beta_marker_api::PlanRequest;
#[derive(Default)]
struct SearchStatus {
    started: Option<Instant>,
    running: bool,
    length: Option<f64>,
    efficiency: Option<f64>,
    elapsed_s: f64,
    result: Option<MarkerResult>,
    last_layout: Option<Instant>,
}
impl SearchStatus {
    fn reply(&self) -> Reply {
        Reply::json(json!({"running":self.running,
            "elapsed_s": if self.running { self.started.map_or(0.0, |t| t.elapsed().as_secs_f64()) } else { self.elapsed_s },
            "best_length_mm":self.length,"best_efficiency":self.efficiency,"result":self.result}))
    }
}
#[derive(Deserialize)]
struct ExportRequest {
    job: Job,
    result: MarkerResult,
    #[serde(default)]
    options: ExportOptions,
}
enum Route {
    Reply(Reply),
    Nest(Box<NestRequest>),
    Plan(Box<PlanRequest>),
    Status,
}
impl From<Reply> for Route {
    fn from(reply: Reply) -> Self {
        Self::Reply(reply)
    }
}

/// Keep the busy flag set even if a worker unwinds.
struct NestPermit(Arc<AtomicBool>);
impl NestPermit {
    fn acquire(busy: &Arc<AtomicBool>) -> Option<Self> {
        busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self(Arc::clone(busy)))
    }
}
impl Drop for NestPermit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn decode_path(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let h = char::from(*bytes.get(i + 1)?).to_digit(16)?;
            let l = char::from(*bytes.get(i + 2)?).to_digit(16)?;
            out.push((h * 16 + l) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Reject Windows paths, encoded traversal, symlinks outside the root and NULs.
fn static_path(root: &Path, url: &str) -> Option<PathBuf> {
    let decoded = decode_path(url)?;
    if !decoded.starts_with('/') || decoded.contains(['\\', ':', '\0']) {
        return None;
    }
    let relative = decoded.strip_prefix('/')?;
    if relative.split('/').any(|part| part == ".." || part == ".") {
        return None;
    }
    if Path::new(relative)
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
        && !relative.is_empty()
    {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let file = root
        .join(if relative.is_empty() {
            "index.html"
        } else {
            relative
        })
        .canonicalize()
        .ok()?;
    (file.starts_with(&root) && file.is_file()).then_some(file)
}

fn mime(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "wasm" => "application/wasm",
        "json" | "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn parse<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, Reply> {
    serde_json::from_slice(body)
        .map_err(|error| Reply::error(400, &format!("Invalid JSON: {error}")))
}

fn ensure_counts(job: &Job) -> Result<(), Reply> {
    for piece in &job.pieces {
        let multiplier = beta_marker_core::rules::multiplier(job, piece);
        if piece.quantity.checked_mul(multiplier).is_none()
            || piece.mirror_quantity.checked_mul(multiplier).is_none()
        {
            return Err(Reply::error(400, "Required copy count exceeds u32"));
        }
    }
    Ok(())
}

// Open fabric has one cut outline per demanded copy. Folded layouts can use
// half pieces, so their progress efficiency stays unknown until a result exists.
#[cfg(test)]
fn progress_area(job: &Job) -> Option<f64> {
    if job.fabric.layout != beta_marker_core::Layout::Open {
        return None;
    }
    let effective = beta_marker_core::rules::effective_job(job);
    Some(
        beta_marker_core::demands(&effective)
            .iter()
            .map(|d| {
                let placement = beta_marker_core::Placement {
                    piece_id: d.piece_id.clone(),
                    mirrored: d.mirrored,
                    ..Default::default()
                };
                beta_marker_core::geometry::area(&beta_marker_core::rules::placed_outline(
                    &effective.pieces[d.piece_index],
                    &placement,
                )) * f64::from(d.count)
            })
            .sum(),
    )
}

#[derive(Deserialize)]
struct PatternRequest {
    document: beta_pattern_core::Document,
    #[serde(default)]
    sizes: Vec<String>,
    #[serde(default = "pattern_tolerance")]
    tolerance_mm: f64,
    #[serde(default)]
    fabric: beta_marker_core::Fabric,
    #[serde(default)]
    ratio: std::collections::BTreeMap<String, u32>,
}
fn pattern_tolerance() -> f64 {
    0.1
}
#[derive(Deserialize)]
struct SeamlyRequest {
    pattern: String,
    measurements: Option<String>,
    size: Option<String>,
}
fn cutting_route(path: &str, body: &[u8]) -> Option<anyhow::Result<Reply>> {
    // Roll, spreading and cutting-room routes are shared with desktop and WASM.
    if let Some(output) = beta_marker_api::cutroom(path, body) {
        return Some(output.map(Reply::from_output));
    }
    let known = [
        "/api/pattern/starter",
        "/api/pattern/style",
        "/api/pattern/catalog",
        "/api/pattern/inspect",
        "/api/pattern/formula",
        "/api/pattern/snap",
        "/api/pattern/image-scale",
        "/api/pattern/measurements-import",
        "/api/pattern/measurements-export",
        "/api/pattern/move-point",
        "/api/pattern/nest",
        "/api/pattern/tool",
        "/api/pattern/piece-action",
        "/api/pattern/export-pdf",
        "/api/pattern/export-plt",
        "/api/pattern/import",
        "/api/pattern/from-job",
        "/api/pattern/grade",
        "/api/pattern/export-dxf",
        "/api/pattern/to-job",
        "/api/digitize",
        "/api/digitize/target",
    ];
    if !known.contains(&path) {
        return None;
    }
    Some((|| -> anyhow::Result<Reply> {
        match path {
            "/api/pattern/style" => {
                let request: serde_json::Value = serde_json::from_slice(body)?;
                Ok(Reply::json(
                    pattern_style::dispatch(&request).map_err(anyhow::Error::msg)?,
                ))
            }
            "/api/pattern/catalog" => Ok(Reply::json(beta_pattern_core::tools::catalog())),
            "/api/pattern/inspect"
            | "/api/pattern/formula"
            | "/api/pattern/snap"
            | "/api/pattern/image-scale"
            | "/api/pattern/measurements-import"
            | "/api/pattern/measurements-export"
            | "/api/pattern/move-point"
            | "/api/pattern/nest"
            | "/api/pattern/tool"
            | "/api/pattern/piece-action"
            | "/api/pattern/export-pdf"
            | "/api/pattern/export-plt" => {
                let r: serde_json::Value = serde_json::from_slice(body)?;
                let document: beta_pattern_core::Document =
                    serde_json::from_value(r["document"].clone())?;
                let size = r["size"].as_str().unwrap_or(&document.base_size);
                let adapter_error = |s: String| anyhow::anyhow!(s);
                match path {
                    "/api/pattern/measurements-import" => Ok(Reply::json(
                        pattern_editor::measurements(
                            &document,
                            r["source"].as_str().context("Choose a measurement file")?,
                            r["format"].as_str().unwrap_or("xml"),
                            size,
                        )
                        .map_err(adapter_error)?,
                    )),
                    "/api/pattern/measurements-export" => Ok(Reply::bytes(
                        "text/csv",
                        pattern_editor::measurement_csv(&document)
                            .map_err(adapter_error)?
                            .into_bytes(),
                    )),
                    "/api/pattern/move-point" => {
                        Ok(Reply::json(beta_pattern_core::tools::move_point(
                            &document,
                            r["id"].as_str().context("Choose a point")?,
                            size,
                            serde_json::from_value(r["target"].clone())?,
                        )?))
                    }
                    "/api/pattern/nest" => Ok(Reply::json(beta_pattern_core::evaluate::nest(
                        &document,
                        &document.sizes,
                        r["reference"]
                            .as_str()
                            .context("Choose a reference point")?,
                        0.1,
                    )?)),
                    "/api/pattern/inspect" => Ok(Reply::json(
                        pattern_editor::inspect(&document, size).map_err(adapter_error)?,
                    )),
                    "/api/pattern/snap" => Ok(Reply::json(
                        pattern_editor::snap(
                            &document,
                            size,
                            pattern_editor::SnapSettings {
                                at: serde_json::from_value(r["at"].clone())?,
                                radius: r["radius"].as_f64().unwrap_or(8.),
                                grid: r["grid"].as_f64().unwrap_or(10.),
                                points: r["points"].as_bool().unwrap_or(true),
                                paths: r["paths"].as_bool().unwrap_or(true),
                                ignore: r["ignore"].as_str(),
                            },
                        )
                        .map_err(adapter_error)?,
                    )),
                    "/api/pattern/image-scale" => Ok(Reply::json(
                        json!({"scale":pattern_editor::image_scale(serde_json::from_value(r["a"].clone())?,serde_json::from_value(r["b"].clone())?,r["distance_mm"].as_f64().context("Enter a known distance")?).map_err(adapter_error)?}),
                    )),
                    "/api/pattern/formula" => Ok(Reply::json(
                        json!({"value":pattern_editor::check_formula(&document,size,r["expression"].as_str().context("Enter a formula")?).map_err(adapter_error)?}),
                    )),
                    "/api/pattern/tool" => {
                        let tool = r["tool"].as_str().context("Choose a tool")?;
                        let id = r["id"].as_str().context("Enter an object ID")?;
                        let mut params = r["parameters"]
                            .as_object()
                            .context("Enter tool parameters")?
                            .clone();
                        let next = if let Some(selection) = r["selection"].as_array() {
                            let selection: Vec<String> = selection
                                .iter()
                                .map(|s| {
                                    s.as_str()
                                        .map(str::to_owned)
                                        .context("Choose valid group objects")
                                })
                                .collect::<anyhow::Result<_>>()?;
                            params.remove("source");
                            beta_pattern_core::tools::transform_group(
                                &document, &selection, id, tool, params,
                            )?
                        } else {
                            beta_pattern_core::tools::apply(&document, tool, id, params)?
                        };
                        pattern_editor::inspect(&next, size).map_err(adapter_error)?;
                        Ok(Reply::json(json!({"document":next})))
                    }
                    "/api/pattern/piece-action" => Ok(Reply::json(
                        pattern_operations::apply(&document, &r).map_err(adapter_error)?,
                    )),
                    "/api/pattern/export-pdf" => Ok(Reply::bytes(
                        "application/pdf",
                        pattern_output::pdf(&document, r["paper"].as_str().unwrap_or("A4"))
                            .map_err(adapter_error)?,
                    )),
                    _ => Ok(Reply::bytes(
                        "application/vnd.hp-hpgl",
                        pattern_output::plt(&document)
                            .map_err(adapter_error)?
                            .into_bytes(),
                    )),
                }
            }
            "/api/pattern/starter" => Ok(Reply::json(beta_pattern_core::starters::bodice())),
            "/api/pattern/import" => {
                let input: SeamlyRequest = serde_json::from_slice(body)?;
                Ok(Reply::json(beta_pattern_core::seamly::import(
                    &input.pattern,
                    input.measurements.as_deref(),
                    input.size.as_deref().unwrap_or("M"),
                )?))
            }
            "/api/pattern/from-job" => {
                let job: Job = serde_json::from_slice(body)?;
                Ok(Reply::json(beta_pattern_core::starters::from_marker_job(
                    &job,
                )?))
            }
            "/api/pattern/grade" | "/api/pattern/export-dxf" | "/api/pattern/to-job" => {
                let input: PatternRequest = serde_json::from_slice(body)?;
                anyhow::ensure!(
                    input.tolerance_mm.is_finite() && (0.01..=2.0).contains(&input.tolerance_mm),
                    "Pattern tolerance must be 0.01 to 2 mm"
                );
                let sizes = if input.sizes.is_empty() {
                    &input.document.sizes
                } else {
                    &input.sizes
                };
                anyhow::ensure!(
                    !sizes.is_empty() && sizes.len() <= 100,
                    "Choose 1 to 100 sizes"
                );
                match path {
                    "/api/pattern/export-dxf" => Ok(Reply::bytes(
                        "application/dxf",
                        beta_pattern_core::exchange::export_dxf(
                            &input.document,
                            sizes,
                            input.tolerance_mm,
                        )?
                        .into_bytes(),
                    )),
                    "/api/pattern/to-job" => Ok(Reply::json(beta_pattern_core::exchange::to_job(
                        &input.document,
                        input.fabric,
                        input.ratio,
                        input.tolerance_mm,
                    )?)),
                    _ => {
                        let mut graded = std::collections::BTreeMap::new();
                        for size in sizes {
                            graded.insert(
                                size,
                                beta_pattern_core::evaluate::grade(
                                    &input.document,
                                    size,
                                    input.tolerance_mm,
                                )?,
                            );
                        }
                        Ok(Reply::json(json!({"sizes":graded})))
                    }
                }
            }
            "/api/digitize/target" => {
                let target = beta_pattern_digitize::target::Target::tiled(
                    beta_pattern_digitize::target::SheetSize::A3,
                    1,
                    1,
                    false,
                )?;
                Ok(Reply::bytes("application/pdf", target.pdf()?))
            }
            "/api/digitize" => {
                use base64::Engine as _;
                let mut reader =
                    image::ImageReader::new(std::io::Cursor::new(body)).with_guessed_format()?;
                let mut limits = image::Limits::default();
                limits.max_image_width = Some(16000);
                limits.max_image_height = Some(16000);
                limits.max_alloc = Some(256 * 1024 * 1024);
                reader.limits(limits);
                let image = reader.decode()?.to_luma8();
                let target = beta_pattern_digitize::target::Target::tiled(
                    beta_pattern_digitize::target::SheetSize::A3,
                    1,
                    1,
                    false,
                )?;
                let scan = beta_pattern_digitize::scan(&image, &target, &Default::default())?;
                let mut png = std::io::Cursor::new(Vec::new());
                scan.overlay.write_to(&mut png, image::ImageFormat::Png)?;
                let overlay = format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(png.into_inner())
                );
                Ok(Reply::json(
                    json!({"pieces":scan.pieces,"report":scan.report,"overlay_png":overlay}),
                ))
            }
            _ => anyhow::bail!("Unknown cutting-room route"),
        }
    })())
}

fn route(method: &Method, url: &str, body: &[u8], root: &Path) -> Route {
    if body.len() > MAX_BODY {
        return Reply::error(413, "Request body exceeds 50 MB").into();
    }
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    let mut params = std::collections::BTreeMap::new();
    // Page flags and asset cache-busters do not configure Rust operations.
    // Keep the existing strict syntax/duplicate checks for every API request.
    let static_get = method == Method::GET && path != "/api" && !path.starts_with("/api/");
    if !static_get {
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            let Some((key, value)) = pair.split_once('=') else {
                return Reply::error(400, "Invalid query parameter").into();
            };
            if params.insert(key, value).is_some() {
                return Reply::error(400, "Duplicate query parameter").into();
            }
        }
    }
    if method == Method::GET && path == "/api/health" {
        return Reply::json(json!({"ok": true, "version": beta_marker_core::VERSION})).into();
    }
    if method == Method::GET && path == "/api/nest/status" {
        return Route::Status;
    }
    if path.starts_with("/api/") && method != Method::POST {
        return Reply::error(405, "Use POST for this API route").into();
    }
    if method == Method::POST {
        if path == "/api/digitize" && body.len() > PHOTO_BODY {
            return Reply::error(413, "Photo exceeds 30 MB").into();
        }
        if let Some(reply) = cutting_route(path, body) {
            return reply
                .unwrap_or_else(|e| Reply::error(400, &format!("{e:#}")))
                .into();
        }
    }
    match (method.as_str(), path) {
        ("POST", "/api/import") => {
            match params.get("format").copied().unwrap_or("dxf") {
                "dxf" => {}
                "plt" => {
                    return match beta_marker_api::import_plt(body, params.get("width").copied()) {
                        Ok(output) => Reply::from_output(output).into(),
                        Err(error) => Reply::error(400, &error).into(),
                    };
                }
                _ => return Reply::error(400, "Import format must be dxf or plt").into(),
            }
            let units = match params.get("units").copied().unwrap_or("auto") {
                "auto" => Units::Auto,
                "mm" => Units::Mm,
                "cm" => Units::Cm,
                "inch" => Units::Inch,
                _ => return Reply::error(400, "Units must be auto, mm, cm or inch").into(),
            };
            // Matches ImportOptions::default and WASM: left/right pairs unless pairs=false.
            let pairs = match params.get("pairs").copied().unwrap_or("true") {
                "true" => true,
                "false" => false,
                _ => return Reply::error(400, "Pairs must be true or false").into(),
            };
            // The importer splits an even Quantity N into N/2 normal + N/2 mirrored copies.
            match beta_marker_dxf::import_dxf(body, &ImportOptions { units, pairs }) {
                Ok(report) => Reply::json(json!({"report":report})).into(),
                Err(error) => Reply::error(400, &error.to_string()).into(),
            }
        }
        ("POST", "/api/nest") => match parse::<NestRequest>(body) {
            Ok(input) => {
                if let Err(error) = ensure_counts(&input.job) {
                    return error.into();
                }
                if !input.options.time_limit_s.is_finite() || input.options.time_limit_s <= 0.0 {
                    return Reply::error(400, "Time limit must be positive and finite").into();
                }
                if input.options.workers == Some(0) {
                    return Reply::error(400, "Workers must be greater than zero").into();
                }
                let issues = beta_marker_core::check_job(&input.job);
                if !issues.is_empty() {
                    return Reply::error(
                        400,
                        &issues
                            .iter()
                            .map(|issue| issue.message.as_str())
                            .collect::<Vec<_>>()
                            .join("; "),
                    )
                    .into();
                }
                Route::Nest(Box::new(input))
            }
            Err(error) => error.into(),
        },
        ("POST", "/api/plan") => match parse::<PlanRequest>(body) {
            Ok(input) => {
                if let Err(error) = beta_marker_api::check_plan(&input) {
                    return Reply::error(400, &error).into();
                }
                Route::Plan(Box::new(input))
            }
            Err(error) => error.into(),
        },
        ("POST", "/api/validate") => match parse::<MarkerRequest>(body) {
            Ok(input) => match ensure_counts(&input.job) {
                Ok(()) => Reply::json(beta_marker_core::validate(&input.job, &input.result)).into(),
                Err(error) => error.into(),
            },
            Err(error) => error.into(),
        },
        ("POST", "/api/export") => {
            let input = match parse::<ExportRequest>(body) {
                Ok(input) => input,
                Err(error) => return error.into(),
            };
            if let Err(error) = ensure_counts(&input.job) {
                return error.into();
            }
            let format = params.get("format").copied().unwrap_or("svg");
            match beta_marker_export::export(format, &input.job, &input.result, &input.options) {
                Ok((bytes, mime, extension)) => {
                    let mut reply = Reply::bytes(mime, bytes);
                    reply.extension = Some(extension);
                    reply.into()
                }
                Err(error) => Reply::error(400, &format!("{error:#}")).into(),
            }
        }
        ("GET", _) if !path.starts_with("/api/") => match static_path(root, path) {
            Some(file) => match std::fs::read(&file) {
                Ok(bytes) => Reply::bytes(mime(&file), bytes).into(),
                Err(error) => Reply::error(500, &error.to_string()).into(),
            },
            None => Reply::error(404, "File not found or path is not allowed").into(),
        },
        _ => Reply::error(404, "Unknown route").into(),
    }
}

#[cfg(test)]
fn read_body(
    reader: &mut impl Read,
    length: Option<usize>,
    limit: usize,
) -> Result<Vec<u8>, Reply> {
    if length.is_some_and(|length| length > limit) {
        return Err(Reply::error(
            413,
            &format!("Request body exceeds {} MB", limit / 1024 / 1024),
        ));
    }
    let mut body = Vec::new();
    reader
        .take((limit + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|error| Reply::error(400, &error.to_string()))?;
    if body.len() > limit {
        return Err(Reply::error(
            413,
            &format!("Request body exceeds {} MB", limit / 1024 / 1024),
        ));
    }
    Ok(body)
}

// One request per connection avoids retaining or draining a rejected body.
const MAX_CONNECTIONS: usize = 8;
const MAX_HEADERS: usize = 64;
const MAX_HEADER_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy)]
struct HttpLimits {
    max_body: usize,
    max_photo: usize,
    header_time: Duration,
    body_time: Duration,
    write_idle: Duration,
}
impl Default for HttpLimits {
    fn default() -> Self {
        Self {
            max_body: MAX_BODY,
            max_photo: PHOTO_BODY,
            header_time: Duration::from_secs(10),
            body_time: Duration::from_secs(30),
            write_idle: Duration::from_secs(30),
        }
    }
}

/// Bound a stalled response writer without timing out a running nesting job.
struct WriteTimeoutIo<T> {
    inner: T,
    idle: Duration,
    deadline: Option<Pin<Box<Sleep>>>,
}
impl<T> WriteTimeoutIo<T> {
    fn new(inner: T, idle: Duration) -> Self {
        Self {
            inner,
            idle,
            deadline: None,
        }
    }
    fn check_write<U>(
        &mut self,
        cx: &mut TaskContext<'_>,
        result: Poll<io::Result<U>>,
    ) -> Poll<io::Result<U>> {
        match result {
            Poll::Ready(result) => {
                self.deadline = None;
                Poll::Ready(result)
            }
            Poll::Pending => {
                let deadline = self
                    .deadline
                    .get_or_insert_with(|| Box::pin(tokio::time::sleep(self.idle)));
                if deadline.as_mut().poll(cx).is_ready() {
                    Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "HTTP response write timed out",
                    )))
                } else {
                    Poll::Pending
                }
            }
        }
    }
}
impl<T: AsyncRead + Unpin> AsyncRead for WriteTimeoutIo<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}
impl<T: AsyncWrite + Unpin> AsyncWrite for WriteTimeoutIo<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let result = Pin::new(&mut self.inner).poll_write(cx, buf);
        self.check_write(cx, result)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let result = Pin::new(&mut self.inner).poll_flush(cx);
        self.check_write(cx, result)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let result = Pin::new(&mut self.inner).poll_shutdown(cx);
        self.check_write(cx, result)
    }
}

struct ServiceState {
    root: PathBuf,
    token: Option<String>,
    busy: Arc<AtomicBool>,
    progress: Arc<Mutex<SearchStatus>>,
    compute_slots: Arc<Semaphore>,
    limits: HttpLimits,
}

fn body_limit(url: &str, limits: HttpLimits) -> usize {
    if url.split('?').next() == Some("/api/digitize") {
        limits.max_photo
    } else {
        limits.max_body
    }
}

async fn bounded_body(
    request: Request<Incoming>,
    token: Option<&str>,
    limits: HttpLimits,
) -> Result<Bytes, Reply> {
    let url = request
        .uri()
        .path_and_query()
        .map_or("/", |value| value.as_str());
    check_access(url, request.headers(), token)?;
    let limit = body_limit(request.uri().path(), limits);
    if let Some(length) = request.headers().get(hyper::header::CONTENT_LENGTH) {
        let length = length
            .to_str()
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .ok_or_else(|| Reply::error(400, "Invalid Content-Length"))?;
        if length > limit as u64 {
            return Err(Reply::error(
                413,
                &format!("Request body exceeds {} MB", limit / 1024 / 1024),
            ));
        }
    }
    // Limited also checks chunked/unknown-length DATA frames. No unbounded collect.
    match timeout(
        limits.body_time,
        Limited::new(request.into_body(), limit).collect(),
    )
    .await
    {
        Err(_) => Err(Reply::error(408, "Request body upload timed out")),
        Ok(Err(error)) if error.is::<LengthLimitError>() => Err(Reply::error(
            413,
            &format!("Request body exceeds {} MB", limit / 1024 / 1024),
        )),
        Ok(Err(_)) => Err(Reply::error(400, "Could not read request body")),
        Ok(Ok(body)) => Ok(body.to_bytes()),
    }
}

fn dispatch(method: Method, url: String, body: Bytes, state: &ServiceState) -> Reply {
    let work = match route(&method, &url, &body, &state.root) {
        Route::Reply(reply) => return reply,
        Route::Status => {
            return state
                .progress
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .reply();
        }
        work => work,
    };
    let Some(_permit) = NestPermit::acquire(&state.busy) else {
        return Reply::error(409, "A nesting or planning job is already running");
    };
    *state.progress.lock().unwrap_or_else(|e| e.into_inner()) = SearchStatus {
        started: Some(Instant::now()),
        running: true,
        ..Default::default()
    };
    let reply = match std::panic::catch_unwind(|| match work {
        Route::Nest(input) => {
            let mut on_progress = |p: beta_marker_nest::NestProgress| {
                let mut status = state.progress.lock().unwrap_or_else(|e| e.into_inner());
                if status.length.is_none_or(|length| p.length < length)
                    && status
                        .last_layout
                        .is_none_or(|last| last.elapsed().as_secs_f64() >= 0.5)
                {
                    status.length = Some(p.length);
                    status.efficiency = Some(p.result.efficiency);
                    status.result = Some(p.result);
                    status.last_layout = Some(Instant::now());
                }
            };
            let result = if beta_marker_match::needs_rules(&input.job) {
                beta_marker_match::nest_with_rules_and_progress(
                    &input.job,
                    &input.options,
                    &mut on_progress,
                )
            } else {
                beta_marker_nest::nest_with_progress(&input.job, &input.options, &mut on_progress)
            }?;
            let mut status = state.progress.lock().unwrap_or_else(|e| e.into_inner());
            status.length = Some(result.length);
            status.efficiency = Some(result.efficiency);
            status.result = Some(result.clone());
            Ok::<Reply, anyhow::Error>(Reply::json(result))
        }
        Route::Plan(input) => {
            beta_marker_api::plan(&input, &mut beta_marker_plan::checked_nest, |_| Ok(()))
                .map(Reply::json)
        }
        _ => unreachable!("only work routes reach the worker"),
    }) {
        Ok(Ok(reply)) => reply,
        Ok(Err(error)) => Reply::error(500, &error.to_string()),
        Err(_) => Reply::error(500, "Nesting worker failed"),
    };
    let mut status = state.progress.lock().unwrap_or_else(|e| e.into_inner());
    status.elapsed_s = status.started.map_or(0.0, |t| t.elapsed().as_secs_f64());
    status.running = false;
    reply
}

async fn handle(
    request: Request<Incoming>,
    state: Arc<ServiceState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = request.method().clone();
    let url = request
        .uri()
        .path_and_query()
        .map_or("/", |value| value.as_str())
        .to_owned();
    if let Err(reply) = check_access(&url, request.headers(), state.token.as_deref()) {
        return Ok(reply.response());
    }
    let Ok(permit) = Arc::clone(&state.compute_slots).try_acquire_owned() else {
        return Ok(Reply::error(503, "HTTP compute capacity is busy; try again later").response());
    };
    let reply = match bounded_body(request, state.token.as_deref(), state.limits).await {
        Err(reply) => reply,
        Ok(body) => {
            match tokio::task::spawn_blocking(move || {
                // Keep capacity reserved even if the client disconnects during computation.
                let _permit = permit;
                dispatch(method, url, body, &state)
            })
            .await
            {
                Ok(reply) => reply,
                Err(_) => Reply::error(500, "Request worker failed"),
            }
        }
    };
    Ok(reply.response())
}

/// Replay a bounded header prefix to Hyper after the length/authentication gate.
struct PrefixedIo<T> {
    inner: T,
    prefix: Bytes,
}
impl<T: AsyncRead + Unpin> AsyncRead for PrefixedIo<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if !self.prefix.is_empty() {
            let count = buf.remaining().min(self.prefix.len());
            buf.put_slice(&self.prefix.split_to(count));
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}
impl<T: AsyncWrite + Unpin> AsyncWrite for PrefixedIo<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

fn check_header_prefix(bytes: &[u8], state: &ServiceState) -> Result<(), Reply> {
    let mut parsed_headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let mut parsed = httparse::Request::new(&mut parsed_headers);
    match parsed.parse(bytes) {
        Ok(httparse::Status::Complete(_)) => {}
        Err(httparse::Error::TooManyHeaders) => {
            return Err(Reply::error(431, "Too many request headers"));
        }
        _ => return Err(Reply::error(400, "Invalid HTTP request headers")),
    }
    let uri: hyper::Uri = parsed
        .path
        .unwrap_or("/")
        .parse()
        .map_err(|_| Reply::error(400, "Invalid request URI"))?;
    let url = uri.path_and_query().map_or("/", |value| value.as_str());
    let mut headers = HeaderMap::new();
    for header in parsed.headers.iter() {
        let name = HeaderName::from_bytes(header.name.as_bytes())
            .map_err(|_| Reply::error(400, "Invalid header name"))?;
        let value = HeaderValue::from_bytes(header.value)
            .map_err(|_| Reply::error(400, "Invalid header value"))?;
        headers.append(name, value);
    }
    check_access(url, &headers, state.token.as_deref())?;
    let mut lengths = headers.get_all(hyper::header::CONTENT_LENGTH).iter();
    if let Some(value) = lengths.next() {
        if lengths.next().is_some() || headers.contains_key(hyper::header::TRANSFER_ENCODING) {
            return Err(Reply::error(400, "Ambiguous request body framing"));
        }
        let text = value
            .to_str()
            .map_err(|_| Reply::error(400, "Invalid Content-Length"))?
            .trim();
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(Reply::error(400, "Invalid Content-Length"));
        }
        let length = text
            .parse::<u64>()
            .map_err(|_| Reply::error(400, "Unparseable Content-Length"))?;
        let limit = body_limit(url, state.limits);
        if length > limit as u64 {
            return Err(Reply::error(
                413,
                &format!("Request body exceeds {} MB", limit / 1024 / 1024),
            ));
        }
    }
    Ok(())
}

async fn header_prefix(
    stream: &mut (impl AsyncRead + Unpin),
    state: &ServiceState,
) -> Result<Bytes, Reply> {
    let read = async {
        let mut bytes = Vec::with_capacity(MAX_HEADER_BYTES);
        loop {
            if bytes.len() == MAX_HEADER_BYTES {
                return Err(Reply::error(431, "Request headers exceed 16 KiB"));
            }
            let mut chunk = [0; 1024];
            let available = chunk.len().min(MAX_HEADER_BYTES - bytes.len());
            let count = stream
                .read(&mut chunk[..available])
                .await
                .map_err(|_| Reply::error(400, "Could not read HTTP headers"))?;
            if count == 0 {
                return Err(Reply::error(400, "Incomplete HTTP headers"));
            }
            bytes.extend_from_slice(&chunk[..count]);
            if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                check_header_prefix(&bytes[..end + 4], state)?;
                // Incidental bytes after the headers are bounded by this same 16 KiB buffer.
                return Ok(Bytes::from(bytes));
            }
        }
    };
    timeout(state.limits.header_time, read)
        .await
        .unwrap_or_else(|_| Err(Reply::error(408, "Request headers timed out")))
}

async fn early_reply(stream: &mut (impl AsyncWrite + Unpin), reply: Reply) -> io::Result<()> {
    let (parts, body) = reply.response().into_parts();
    let body = body
        .collect()
        .await
        .unwrap_or_else(|never| match never {})
        .to_bytes();
    let mut headers = format!(
        "HTTP/1.1 {} {}\r\n",
        parts.status.as_u16(),
        parts.status.canonical_reason().unwrap_or("Error")
    );
    for (name, value) in &parts.headers {
        if let Ok(value) = value.to_str() {
            headers.push_str(&format!("{name}: {value}\r\n"));
        }
    }
    headers.push_str(&format!("content-length: {}\r\n\r\n", body.len()));
    stream.write_all(headers.as_bytes()).await?;
    stream.write_all(&body).await?;
    stream.shutdown().await
}

async fn connection(stream: TcpStream, state: Arc<ServiceState>) -> anyhow::Result<()> {
    let mut stream = WriteTimeoutIo::new(stream, state.limits.write_idle);
    let prefix = match header_prefix(&mut stream, &state).await {
        Ok(prefix) => prefix,
        Err(reply) => {
            early_reply(&mut stream, reply).await?;
            return Ok(());
        }
    };
    let io = TokioIo::new(PrefixedIo {
        inner: stream,
        prefix,
    });
    let mut http = http1::Builder::new();
    http.timer(TokioTimer::new())
        .keep_alive(false)
        .max_headers(MAX_HEADERS)
        .max_header_size(MAX_HEADER_BYTES)
        .max_buf_size(MAX_HEADER_BYTES)
        .header_read_timeout(state.limits.header_time);
    http.serve_connection(
        io,
        service_fn(move |request| handle(request, Arc::clone(&state))),
    )
    .await?;
    Ok(())
}

async fn accept_loop(
    listener: TcpListener,
    state: Arc<ServiceState>,
    slots: Arc<Semaphore>,
) -> anyhow::Result<()> {
    loop {
        let (stream, _) = listener
            .accept()
            .await
            .context("Could not accept HTTP connection")?;
        let Ok(permit) = Arc::clone(&slots).try_acquire_owned() else {
            // No unbounded queue or per-rejected-connection task.
            drop(stream);
            continue;
        };
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let _permit = permit;
            // Malformed, idle and disconnected clients are closed without logging headers or tokens.
            let _ = connection(stream, state).await;
        });
    }
}

/// Run until stopped, with bounded HTTP transport and background compute workers.
pub fn run(args: Args) -> anyhow::Result<()> {
    let token = access_token(&args)?;
    let loopback = args.bind.is_loopback()
        || matches!(args.bind, IpAddr::V6(ip) if ip.to_ipv4_mapped().is_some_and(|ip| ip.is_loopback()));
    if !loopback {
        eprintln!(
            "WARNING: BETA Cutting Room is listening beyond this computer. Every API call requires a bearer token. HTTP is not encrypted: use Caddy/nginx for HTTPS and restrict access with a firewall. Static web assets are public."
        );
    }
    let address = std::net::SocketAddr::new(args.bind, args.port);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(3)
        .enable_all()
        .build()
        .context("Could not start HTTP runtime")?;
    runtime.block_on(async {
        let listener = TcpListener::bind(address)
            .await
            .context("Could not bind HTTP address")?;
        println!(
            "BETA Cutting Room: http://{} (web: {}; API token: {})",
            listener.local_addr()?,
            args.web_dir.display(),
            if token.is_some() {
                "required"
            } else {
                "not set (loopback only)"
            }
        );
        let state = Arc::new(ServiceState {
            root: args.web_dir,
            token,
            busy: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(Mutex::new(SearchStatus::default())),
            compute_slots: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
            limits: HttpLimits::default(),
        });
        accept_loop(listener, state, Arc::new(Semaphore::new(MAX_CONNECTIONS))).await
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn serve_cli_defaults_to_loopback_and_parses_explicit_network_options() {
        use clap::Parser;
        #[derive(Parser)]
        struct Options {
            #[command(flatten)]
            serve: Args,
        }
        let defaults = Options::try_parse_from(["serve"]).unwrap().serve;
        assert_eq!(defaults.bind.to_string(), "127.0.0.1");
        assert_eq!(defaults.port, 7878);
        assert!(defaults.token.is_none());
        let explicit =
            Options::try_parse_from(["serve", "--bind", "::1", "--port", "7880", "--token", TOKEN])
                .unwrap()
                .serve;
        assert_eq!(explicit.bind.to_string(), "::1");
        assert_eq!(explicit.port, 7880);
        assert_eq!(explicit.token.as_deref(), Some(TOKEN));
        assert!(Options::try_parse_from(["serve", "--bind", "localhost"]).is_err());
    }

    #[test]
    fn network_binding_requires_a_strong_token_and_loopback_stays_optional() {
        for bind in ["127.0.0.1", "127.1.2.3", "::1", "::ffff:127.0.0.1"] {
            assert!(
                validate_access(bind.parse().unwrap(), None).is_ok(),
                "{bind}"
            );
        }
        for bind in ["0.0.0.0", "192.0.2.1", "::", "::ffff:192.0.2.1"] {
            assert!(
                validate_access(bind.parse().unwrap(), None).is_err(),
                "{bind}"
            );
            assert!(
                validate_access(bind.parse().unwrap(), Some(TOKEN)).is_ok(),
                "{bind}"
            );
        }
        for token in [
            "",
            "short",
            "0123456789abcdef 123456789abcdef0",
            "0123456789abcdef0123456789abcdef\n",
        ] {
            assert!(validate_access("127.0.0.1".parse().unwrap(), Some(token)).is_err());
        }
    }

    #[test]
    fn all_api_paths_require_the_exact_bearer_token() {
        let mut valid = HeaderMap::new();
        valid.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {TOKEN}")).unwrap(),
        );
        let empty = HeaderMap::new();
        for url in [
            "/api",
            "/api/health",
            "/api/nest/status?poll=1",
            "/api/nest",
            "/api/unknown",
            "/api/export?format=pdf",
        ] {
            assert!(check_access(url, &empty, None).is_ok());
            assert_eq!(
                check_access(url, &empty, Some(TOKEN)).err().unwrap().status,
                401
            );
            assert!(check_access(url, &valid, Some(TOKEN)).is_ok());
            for value in [
                "Bearer wrong",
                "Basic abc",
                TOKEN,
                &format!("Bearer {TOKEN}extra"),
                &format!("Bearer {}", &TOKEN[..31]),
            ] {
                let mut invalid = HeaderMap::new();
                invalid.insert(AUTHORIZATION, HeaderValue::from_str(value).unwrap());
                assert!(
                    check_access(url, &invalid, Some(TOKEN)).is_err(),
                    "{url}: {value}"
                );
            }
            let mut duplicate = valid.clone();
            duplicate.append(AUTHORIZATION, valid[AUTHORIZATION].clone());
            assert!(check_access(url, &duplicate, Some(TOKEN)).is_err());
        }
        for url in ["/", "/index.html", "/wasm/engine.wasm", "/assets/app.js"] {
            assert!(check_access(url, &empty, Some(TOKEN)).is_ok());
        }
        assert!(
            check_access(
                "/api/health?token=0123456789abcdef0123456789abcdef",
                &empty,
                Some(TOKEN)
            )
            .is_err()
        );
    }

    fn test_state() -> Arc<ServiceState> {
        Arc::new(ServiceState {
            root: PathBuf::from("missing-web-root"),
            token: Some(TOKEN.into()),
            busy: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(Mutex::new(SearchStatus::default())),
            compute_slots: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
            limits: HttpLimits {
                header_time: Duration::from_millis(300),
                body_time: Duration::from_millis(300),
                write_idle: Duration::from_millis(300),
                ..HttpLimits::default()
            },
        })
    }

    // Serve one real connection, with a hard harness deadline so a regression cannot hang tests.
    fn wire(request: &str) -> String {
        wire_with_state(request, test_state())
    }

    fn wire_with_state(request: &str, state: Arc<ServiceState>) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let worker = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                timeout(Duration::from_secs(5), async {
                    let listener = TcpListener::from_std(listener).unwrap();
                    let (stream, _) = listener.accept().await.unwrap();
                    let _ = connection(stream, state).await;
                })
                .await
                .expect("bounded wire test");
            });
        });
        let mut client = std::net::TcpStream::connect(address).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        client.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        let result = client.read_to_string(&mut response);
        // Some platforms reset a connection after an early response with unread bytes.
        if let Err(error) = result {
            assert_eq!(error.kind(), io::ErrorKind::ConnectionReset);
        }
        drop(client);
        worker.join().unwrap();
        response
    }

    #[test]
    fn http_authentication_precedes_body_reads_and_preserves_upload_limits() {
        for (authorization, length, path, expected) in [
            (None, MAX_BODY as u64 + 1, "/api/import", 401),
            (Some(TOKEN), MAX_BODY as u64 + 1, "/api/import", 413),
            (
                Some(TOKEN),
                PHOTO_BODY as u64 + 1,
                "/api/digitize?mode=photo",
                413,
            ),
            (Some(TOKEN), u64::MAX, "/api/import", 413),
            (Some(TOKEN), 0, "/api/health", 200),
        ] {
            let authorization = authorization
                .map(|token| format!("Authorization: Bearer {token}\r\n"))
                .unwrap_or_default();
            // No body and an enormous declared length must get an immediate, closed response.
            let response = wire(&format!(
                "GET {path} HTTP/1.1\r\nHost: localhost\r\n{authorization}Content-Length: {length}\r\nConnection: close\r\n\r\n"
            ));
            assert!(
                response.starts_with(&format!("HTTP/1.1 {expected}")),
                "{response}"
            );
            if expected == 401 {
                assert!(
                    response
                        .to_ascii_lowercase()
                        .contains("www-authenticate: bearer")
                );
            }
        }
    }

    #[test]
    fn invalid_content_lengths_and_ambiguous_framing_return_400() {
        for length in ["not-a-number", "-1", "+1", "18446744073709551616"] {
            let response = wire(&format!(
                "POST /api/validate HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: {length}\r\n\r\n"
            ));
            assert!(response.starts_with("HTTP/1.1 400"), "{length}: {response}");
        }
        for framing in [
            "Content-Length: 1\r\nContent-Length: 2",
            "Content-Length: 1\r\nTransfer-Encoding: chunked",
        ] {
            let response = wire(&format!(
                "POST /api/validate HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {TOKEN}\r\n{framing}\r\n\r\n"
            ));
            assert!(response.starts_with("HTTP/1.1 400"), "{response}");
        }
    }

    #[test]
    fn chunked_upload_limit_is_enforced_on_the_wire() {
        let mut state = test_state();
        Arc::get_mut(&mut state).unwrap().limits.max_body = 4096;
        let chunk = "x".repeat(8192);
        let response = wire_with_state(
            &format!(
                "POST /api/validate HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {TOKEN}\r\nTransfer-Encoding: chunked\r\n\r\n2000\r\n{chunk}\r\n0\r\n\r\n"
            ),
            state,
        );
        assert!(response.starts_with("HTTP/1.1 413"), "{response}");
    }

    #[test]
    fn occupied_compute_capacity_rejects_before_upload_and_still_requires_auth() {
        let state = test_state();
        let _permit = Arc::clone(&state.compute_slots)
            .try_acquire_many_owned(MAX_CONNECTIONS as u32)
            .unwrap();
        let response = wire_with_state(
            &format!(
                "POST /api/validate HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: 1025\r\n\r\n"
            ),
            Arc::clone(&state),
        );
        assert!(response.starts_with("HTTP/1.1 503"), "{response}");
        let response =
            wire_with_state("GET /api/health HTTP/1.1\r\nHost: localhost\r\n\r\n", state);
        assert!(response.starts_with("HTTP/1.1 401"), "{response}");
    }

    #[test]
    fn stalled_upload_and_idle_headers_are_bounded() {
        let response = wire(&format!(
            "POST /api/validate HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: 1025\r\n\r\n"
        ));
        assert!(response.starts_with("HTTP/1.1 408"), "{response}");
        let start = Instant::now();
        let response = wire("GET /api/health HTTP/1.1\r\nHost:");
        assert!(
            response.is_empty() || response.starts_with("HTTP/1.1 408"),
            "{response}"
        );
        assert!(start.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn oversized_headers_and_duplicate_authorization_are_rejected() {
        let response = wire(&format!(
            "GET /api/health HTTP/1.1\r\nHost: localhost\r\nX-Large: {}\r\n\r\n",
            "x".repeat(MAX_HEADER_BYTES * 2)
        ));
        assert!(response.starts_with("HTTP/1.1 431"), "{response}");
        let response = wire(&format!(
            "GET /api/health HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {TOKEN}\r\nAuthorization: Bearer {TOKEN}\r\n\r\n"
        ));
        assert!(response.starts_with("HTTP/1.1 401"), "{response}");
        let headers = (0..MAX_HEADERS + 1)
            .map(|i| format!("X-{i}: value\r\n"))
            .collect::<String>();
        let response = wire(&format!(
            "GET / HTTP/1.1\r\nHost: localhost\r\n{headers}\r\n"
        ));
        assert!(response.starts_with("HTTP/1.1 431"), "{response}");
    }

    #[test]
    fn streaming_limit_and_response_write_timeout_are_bounded() {
        use tokio::io::AsyncWriteExt;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let body = Full::new(Bytes::from_static(b"12345"));
            assert!(
                Limited::new(body, 4)
                    .collect()
                    .await
                    .unwrap_err()
                    .is::<LengthLimitError>()
            );
            let (sender, _receiver) = tokio::io::duplex(1);
            let mut sender = WriteTimeoutIo::new(sender, Duration::from_millis(30));
            let error = sender.write_all(b"12").await.unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        });
    }

    #[test]
    fn connection_cap_rejects_excess_clients_and_releases_capacity() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            timeout(Duration::from_secs(5), async {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let address = listener.local_addr().unwrap();
                let slots = Arc::new(Semaphore::new(1));
                let task = tokio::spawn(accept_loop(listener, test_state(), Arc::clone(&slots)));
                let mut first = TcpStream::connect(address).await.unwrap();
                while slots.available_permits() != 0 { tokio::task::yield_now().await; }
                let mut excess = TcpStream::connect(address).await.unwrap();
                let mut byte = [0];
                match excess.read(&mut byte).await {
                    Ok(0) => {},
                    Err(error) if error.kind() == io::ErrorKind::ConnectionReset => {},
                    other => panic!("Excess connection was not closed: {other:?}"),
                }
                first.write_all(format!("GET /api/health HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {TOKEN}\r\n\r\n").as_bytes()).await.unwrap();
                let mut response = String::new();
                first.read_to_string(&mut response).await.unwrap();
                assert!(response.starts_with("HTTP/1.1 200"), "{response}");
                while slots.available_permits() != 1 { tokio::task::yield_now().await; }
                task.abort();
            }).await.expect("bounded connection-cap test");
        });
    }

    fn assert_wire(expected: &serde_json::Value, actual: &serde_json::Value) {
        match (expected, actual) {
            (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
                assert_eq!(a.as_f64(), b.as_f64())
            }
            (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
                assert_eq!(a.len(), b.len());
                for (a, b) in a.iter().zip(b) {
                    assert_wire(a, b);
                }
            }
            (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>());
                for (k, a) in a {
                    assert_wire(a, &b[k]);
                }
            }
            _ => assert_eq!(expected, actual),
        }
    }
    fn reply(method: Method, url: &str, body: &[u8]) -> Reply {
        match route(&method, url, body, Path::new("missing-web-root")) {
            Route::Reply(reply) => reply,
            Route::Nest(_) => panic!("unexpected nest"),
            Route::Plan(_) => panic!("unexpected plan"),
            Route::Status => SearchStatus::default().reply(),
        }
    }
    #[test]
    fn bounds_both_declared_and_chunked_bodies() {
        let mut data = std::io::Cursor::new(b"123456789");
        assert_eq!(read_body(&mut data, Some(9), 4).err().unwrap().status, 413);
        assert_eq!(data.position(), 0);
        assert_eq!(read_body(&mut data, None, 4).err().unwrap().status, 413);
        assert_eq!(data.position(), 5);
        assert_eq!(read_body(&mut &b"1234"[..], None, 4).ok().unwrap(), b"1234");
    }
    #[test]
    fn routes_shared_contract_without_opening_a_socket() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../packages/marker-wasm/test/fixtures/contract.json"
        ))
        .unwrap();
        let body = fixture.to_string();
        let validation = reply(Method::POST, "/api/validate", body.as_bytes());
        assert_eq!(validation.status, 200);
        let report: beta_marker_core::ValidationReport =
            serde_json::from_slice(&validation.body).unwrap();
        assert_eq!(report.length, 21.0);
        let exported = reply(Method::POST, "/api/export?format=report", body.as_bytes());
        assert_eq!(exported.status, 200);
        assert_eq!(exported.mime, "text/plain; charset=utf-8");
        assert_eq!(exported.extension, Some("txt"));
        assert!(String::from_utf8_lossy(&exported.body).contains("Job:"));
        assert_eq!(
            reply(Method::POST, "/api/export?format=unknown", body.as_bytes()).status,
            400
        );
        let body = json!({"job":fixture["job"],"options":{"time_limit_s":1}}).to_string();
        assert!(matches!(
            route(&Method::POST, "/api/nest", body.as_bytes(), Path::new(".")),
            Route::Nest(_)
        ));
        let mut overflow = fixture;
        overflow["job"]["pieces"][0]["quantity"] = json!(u32::MAX);
        assert_eq!(
            reply(
                Method::POST,
                "/api/validate",
                overflow.to_string().as_bytes()
            )
            .status,
            400
        );
    }
    #[test]
    fn static_query_flags_are_ignored_while_api_queries_stay_strict() -> anyhow::Result<()> {
        let root = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!("beta-static-query-{}", std::process::id()));
        std::fs::create_dir_all(&root)?;
        std::fs::write(
            root.join("index.html"),
            b"<!doctype html><title>Original query test</title>",
        )?;
        std::fs::write(root.join("app.js"), b"// Original static query test")?;
        let get = |url: &str| match route(&Method::GET, url, b"", &root) {
            Route::Reply(reply) => reply,
            _ => panic!("Expected static reply for {url}"),
        };
        for url in [
            "/?welcome",
            "/?feedback-ask&unknown",
            "/?x=1&x=2",
            "/index.html?welcome",
        ] {
            let reply = get(url);
            assert_eq!(reply.status, 200, "{url}");
            assert!(reply.body.starts_with(b"<!doctype html>"));
        }
        assert_eq!(get("/app.js?cache-bust&x=1&x=2").status, 200);
        assert_eq!(get("/%2e%2e/Cargo.toml?welcome").status, 404);
        for url in ["/api?welcome", "/api/health?welcome", "/api/health?x=1&x=2"] {
            assert_eq!(get(url).status, 400, "{url}");
        }
        assert_eq!(get("/api/health").status, 200);
        assert_eq!(
            reply(Method::POST, "/api/export?format=pdf&welcome", b"{}").status,
            400
        );
        std::fs::remove_file(root.join("index.html"))?;
        std::fs::remove_file(root.join("app.js"))?;
        std::fs::remove_dir(root)?;
        Ok(())
    }
    #[test]
    fn health_bad_input_and_missing_static() {
        let health = reply(Method::GET, "/api/health", b"");
        assert_eq!(health.status, 200);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&health.body).unwrap()["ok"],
            true
        );
        assert_eq!(reply(Method::POST, "/api/validate", b"{}").status, 400);
        assert_eq!(reply(Method::POST, "/api/nest", b"oops").status, 400);
        assert_eq!(reply(Method::GET, "/missing.js", b"").status, 404);
        assert_eq!(reply(Method::GET, "/api/nest", b"").status, 405);
        assert_eq!(
            reply(Method::POST, "/api/import?units=feet", b"").status,
            400
        );
        assert_eq!(
            reply(Method::POST, "/api/import?pairs=yes", b"").status,
            400
        );
        assert_eq!(
            reply(Method::POST, "/api/import?pairs=true&pairs=false", b"").status,
            400
        );
        assert_eq!(mime(Path::new("app.wasm")), "application/wasm");
    }
    #[test]
    fn v02_routes_preserve_options_and_plan_contract() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../packages/marker-wasm/test/fixtures/contract.json"
        ))
        .unwrap();
        let v = &fixture["v02"];
        let body = json!({"job":v["job"],"options":v["options"]}).to_string();
        let Route::Nest(input) = route(&Method::POST, "/api/nest", body.as_bytes(), Path::new("."))
        else {
            panic!("expected nesting work")
        };
        assert_eq!(input.options.runs, 3);
        assert_eq!(input.options.fixed[0].bundle, Some(0));
        assert!(input.options.fixed[0].on_fold);
        assert!(input.options.warm_start.is_some());
        assert_wire(
            &v["options"],
            &serde_json::to_value(&input.options).unwrap(),
        );
        let body = json!({"order":v["order"],"options":v["plan_options"]}).to_string();
        let Route::Plan(input) = route(&Method::POST, "/api/plan", body.as_bytes(), Path::new("."))
        else {
            panic!("expected planning work")
        };
        assert_eq!(input.order.quantities["M"], 20);
        assert_eq!(input.options.end_allowance_mm, 10.0);
        assert_wire(&v["order"], &serde_json::to_value(&input.order).unwrap());
        assert_wire(
            &v["plan_options"],
            &serde_json::to_value(&input.options).unwrap(),
        );
        let plan: beta_marker_plan::CutPlan = serde_json::from_value(v["plan"].clone()).unwrap();
        assert_wire(&v["plan"], &serde_json::to_value(&plan).unwrap());
        assert_eq!(
            plan.lays[0].result.as_ref().unwrap().placements[0].bundle,
            Some(0)
        );
        let mut invalid: serde_json::Value = serde_json::from_str(&body).unwrap();
        invalid["order"]["max_plies"] = json!(0);
        assert_eq!(
            reply(Method::POST, "/api/plan", invalid.to_string().as_bytes()).status,
            400
        );
        assert_eq!(reply(Method::POST, "/api/plan", b"{}").status, 400);
        let square = b"IN;SP1;PU0,0;PD4000,0,4000,4000,0,4000,0,0;PU;SP0;";
        let plt = reply(Method::POST, "/api/import?format=plt&width=1500", square);
        assert_eq!(plt.status, 200);
        let plt: serde_json::Value = serde_json::from_slice(&plt.body).unwrap();
        assert_eq!(plt["job"]["pieces"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            plt["result"]["placements"].as_array().map(Vec::len),
            Some(1)
        );
        assert_eq!(
            reply(Method::POST, "/api/import?format=plt&width=0", b"IN;").status,
            400
        );
        assert_eq!(
            reply(Method::POST, "/api/import?format=unknown", b"").status,
            400
        );
        let status = reply(Method::GET, "/api/nest/status", b"");
        let status: serde_json::Value = serde_json::from_slice(&status.body).unwrap();
        assert_eq!(status["running"], false);
        assert!(status["best_length_mm"].is_null());
        assert_eq!(status["elapsed_s"], 0.0);
        let imported = reply(
            Method::POST,
            "/api/import?format=dxf&width=1500",
            include_bytes!("../../../../testdata/dxf/shirt-aama-mm.dxf"),
        );
        assert_eq!(imported.status, 200);
        let imported: serde_json::Value = serde_json::from_slice(&imported.body).unwrap();
        assert!(imported["report"]["pieces"].as_array().unwrap().len() > 1);
        assert_eq!(
            progress_area(&serde_json::from_value::<Job>(fixture["job"].clone()).unwrap()),
            Some(800.0)
        );
    }

    fn floor_fixture() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../../packages/marker-wasm/test/fixtures/cutting-room.json"
        ))
        .unwrap()
    }
    fn post(path: &str, body: serde_json::Value) -> Reply {
        reply(Method::POST, path, body.to_string().as_bytes())
    }
    #[test]
    fn style_route_uses_the_shared_dispatcher_and_rejects_bad_edits() {
        let created = post(
            "/api/pattern/style",
            json!({"action":"create","request":{"block":"t_shirt"}}),
        );
        assert_eq!(created.status, 200);
        let doc: serde_json::Value = serde_json::from_slice(&created.body).unwrap();
        assert_eq!(doc["pieces"].as_array().unwrap().len(), 4);
        let graded = post(
            "/api/pattern/style",
            json!({"action":"grade","document":doc}),
        );
        assert_eq!(graded.status, 200);
        let bad = post(
            "/api/pattern/style",
            json!({"action":"edit","document":doc,"operations":[{"op":"delete_object","id":"front.A"}]}),
        );
        assert_eq!(bad.status, 400);
        assert_eq!(
            post("/api/pattern/style", json!({"action":"unknown"})).status,
            400
        );
    }
    #[test]
    fn pattern_editor_routes_validate_catalog_forms_and_physical_exports() {
        let doc = beta_pattern_core::starters::bodice();
        let catalog = post("/api/pattern/catalog", json!({}));
        assert_eq!(catalog.status, 200);
        assert!(
            serde_json::from_slice::<serde_json::Value>(&catalog.body)
                .unwrap()
                .as_array()
                .unwrap()
                .len()
                >= 35
        );
        let inspected = post("/api/pattern/inspect", json!({"document":doc,"size":"M"}));
        assert_eq!(inspected.status, 200);
        assert!(
            serde_json::from_slice::<serde_json::Value>(&inspected.body).unwrap()["objects"]
                .as_object()
                .unwrap()
                .len()
                > 20
        );
        assert_eq!(
            post(
                "/api/pattern/formula",
                json!({"document":doc,"size":"M","expression":"m.chest/4"})
            )
            .status,
            200
        );
        assert_eq!(
            post(
                "/api/pattern/formula",
                json!({"document":doc,"size":"M","expression":"m.missing"})
            )
            .status,
            400
        );
        let added = post(
            "/api/pattern/tool",
            json!({"document":doc,"tool":"absolute","id":"user_origin","parameters":{"x":0,"y":0}}),
        );
        assert_eq!(added.status, 200);
        let changed: serde_json::Value = serde_json::from_slice(&added.body).unwrap();
        assert!(changed["document"]["drawing"]["user_origin"].is_object());
        let pdf = post(
            "/api/pattern/export-pdf",
            json!({"document":doc,"paper":"A4"}),
        );
        assert_eq!(pdf.status, 200);
        assert_eq!(pdf.mime, "application/pdf");
        assert!(pdf.body.starts_with(b"%PDF"));
        let plt = post("/api/pattern/export-plt", json!({"document":doc}));
        assert_eq!(plt.status, 200);
        assert!(plt.body.starts_with(b"IN;"));
        let csv = post("/api/pattern/measurements-export", json!({"document":doc}));
        assert_eq!(csv.status, 200);
        assert_eq!(
            post(
                "/api/pattern/measurements-import",
                json!({"document":doc,"format":"csv","source":String::from_utf8(csv.body).unwrap()})
            )
            .status,
            200
        );
        assert_eq!(
            post(
                "/api/pattern/export-pdf",
                json!({"document":doc,"paper":"invalid"})
            )
            .status,
            400
        );
    }
    #[test]
    fn pattern_routes_grade_edit_import_and_export() {
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../packages/marker-wasm/test/fixtures/pattern.doc.json"
        ))
        .unwrap();
        let graded = post("/api/pattern/grade", json!({"document":document}));
        assert_eq!(graded.status, 200);
        let graded: serde_json::Value = serde_json::from_slice(&graded.body).unwrap();
        assert_eq!(graded["sizes"].as_object().unwrap().len(), 4);
        let mut edited = document.clone();
        edited["measurements"]["M"]["chest"] = json!(1000);
        let after = post("/api/pattern/grade", json!({"document":edited}));
        assert_eq!(after.status, 200);
        assert_ne!(
            graded,
            serde_json::from_slice::<serde_json::Value>(&after.body).unwrap()
        );
        let exported = post("/api/pattern/export-dxf", json!({"document":document}));
        assert_eq!(exported.status, 200);
        assert!(String::from_utf8_lossy(&exported.body).contains("BETA_PATTERN_RULES_BEGIN"));
        let job = post(
            "/api/pattern/to-job",
            json!({"document":document,"fabric":{"width":1500},"ratio":{"M":1}}),
        );
        assert_eq!(job.status, 200);
        let job: Job = serde_json::from_slice(&job.body).unwrap();
        assert!(!job.pieces.is_empty());
        assert_eq!(
            post("/api/pattern/to-job", json!({"document":document})).status,
            400
        );
        assert_eq!(
            post(
                "/api/pattern/grade",
                json!({"document":document,"tolerance_mm":0})
            )
            .status,
            400
        );
        let imported = post(
            "/api/pattern/import",
            json!({"pattern":include_str!("../../../../../pattern/crates/beta-pattern-core/tests/fixtures/basic.sm2d"),"measurements":include_str!("../../../../../pattern/crates/beta-pattern-core/tests/fixtures/basic.smis")}),
        );
        assert_eq!(imported.status, 200);
        assert_eq!(
            post("/api/pattern/import", json!({"pattern":"bad XML"})).status,
            400
        );
    }
    #[test]
    fn cutting_room_shared_fixture_routes_and_pdf_bytes() {
        let f = floor_fixture();
        let inspected = post("/api/roll/inspect", json!({"register":f["register"]}));
        assert_eq!(inspected.status, 200);
        let inspected: serde_json::Value = serde_json::from_slice(&inspected.body).unwrap();
        assert_eq!(inspected["inspection"]["rolls"][0]["score"]["passed"], true);
        assert_eq!(inspected["groups"][0]["roll_ids"][0], "R1");
        let allocated = post(
            "/api/roll/spread",
            json!({"register":f["register"],"lays":f["lays"]}),
        );
        assert_eq!(allocated.status, 200);
        let spread: beta_marker_roll::SpreadingPlan =
            serde_json::from_slice(&allocated.body).unwrap();
        assert!(spread.complete);
        assert_eq!(spread.total_used_m, 12.0);
        let from_spread = post(
            "/api/cutroom/from-spread",
            json!({"job":f["job"],"result":f["result"],"register":f["register"],"spread":spread,"lay_id":"LAY-01","order":"ORDER-01","colour":"Blue"}),
        );
        assert_eq!(
            from_spread.status,
            200,
            "{}",
            String::from_utf8_lossy(&from_spread.body)
        );
        let from_spread: serde_json::Value = serde_json::from_slice(&from_spread.body).unwrap();
        assert_eq!(
            from_spread["plies"][0]["shade_group"],
            r#"["LOT1","DYE1","A"]"#
        );

        let physical = post(
            "/api/cutroom/lay",
            json!({"job":f["job"],"result":f["result"],"id":"LAY-01","order":"ORDER-01","colour":"Blue","plies":f["lay"]["plies"]}),
        );
        assert_eq!(physical.status, 200);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&physical.body).unwrap(),
            f["lay"]
        );
        let bundled = post("/api/cutroom/bundles", json!({"lay":f["lay"]}));
        assert_eq!(bundled.status, 200);
        let bundles: serde_json::Value = serde_json::from_slice(&bundled.body).unwrap();
        assert_eq!(bundles[0]["stickers"].as_array().unwrap().len(), 2);
        for path in ["/api/cutroom/tickets", "/api/cutroom/stickers"] {
            for paper in ["a4", "label100x50"] {
                let pdf = post(path, json!({"bundles":bundles,"paper":paper}));
                assert_eq!(pdf.status, 200);
                assert!(pdf.body.starts_with(b"%PDF-1.4"));
            }
        }
        let damage = post(
            "/api/cutroom/recut",
            json!({"id":"DAMAGE1","bundle":bundles[0],"piece":bundles[0]["stickers"][0]["piece"],"reason":"Knife damage"}),
        );
        assert_eq!(damage.status, 200);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&damage.body).unwrap()["status"],
            "open"
        );
        let report = post("/api/cutroom/report", f["actuals"].clone());
        assert_eq!(report.status, 200);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&report.body).unwrap()["variance_mm"],
            0.0
        );
        let cost = post(
            "/api/cutroom/cost",
            json!({"job":f["job"],"garments":1,"price_per_m":100}),
        );
        assert_eq!(cost.status, 200);
        let mut bad = f["register"].clone();
        bad["rolls"][0]["measured_length_m"] = json!(-1);
        assert_eq!(
            post("/api/roll/inspect", json!({"register":bad})).status,
            400
        );
        for path in [
            "/api/roll/inspect",
            "/api/roll/spread",
            "/api/cutroom/bundles",
            "/api/cutroom/cost",
            "/api/cutroom/tickets",
            "/api/cutroom/recut",
            "/api/cutroom/report",
            "/api/cutroom/report-marker",
        ] {
            assert_eq!(post(path, json!({})).status, 400, "{path}");
        }
    }
    #[test]
    fn physical_lay_keeps_explicit_layer_garments_and_omits_surplus() {
        let mut f = floor_fixture();
        f["job"]["fabric"]["layout"] = json!("folded");
        f["job"]["order"]["sizes"]["M"] = json!(3);
        f["job"]["pieces"][0]["fold_line"] = json!([[0, 10], [20, 10]]);
        let mut first = f["result"]["placements"][0].clone();
        first["layer_allocations"] = json!([
            {"bundle":0,"mirrored":false}, {"bundle":1,"mirrored":false}
        ]);
        let mut last = first.clone();
        last["copy"] = json!(1);
        last["bundle"] = json!(2);
        last["translate"] = json!([21, 1]);
        last["layer_allocations"] = json!([{"bundle":2,"mirrored":false}, null]);
        f["result"]["placements"] = json!([first, last]);
        let response = post(
            "/api/cutroom/lay",
            json!({
                "job": f["job"], "result": f["result"], "id":"LAY-01",
                "order":"ORDER-01", "colour":"Blue", "plies":f["lay"]["plies"]
            }),
        );
        assert_eq!(
            response.status,
            200,
            "{}",
            String::from_utf8_lossy(&response.body)
        );
        let lay: beta_marker_cutroom::bundles::Lay =
            serde_json::from_slice(&response.body).unwrap();
        assert_eq!(
            lay.parts.iter().map(|p| p.garment).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert!(lay.parts.iter().all(|p| p.slot == 0 && !p.mirrored));
    }

    #[test]
    fn marker_pipeline_recomputes_end_allowance_and_physical_report() {
        let f = floor_fixture();
        let spread = post(
            "/api/roll/spread",
            json!({"register":f["register"],"markers":[{"id":"L1","job":f["job"],"result":f["result"],"plies":2,"end_allowance_mm":10}]}),
        );
        assert_eq!(
            spread.status,
            200,
            "{}",
            String::from_utf8_lossy(&spread.body)
        );
        let spread: beta_marker_roll::SpreadingPlan = serde_json::from_slice(&spread.body).unwrap();
        assert!((spread.total_used_m - 0.082).abs() < 1e-10);
        let source = json!({"job":f["job"],"result":f["result"],"id":"LAY-01","order":"ORDER-01","colour":"Blue","plies":f["lay"]["plies"]});
        let actuals = json!({"source":source,"issued_mm":50,"end_bits_mm":8,"actual_piece_area_mm2":800,"actual_pieces":2,"recut_count":0,"end_allowance_per_ply_mm":0});
        let report = post("/api/cutroom/report-marker", actuals.clone());
        assert_eq!(
            report.status,
            200,
            "{}",
            String::from_utf8_lossy(&report.body)
        );
        let report: serde_json::Value = serde_json::from_slice(&report.body).unwrap();
        assert_eq!(report["planned_pieces"], 2);
        assert_eq!(report["variance_mm"], 0.0);
        let mut folded = actuals;
        folded["source"]["job"]["fabric"]["layout"] = json!("folded");
        folded["actual_pieces"] = json!(4);
        folded["actual_piece_area_mm2"] = json!(1600);
        let report = post("/api/cutroom/report-marker", folded);
        assert_eq!(
            report.status,
            200,
            "{}",
            String::from_utf8_lossy(&report.body)
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&report.body).unwrap()["planned_pieces"],
            4
        );
    }
    #[test]
    fn all_marker_exports_return_real_bytes() {
        let f = floor_fixture();
        for format in [
            "svg", "plt", "pdf", "iso", "dxf", "xlsx", "labels", "mini", "report",
        ] {
            let output = post(&format!("/api/export?format={format}"), f.clone());
            assert_eq!(
                output.status,
                200,
                "{format}: {}",
                String::from_utf8_lossy(&output.body)
            );
            assert!(!output.body.is_empty());
            assert!(output.extension.is_some());
            if ["pdf", "labels", "mini"].contains(&format) {
                assert!(output.body.starts_with(b"%PDF"));
            }
            if format == "xlsx" {
                assert!(output.body.starts_with(b"PK"));
            }
        }
    }

    #[test]
    fn digitize_photo_returns_pieces_and_a_real_overlay_png() {
        let fixture = floor_fixture();
        let job: Job = serde_json::from_value(fixture["job"].clone()).unwrap();
        let target = beta_pattern_digitize::target::Target::tiled(
            beta_pattern_digitize::target::SheetSize::A3,
            1,
            1,
            false,
        )
        .unwrap();
        let options = beta_pattern_digitize::synthetic::PhotoOptions {
            dimensions: [1600, 1200],
            perspective: false,
            blur_px: 0.3,
            noise: 0.0,
            lighting: 0.0,
            k1: 0.0,
            ..Default::default()
        };
        let mut source = job.pieces[0].clone();
        source.outline = vec![[0.0, 0.0], [300.0, 0.0], [300.0, 160.0], [0.0, 160.0]];
        let photo =
            beta_pattern_digitize::synthetic::photograph(&target, &source, &options).unwrap();
        let mut bytes = std::io::Cursor::new(Vec::new());
        photo
            .image
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let response = reply(Method::POST, "/api/digitize", bytes.get_ref());
        assert_eq!(
            response.status,
            200,
            "{}",
            String::from_utf8_lossy(&response.body)
        );
        let output: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert!(!output["pieces"].as_array().unwrap().is_empty());
        assert!(
            output["overlay_png"]
                .as_str()
                .unwrap()
                .starts_with("data:image/png;base64,iVBOR")
        );
        if let Ok(dir) = std::env::var("BETA_UI_SMOKE_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                Path::new(&dir).join("synthetic-a3-photo.png"),
                bytes.into_inner(),
            )
            .unwrap();
        }
    }
    #[test]
    fn digitize_target_and_upload_errors_are_bounded() {
        let target = post("/api/digitize/target", json!({}));
        assert_eq!(target.status, 200);
        assert!(target.body.starts_with(b"%PDF"));
        let invalid = reply(Method::POST, "/api/digitize", b"not an image");
        assert_eq!(invalid.status, 400);
        let oversized = reply(Method::POST, "/api/digitize", &vec![0; PHOTO_BODY + 1]);
        assert_eq!(oversized.status, 413);
        let f = floor_fixture();
        let mut invalid = f["result"].clone();
        invalid["placements"][0]["bundle"] = json!(null);
        assert_eq!(post("/api/cutroom/lay",json!({"job":f["job"],"result":invalid,"id":"L1","order":"O1","colour":"Blue","plies":f["lay"]["plies"]})).status,400);
    }
    #[test]
    fn encoded_and_windows_traversal_is_blocked() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        assert!(static_path(&root, "/main.rs").is_some());
        for path in [
            "/../Cargo.toml",
            "/%2e%2e/Cargo.toml",
            "/%2e%2e%5cCargo.toml",
            "/C:/Windows/win.ini",
            "//server/file",
            "/main.rs%00",
            "/%FF",
            "/%2",
        ] {
            assert!(static_path(&root, path).is_none(), "{path}");
        }
    }
    #[test]
    fn only_one_nest_and_unwind_releases_permit() {
        let busy = Arc::new(AtomicBool::new(false));
        let permit = NestPermit::acquire(&busy).unwrap();
        assert!(NestPermit::acquire(&busy).is_none());
        drop(permit);
        let _ = std::panic::catch_unwind(|| {
            let _permit = NestPermit::acquire(&busy).unwrap();
            panic!("worker failure");
        });
        assert!(NestPermit::acquire(&busy).is_some());
    }
}
