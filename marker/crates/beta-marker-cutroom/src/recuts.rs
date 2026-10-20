//! Damaged-piece ledger and shade-separated replacement jobs.
use crate::bundles::{Bundle, PieceKey};
use crate::tickets::ScanPayload;
use anyhow::{Result, ensure};
use beta_marker_core::{Job, Layout};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Replacement lifecycle. Transitions are forward only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecutStatus {
    /// Damage recorded, awaiting a job.
    Open,
    /// Assigned to a re-cut job.
    Scheduled,
    /// Replacement physically cut.
    Cut,
    /// Replacement returned to its original bundle.
    Replaced,
}
/// One damaged physical piece. Persist the ledger as JSON in the calling application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Damage {
    /// Unique ledger record id.
    pub id: String,
    /// Source order id; lay ids may repeat between orders.
    pub order: String,
    /// Source lay id.
    pub lay: String,
    /// Floor bundle number.
    pub bundle: u32,
    /// Exact physical piece reference.
    pub piece: PieceKey,
    /// Size, checked against the bundle.
    pub size: String,
    /// Factory reason (hole, knife damage, etc.).
    pub reason: String,
    /// Replacement status.
    pub status: RecutStatus,
    /// Assigned job id, set only by scheduling.
    pub job_id: Option<String>,
}
impl Damage {
    /// Record damage only for a physical piece present in the source bundle.
    pub fn record(id: String, bundle: &Bundle, piece: PieceKey, reason: String) -> Result<Self> {
        ScanPayload::bundle(bundle)?;
        ensure!(
            !id.trim().is_empty() && !reason.trim().is_empty(),
            "damage id and reason are required"
        );
        ensure!(
            bundle.stickers.iter().any(|s| s.piece == piece),
            "damaged piece is not in bundle"
        );
        Ok(Self {
            id,
            order: bundle.order.clone(),
            lay: bundle.lay.clone(),
            bundle: bundle.number,
            piece,
            size: bundle.size.clone(),
            reason,
            status: RecutStatus::Open,
            job_id: None,
        })
    }
    /// Advance a scheduled replacement through cutting and return to sewing.
    pub fn advance(&mut self, status: RecutStatus) -> Result<()> {
        ensure!(
            self.job_id.is_some()
                && matches!(
                    (self.status, status),
                    (RecutStatus::Scheduled, RecutStatus::Cut)
                        | (RecutStatus::Cut, RecutStatus::Replaced)
                ),
            "invalid re-cut status transition"
        );
        self.status = status;
        Ok(())
    }
}
/// One small nesting job using a single inventory shade group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecutJob {
    /// Stable job identifier.
    pub id: String,
    /// Only fabric from this shade group may be used.
    pub shade_group: String,
    /// Source colour.
    pub colour: String,
    /// Exact replacement quantities. Inventory shade is also saved in `job.meta`.
    pub job: Job,
    /// Ledger records fulfilled by the job.
    pub damage_ids: Vec<String>,
}
/// Prepare and schedule open damage records atomically, grouped by order/style/colour/shade.
/// Original lay-coordinate defects are removed; garment rules and shrinkage are retained.
/// Replacements are cut on open fabric so single-piece demands do not create paired surplus.
pub fn schedule_recuts(
    source: &Job,
    bundles: &[Bundle],
    ledger: &mut [Damage],
    batch_id: &str,
) -> Result<Vec<RecutJob>> {
    ensure!(!batch_id.trim().is_empty(), "re-cut batch id is required");
    for p in &source.pieces {
        for q in [p.quantity, p.mirror_quantity] {
            ensure!(
                q.checked_mul(beta_marker_core::rules::multiplier(source, p))
                    .is_some(),
                "source demand overflow"
            );
        }
    }
    ensure!(
        beta_marker_core::check_job(source).is_empty(),
        "source job is invalid"
    );
    let mut lookup = BTreeMap::new();
    for b in bundles {
        ScanPayload::bundle(b)?;
        ensure!(
            lookup
                .insert((b.order.as_str(), b.lay.as_str(), b.number), b)
                .is_none(),
            "duplicate bundle identity"
        );
    }
    let mut ids = BTreeSet::new();
    let mut physical = BTreeSet::new();
    let mut grouped: BTreeMap<(&str, &str, &str, &str), Vec<&Damage>> = BTreeMap::new();
    for d in ledger.iter() {
        ensure!(ids.insert(&d.id), "duplicate damage id");
        ensure!(
            physical.insert((&d.order, &d.lay, &d.piece)),
            "physical piece already recorded as damaged"
        );
        if d.status != RecutStatus::Open {
            continue;
        }
        ensure!(
            d.job_id.is_none() && !d.reason.trim().is_empty() && !d.id.trim().is_empty(),
            "invalid open damage record"
        );
        let b = lookup
            .get(&(d.order.as_str(), d.lay.as_str(), d.bundle))
            .ok_or_else(|| anyhow::anyhow!("unknown damage bundle"))?;
        ensure!(
            d.size == b.size && b.stickers.iter().any(|s| s.piece == d.piece),
            "damage does not match bundle"
        );
        grouped
            .entry((&b.order, &b.style, &b.colour, &b.shade_group))
            .or_default()
            .push(d);
    }
    let mut out = Vec::new();
    for ((order, style, colour, shade), records) in grouped {
        let id = format!("{batch_id}-{}", out.len() + 1);
        ensure!(
            !ledger.iter().any(|d| d.job_id.as_ref() == Some(&id)),
            "re-cut job id already exists; use another batch id"
        );
        let mut job = source.clone();
        job.name = id.clone();
        job.order = None;
        job.pieces.clear();
        job.fabric.layout = Layout::Open;
        job.fabric.defects.clear();
        let mut quantities: BTreeMap<&str, (u32, u32)> = BTreeMap::new();
        for d in &records {
            let p = source
                .pieces
                .iter()
                .find(|p| p.id == d.piece.piece_id)
                .ok_or_else(|| anyhow::anyhow!("source piece missing"))?;
            ensure!(
                p.size.as_deref() == Some(&d.size),
                "damage size differs from source pattern"
            );
            let q = quantities.entry(&p.id).or_default();
            let n = if d.piece.mirrored { &mut q.1 } else { &mut q.0 };
            *n = n
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("replacement count overflow"))?;
        }
        for (piece_id, (normal, mirror)) in quantities {
            let mut p = source
                .pieces
                .iter()
                .find(|p| p.id == piece_id)
                .ok_or_else(|| anyhow::anyhow!("source piece missing"))?
                .clone();
            p.quantity = normal;
            p.mirror_quantity = mirror;
            job.pieces.push(p);
        }
        let selected: BTreeSet<_> = job.pieces.iter().map(|p| p.id.as_str()).collect();
        job.match_rules.retain(|r| {
            selected.contains(r.a.piece_id.as_str())
                && r.b
                    .as_ref()
                    .is_none_or(|p| selected.contains(p.piece_id.as_str()))
        });
        for (k, v) in [
            ("shade_group", shade),
            ("colour", colour),
            ("order", order),
            ("style", style),
        ] {
            job.meta.insert(k.into(), serde_json::json!(v));
        }
        let damage_ids: Vec<_> = records.iter().map(|d| d.id.clone()).collect();
        job.meta
            .insert("recut_damage_ids".into(), serde_json::json!(damage_ids));
        ensure!(
            beta_marker_core::check_job(&job).is_empty(),
            "replacement job failed check_job"
        );
        out.push(RecutJob {
            id,
            shade_group: shade.into(),
            colour: colour.into(),
            job,
            damage_ids,
        });
    }
    // Mutate only after every group has passed validation.
    for job in &out {
        for d in ledger.iter_mut().filter(|d| job.damage_ids.contains(&d.id)) {
            d.status = RecutStatus::Scheduled;
            d.job_id = Some(job.id.clone());
        }
    }
    Ok(out)
}
