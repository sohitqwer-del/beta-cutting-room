//! Small JSON adapter shared by CLI and WASM. No filesystem or network access.
use crate::{
    floor::{Actuals, FloorState, LayStatus},
    intake::{self, Kind, Mapping},
    sheets,
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Every mutation takes a snapshot and returns a new, validated snapshot.
#[derive(Debug, Deserialize)]
pub struct Request {
    /// Operation name documented in the floor README.
    pub op: String,
    /// Complete accepted floor snapshot.
    #[serde(default)]
    pub state: FloorState,
    /// Operation-specific request or response data.
    #[serde(default)]
    pub data: Value,
}
/// Download bytes are passed to the existing browser/native save adapter.
#[derive(Debug, Serialize)]
pub struct Download {
    /// Suggested download filename; contains no filesystem path.
    pub name: String,
    /// Media type for the exported bytes.
    pub mime: String,
    /// Raw file contents.
    pub bytes: Vec<u8>,
}
/// Successful result. The client saves state only after this response exists.
#[derive(Debug, Serialize)]
pub struct Response {
    /// Complete accepted floor snapshot.
    pub state: FloorState,
    /// Operation-specific request or response data.
    pub data: Value,
    /// Optional exported file returned with a successful operation.
    pub download: Option<Download>,
}
#[derive(Deserialize)]
struct Import {
    kind: Kind,
    format: String,
    bytes: Vec<u8>,
    #[serde(default)]
    mapping: Mapping,
}
#[derive(Deserialize)]
struct Transition {
    id: String,
    status: LayStatus,
    actuals: Option<Actuals>,
    date: String,
    operator: String,
    #[serde(default)]
    note: String,
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .with_context(|| format!("{key} is required."))
}

/// Execute a typed operation with independent validation before and after mutation.
pub fn execute(r: Request) -> Result<Response> {
    let mut s = r.state;
    s.validate()?;
    let v = r.data;
    let mut data = Value::Null;
    let mut download = None;
    match r.op.as_str() {
        "validate" => {}
        "sync_register" => {
            let register: beta_marker_roll::RollRegister = serde_json::from_value(v)?;
            register.validate()?;
            for roll in &register.rolls {
                if let Some(old) = s.register.rolls.iter_mut().find(|old| old.id == roll.id) {
                    let reserved = s.lays.iter().any(|l| {
                        l.spread
                            .rolls
                            .iter()
                            .any(|r| r.opened && r.roll_id == roll.id)
                    });
                    anyhow::ensure!(
                        !reserved || old == roll,
                        "Reserved roll {} cannot change. Keep its saved measurements.",
                        roll.id
                    );
                    *old = roll.clone();
                } else {
                    s.register.rolls.push(roll.clone());
                }
            }
        }
        "import" => {
            let i: Import = serde_json::from_value(v)?;
            let p = intake::preview(&i.bytes, &i.format, i.kind, &i.mapping)?;
            let added = s.import(&p)?;
            data = json!({"added":added});
        }
        "template" => {
            let kind: Kind = text(&v, "kind")?.parse()?;
            let format = text(&v, "format")?;
            download = Some(Download {
                name: format!("beta-{}-v1.{format}", text(&v, "kind")?),
                mime: if format == "xlsx" {
                    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                } else {
                    "text/csv;charset=utf-8"
                }
                .into(),
                bytes: intake::template(kind, format)?,
            });
        }
        "quantities" => {
            let job = serde_json::from_value(v.get("job").context("job is required.")?.clone())?;
            data = json!(s.quantities(
                text(&v, "order")?,
                text(&v, "style")?,
                text(&v, "colour")?,
                &job
            )?);
        }
        "prepare" => {
            s.prepare(serde_json::from_value(v)?)?;
        }
        "advance" => {
            let t: Transition = serde_json::from_value(v)?;
            s.advance(&t.id, t.status, t.actuals, &t.date, &t.operator, &t.note)?;
        }
        "dashboard" => {
            data = json!(s.dashboard(text(&v, "date")?)?);
        }
        "issue" => {
            data = json!({"added":s.issue(text(&v,"payload")?,text(&v,"event_id")?,text(&v,"date")?,text(&v,"line")?,text(&v,"operator")?)?});
        }
        "damage" => {
            let n = v
                .get("bundle")
                .and_then(Value::as_u64)
                .and_then(|n| u32::try_from(n).ok())
                .context("Bundle number is required.")?;
            s.damage(
                text(&v, "lay")?,
                text(&v, "id")?,
                n,
                serde_json::from_value(v.get("piece").context("Piece is required.")?.clone())?,
                text(&v, "reason")?,
            )?;
        }
        "schedule" => {
            s.schedule(text(&v, "lay")?, text(&v, "batch")?)?;
        }
        "recut_advance" => {
            let l = s
                .lays
                .iter_mut()
                .find(|l| Some(l.id.as_str()) == v.get("lay").and_then(Value::as_str))
                .context("Unknown lay.")?;
            let d = l
                .damage
                .iter_mut()
                .find(|d| Some(d.id.as_str()) == v.get("id").and_then(Value::as_str))
                .context("Unknown damage record.")?;
            d.advance(serde_json::from_value(
                v.get("status").context("status is required.")?.clone(),
            )?)?;
        }
        "export" => {
            let kind = text(&v, "kind")?;
            let (name, mime, bytes) = match kind {
                "wip" => (
                    "beta-time-wip-v1.csv",
                    "text/csv;charset=utf-8",
                    sheets::wip_csv(&s)?,
                ),
                "consumption" => (
                    "beta-consumption-v1.csv",
                    "text/csv;charset=utf-8",
                    sheets::consumption_csv(&s)?,
                ),
                "backup" => (
                    "beta-floor-v1.json",
                    "application/json",
                    serde_json::to_vec_pretty(&s)?,
                ),
                _ => {
                    let l = s
                        .lays
                        .iter()
                        .find(|l| Some(l.id.as_str()) == v.get("lay").and_then(Value::as_str))
                        .context("Choose a saved floor lay.")?;
                    match kind {
                        "lay_sheet" => (
                            "beta-lay-sheet.html",
                            "text/html;charset=utf-8",
                            sheets::lay_html(l)?,
                        ),
                        "machine_csv" => (
                            "beta-spreader-v1.csv",
                            "text/csv;charset=utf-8",
                            sheets::machine_csv(l)?,
                        ),
                        "machine_json" => (
                            "beta-spreader-v1.json",
                            "application/json",
                            serde_json::to_vec_pretty(&sheets::machine(l)?)?,
                        ),
                        "sticker_order" => (
                            "beta-sticker-order.csv",
                            "text/csv;charset=utf-8",
                            sheets::stickers_csv(l)?,
                        ),
                        "tickets" => (
                            "beta-bundle-tickets.pdf",
                            "application/pdf",
                            crate::tickets::ticket_pdf(
                                &l.bundles,
                                crate::tickets::TicketPaper::A4,
                            )?,
                        ),
                        "stickers" => (
                            "beta-piece-stickers.pdf",
                            "application/pdf",
                            crate::tickets::sticker_pdf(
                                &l.bundles,
                                crate::tickets::TicketPaper::A4,
                            )?,
                        ),
                        "recuts" => (
                            "beta-recut-jobs.json",
                            "application/json",
                            serde_json::to_vec_pretty(&json!({"jobs":l.recuts,"ledger":l.damage}))?,
                        ),
                        _ => bail!("Unknown export kind."),
                    }
                }
            };
            download = Some(Download {
                name: name.into(),
                mime: mime.into(),
                bytes,
            });
        }
        _ => bail!("Unknown floor operation."),
    }
    s.validate()?;
    Ok(Response {
        state: s,
        data,
        download,
    })
}
