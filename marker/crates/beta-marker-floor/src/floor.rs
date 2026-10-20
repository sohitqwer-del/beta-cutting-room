//! Durable floor records and forward-only physical production events.
use crate::{
    bundles::{Bundle, BundleRules},
    intake::{self, OrderLine, Preview},
    recuts::{Damage, RecutJob, RecutStatus},
    tickets::ScanPayload,
};
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Job, Layout, MarkerResult};
use beta_marker_roll::{
    AllocationSettings, LayRequest, RollRegister, ShadePolicy, SpliceZone, SpreadingPlan,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Persisted schema. Keep a JSON backup outside the browser cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorState {
    /// Persisted schema version; currently 1.
    pub version: u32,
    /// Received purchase-order lines with stable external IDs.
    pub orders: Vec<OrderLine>,
    /// Measured receiving stock and inspection records.
    pub register: RollRegister,
    /// Saved production lays, including frozen source geometry.
    pub lays: Vec<FloorLay>,
    /// Accepted whole-bundle issues to sewing.
    pub issues: Vec<SewingIssue>,
}
impl Default for FloorState {
    fn default() -> Self {
        Self {
            version: 1,
            orders: vec![],
            register: RollRegister::default(),
            lays: vec![],
            issues: vec![],
        }
    }
}
/// A physical lay cannot skip spreading, cutting or bundling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayStatus {
    /// Validated marker, stock and instructions are ready for release.
    Planned,
    /// Physical spreading and measured use are recorded.
    Spread,
    /// Physical cutting is recorded.
    Cut,
    /// Numbered bundles are ready for sewing issue.
    Bundled,
}
/// Audited measured cloth for one roll in one lay; all lengths in metres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActualRoll {
    /// Exact received roll identity.
    pub roll_id: String,
    /// Measured fabric withdrawn, in metres, before reusable returns.
    pub issued_m: f64,
    /// Reusable fabric returned, in metres.
    pub returned_m: f64,
}
/// Final physical measurement. Counts are spread passes, including folded passes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Actuals {
    /// Number of spread passes; folded passes have two physical layers.
    pub plies: u32,
    /// Per-roll records in receiving or execution scope.
    pub rolls: Vec<ActualRoll>,
    /// Operator note or explicit format limitation.
    pub note: String,
}
/// Operator and calendar date recorded for each accepted status change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayEvent {
    /// Current physical production stage.
    pub status: LayStatus,
    /// Calendar date in YYYY-MM-DD form.
    pub date: String,
    /// Person recording the physical event.
    pub operator: String,
    /// Operator note or explicit format limitation.
    pub note: String,
}
/// Source geometry and stock are frozen with each production instruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorLay {
    /// Stable identity of this record.
    pub id: String,
    /// Purchase-order identifier.
    pub order: String,
    /// Fabric colour; never combined across colours.
    pub colour: String,
    /// Calendar date in YYYY-MM-DD form.
    pub date: String,
    /// Factory cutting-table identifier.
    pub table: String,
    /// Current physical production stage.
    pub status: LayStatus,
    /// Source pattern geometry and garment rules.
    pub job: Job,
    /// Independently checked marker and recomputed metrics.
    pub marker: MarkerResult,
    /// Frozen measured stock used for this release.
    pub stock: RollRegister,
    /// Complete independently audited spreading allocation.
    pub spread: SpreadingPlan,
    /// Allowance at each end of every spread pass, in millimetres.
    pub end_allowance_mm: f64,
    /// Registered physical bundles and exact piece identities.
    pub bundles: Vec<Bundle>,
    /// Rules used to generate and independently reconstruct bundles.
    pub bundle_rules: BundleRules,
    /// Measured spreading result; absent until spreading is recorded.
    pub actuals: Option<Actuals>,
    /// Accepted forward-only status history.
    pub events: Vec<LayEvent>,
    /// Exact damaged-piece ledger for this original lay.
    pub damage: Vec<Damage>,
    /// Shade-separated replacement jobs linked to damage records.
    pub recuts: Vec<RecutJob>,
}
/// Form input. Safe splice zones use table metres, including the head allowance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prepare {
    /// Stable identity of this record.
    pub id: String,
    /// Purchase-order identifier.
    pub order: String,
    /// Fabric colour; never combined across colours.
    pub colour: String,
    /// Calendar date in YYYY-MM-DD form.
    pub date: String,
    /// Factory cutting-table identifier.
    pub table: String,
    /// Source pattern geometry and garment rules.
    pub job: Job,
    /// Independently checked marker and recomputed metrics.
    pub marker: MarkerResult,
    /// Number of spread passes; folded passes have two physical layers.
    pub plies: u32,
    /// Allowance at each end of every spread pass, in millimetres.
    pub end_allowance_mm: f64,
    /// Inspected roll identities approved for this release.
    pub roll_ids: Vec<String>,
    /// Full-width safe overlap intervals in table metres.
    #[serde(default)]
    pub splice_zones: Vec<SpliceZone>,
    /// Extra physical cloth at a join, in metres.
    #[serde(default = "default_overlap")]
    pub splice_overlap_m: f64,
    /// Maximum physical pieces in one floor bundle.
    #[serde(default = "default_bundle_size")]
    pub pieces_per_bundle: u32,
}
fn default_overlap() -> f64 {
    0.1
}
fn default_bundle_size() -> u32 {
    20
}

/// One whole-bundle issue to a sewing line. Repeat scans do not increment WIP.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SewingIssue {
    /// Stable issue-event identity for replay checks.
    pub event_id: String,
    /// Calendar date in YYYY-MM-DD form.
    pub date: String,
    /// Factory sewing-line identifier.
    pub line: String,
    /// Person recording the physical event.
    pub operator: String,
    /// Validated whole-bundle version-1 QR payload.
    pub payload: ScanPayload,
}
/// Counters distinguish planned fabric from the subset that has measurements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    /// Calendar date in YYYY-MM-DD form.
    pub date: String,
    /// Saved production lays, including frozen source geometry.
    pub lays: usize,
    /// Count of scheduled lays at each physical stage.
    pub by_status: BTreeMap<String, usize>,
    /// Planned cloth for all selected lays, including end and splice allowances.
    pub planned_m: f64,
    /// Number of selected lays with measured actuals.
    pub measured_lays: usize,
    /// Planned cloth for only the measured subset, in metres.
    pub measured_plan_m: f64,
    /// Measured issue minus reusable returns, in metres.
    pub used_m: f64,
    /// Actual use minus comparable measured-subset plan, in metres.
    pub variance_m: f64,
    /// Reusable fabric returned, in metres.
    pub returned_m: f64,
    /// Predicted remainder on opened rolls, distinct from actual returns.
    pub planned_end_bits_m: f64,
    /// Area-weighted marker efficiency, in percentage points.
    pub marker_efficiency_pct: Option<f64>,
    /// Geometric cut area for measured plies / measured cloth area; includes damaged parts.
    pub cutting_efficiency_pct: Option<f64>,
    /// Requested replacements not yet returned to their original bundles.
    pub open_recuts: usize,
    /// Damaged parts whose replacement has been returned.
    pub replaced: usize,
    /// Whole bundles issued on the selected issue date.
    pub issued_bundles: usize,
    /// Physical part count; never a finished-garment count.
    pub issued_parts: u64,
}

pub(crate) fn finite_nonnegative(n: f64, name: &str) -> Result<()> {
    ensure!(
        n.is_finite() && n >= 0.0,
        "{name} must be a finite nonnegative number."
    );
    Ok(())
}
fn validate_job(job: &Job) -> Result<()> {
    ensure!(job.pieces.len() <= 10000, "Too many pattern pieces.");
    let mut count = 0u64;
    for p in &job.pieces {
        count += u64::from(p.quantity) + u64::from(p.mirror_quantity);
        ensure!(
            p.quantity
                .checked_mul(beta_marker_core::rules::multiplier(job, p))
                .is_some()
                && p.mirror_quantity
                    .checked_mul(beta_marker_core::rules::multiplier(job, p))
                    .is_some(),
            "Pattern quantity overflow."
        );
    }
    ensure!(
        count <= 100000
            && job.order.as_ref().is_none_or(|o| o
                .sizes
                .values()
                .map(|v| u64::from(*v))
                .sum::<u64>()
                <= 100000),
        "Pattern demand is too large."
    );
    ensure!(
        beta_marker_core::check_job(job).is_empty(),
        "Source pattern has invalid rules or geometry."
    );
    Ok(())
}
impl FloorState {
    /// Validate restored state before rendering or changing it.
    pub fn validate(&self) -> Result<()> {
        ensure!(self.version == 1, "Unsupported floor backup version.");
        ensure!(
            self.orders.len() <= 10000
                && self.register.rolls.len() <= 10000
                && self.lays.len() <= 1000
                && self.issues.len() <= 100000,
            "Floor backup exceeds record limits."
        );
        self.register.validate()?;
        let mut ids = BTreeSet::new();
        for o in &self.orders {
            o.validate()?;
            ensure!(ids.insert(&o.external_id), "Duplicate external order id.");
        }
        let mut lays = BTreeSet::new();
        let mut stock = BTreeSet::new();
        for l in &self.lays {
            intake::label(&l.id, "lay id")?;
            intake::date(&l.date)?;
            validate_job(&l.job)?;
            ensure!(lays.insert(&l.id), "Duplicate lay id.");
            let check = beta_marker_core::validate(&l.job, &l.marker);
            ensure!(
                check.ok
                    && (check.length - l.marker.length).abs() < 0.001
                    && (check.piece_area - l.marker.piece_area).abs() < 0.01,
                "Saved lay marker or metrics are invalid."
            );
            beta_marker_roll::validate_plan(&l.stock, &l.spread)?;
            ensure!(
                l.spread.complete && l.spread.lays.len() == 1 && l.spread.lays[0].lay.id == l.id,
                "Invalid saved lay spread."
            );
            finite_nonnegative(l.end_allowance_mm, "Saved end allowance")?;
            ensure!(
                (l.spread.lays[0].lay.marker_length_m * 1000.0
                    - check.length
                    - 2.0 * l.end_allowance_mm)
                    .abs()
                    < 0.01,
                "Saved lay length and allowances disagree."
            );
            for r in l.spread.rolls.iter().filter(|r| r.opened) {
                ensure!(
                    stock.insert(&r.roll_id),
                    "A roll is reserved by more than one floor lay."
                );
                ensure!(
                    l.stock.rolls.iter().find(|s| s.id == r.roll_id)
                        == self.register.rolls.iter().find(|s| s.id == r.roll_id),
                    "Reserved stock differs from the receiving register."
                );
            }
            let mut numbers = BTreeSet::new();
            for b in &l.bundles {
                ScanPayload::bundle(b)?;
                ensure!(
                    b.order == l.order && b.lay == l.id && numbers.insert(b.number),
                    "Bundle identity does not match the lay."
                );
            }
            let physical = physical_lay(
                l,
                l.actuals
                    .as_ref()
                    .map_or(l.spread.lays[0].allocated_plies, |a| a.plies),
            )?;
            ensure!(
                crate::bundles::bundle_lay(&physical, l.bundle_rules)? == l.bundles,
                "Saved bundles differ from the released physical lay."
            );
            if let Some(a) = &l.actuals {
                validate_actuals(l, a)?;
            }
            ensure!(
                (l.status == LayStatus::Planned) == l.actuals.is_none(),
                "Measured lay status and actuals disagree."
            );
            let expected = match l.status {
                LayStatus::Planned => 0,
                LayStatus::Spread => 1,
                LayStatus::Cut => 2,
                LayStatus::Bundled => 3,
            };
            ensure!(
                l.events.len() == expected,
                "Lay history does not match its status."
            );
            for (event, status) in
                l.events
                    .iter()
                    .zip([LayStatus::Spread, LayStatus::Cut, LayStatus::Bundled])
            {
                intake::date(&event.date)?;
                intake::label(&event.operator, "operator")?;
                ensure!(event.status == status, "Invalid lay status history.");
            }
            let mut damage_ids = BTreeSet::new();
            let mut damage_pieces = BTreeSet::new();
            for d in &l.damage {
                ensure!(
                    damage_ids.insert(&d.id) && damage_pieces.insert(&d.piece),
                    "Duplicate damage record."
                );
                let b = l
                    .bundles
                    .iter()
                    .find(|b| b.number == d.bundle)
                    .context("Unknown damage bundle.")?;
                let original = Damage::record(d.id.clone(), b, d.piece.clone(), d.reason.clone())?;
                ensure!(
                    original.order == d.order && original.lay == d.lay && original.size == d.size,
                    "Damage record differs from source bundle."
                );
                ensure!(
                    if d.status == RecutStatus::Open {
                        d.job_id.is_none()
                    } else {
                        l.recuts.iter().any(|r| {
                            Some(&r.id) == d.job_id.as_ref() && r.damage_ids.contains(&d.id)
                        })
                    },
                    "Damage replacement job is missing or inconsistent."
                );
            }
        }
        let mut scans = BTreeSet::new();
        let mut events = BTreeSet::new();
        for s in &self.issues {
            intake::date(&s.date)?;
            intake::label(&s.line, "line")?;
            intake::label(&s.operator, "operator")?;
            let b = self.find_bundle(&s.payload)?;
            ensure!(
                ScanPayload::bundle(b)? == s.payload,
                "Saved scan differs from the registered bundle."
            );
            ensure!(
                events.insert(&s.event_id)
                    && scans.insert((&s.payload.o, &s.payload.l, s.payload.b)),
                "Duplicate saved sewing issue."
            );
            ensure!(
                self.lays
                    .iter()
                    .any(|l| l.id == s.payload.l && l.status == LayStatus::Bundled),
                "Issued lay is not bundled."
            );
        }
        Ok(())
    }
    /// Commit a checked preview atomically. Replaying identical records is a no-op.
    pub fn import(&mut self, p: &Preview) -> Result<usize> {
        ensure!(
            p.errors.is_empty(),
            "Fix all preview errors before importing."
        );
        let mut next = self.clone();
        let mut count = 0;
        match p.kind {
            intake::Kind::Orders => {
                ensure!(!p.orders.is_empty(), "No order rows to import.");
                for o in &p.orders {
                    o.validate()?;
                    if let Some(old) = next.orders.iter().find(|x| x.external_id == o.external_id) {
                        ensure!(
                            old == o,
                            "External id {} already exists with different data. Use a reviewed revision id.",
                            o.external_id
                        );
                    } else {
                        next.orders.push(o.clone());
                        count += 1;
                    }
                }
            }
            intake::Kind::Rolls => {
                ensure!(!p.register.rolls.is_empty(), "No rolls to import.");
                p.register.validate()?;
                for r in &p.register.rolls {
                    if let Some(old) = next.register.rolls.iter().find(|x| x.id == r.id) {
                        ensure!(
                            old == r,
                            "Roll {} already exists with different data. Review stock in Rolls first.",
                            r.id
                        );
                    } else {
                        next.register.rolls.push(r.clone());
                        count += 1;
                    }
                }
            }
        }
        next.validate()?;
        *self = next;
        Ok(count)
    }
    /// Aggregate one PO/style/colour for the existing cut planner. Never combine colours.
    pub fn quantities(
        &self,
        po: &str,
        style: &str,
        colour: &str,
        job: &Job,
    ) -> Result<BTreeMap<String, u32>> {
        let mut q = BTreeMap::<String, u32>::new();
        for o in self
            .orders
            .iter()
            .filter(|o| o.purchase_order == po && o.style == style && o.colour == colour)
        {
            ensure!(
                job.pieces
                    .iter()
                    .any(|p| p.size.as_deref() == Some(&o.size)),
                "No source patterns for size {}.",
                o.size
            );
            let v = q.entry(o.size.clone()).or_default();
            *v = v
                .checked_add(o.quantity)
                .context("Order quantity overflow.")?;
        }
        ensure!(!q.is_empty(), "Choose an imported PO/style/colour.");
        Ok(q)
    }
    /// Build a checked spread and physical bundles. Each opened roll is reserved once.
    pub fn prepare(&mut self, p: Prepare) -> Result<()> {
        for (s, n) in [
            (&p.id, "lay id"),
            (&p.order, "order"),
            (&p.colour, "colour"),
            (&p.table, "table"),
        ] {
            intake::label(s, n)?;
        }
        intake::date(&p.date)?;
        validate_job(&p.job)?;
        ensure!(
            !self.lays.iter().any(|l| l.id == p.id),
            "Lay id already exists. Choose a new id."
        );
        ensure!(
            (1..=10000).contains(&p.plies) && (1..=10000).contains(&p.pieces_per_bundle),
            "Plies and bundle size must be between 1 and 10000."
        );
        finite_nonnegative(p.end_allowance_mm, "End allowance")?;
        let checked = beta_marker_core::validate(&p.job, &p.marker);
        ensure!(
            checked.ok,
            "Marker must pass independent validation before spreading."
        );
        ensure!(
            !p.roll_ids.is_empty() && p.roll_ids.len() <= 10000,
            "Select inspected rolls."
        );
        let reserved: BTreeSet<_> = self
            .lays
            .iter()
            .flat_map(|l| {
                l.spread
                    .rolls
                    .iter()
                    .filter(|r| r.opened)
                    .map(|r| r.roll_id.as_str())
            })
            .collect();
        let mut stock = RollRegister::default();
        let mut seen = BTreeSet::new();
        for id in &p.roll_ids {
            ensure!(
                seen.insert(id) && !reserved.contains(id.as_str()),
                "Roll {id} is duplicated or already reserved. Receive verified returned ends under a new remnant id."
            );
            let r = self
                .register
                .rolls
                .iter()
                .find(|r| &r.id == id)
                .context("Unknown selected roll.")?;
            ensure!(
                r.shrinkage.length_pct <= p.job.fabric.shrinkage.length_pct + 1e-8
                    && r.shrinkage.width_pct <= p.job.fabric.shrinkage.width_pct + 1e-8,
                "Roll {id} needs more shrinkage compensation. Apply its tested shrinkage and re-nest."
            );
            stock.rolls.push(r.clone());
        }
        if let Some(required) = p
            .job
            .meta
            .get("shade_group")
            .and_then(serde_json::Value::as_str)
        {
            for r in &stock.rolls {
                let identity = serde_json::to_string(&[&r.lot, &r.dye_lot, &r.shade_group])?;
                ensure!(
                    identity == required,
                    "Replacement stock must match the original lot, dye lot and shade identity."
                );
            }
        }
        // The floor consumes approved inspection records. The default buyer policy is
        // deliberately conservative; changed buyer limits remain in the Rolls workflow.
        let inspection = beta_marker_roll::inspect(&stock, &Default::default())?;
        ensure!(
            inspection.rolls.iter().all(|r| r.score.passed),
            "Selected stock fails the default inspection policy. Review it in Rolls."
        );
        let length = (checked.length + 2.0 * p.end_allowance_mm) / 1000.0;
        let effective = beta_marker_core::rules::effective_job(&p.job);
        for z in &p.splice_zones {
            ensure!(
                z.start_m >= 0.0 && z.end_m <= length && z.end_m > z.start_m,
                "Splice zone is outside the table length."
            );
            for pl in &p.marker.placements {
                let piece = effective
                    .pieces
                    .iter()
                    .find(|piece| piece.id == pl.piece_id)
                    .context("Unknown placed piece.")?;
                let outline = beta_marker_core::rules::cut_outline(piece, pl);
                if let Some([lo, _, hi, _]) = beta_marker_core::geometry::bbox(&outline) {
                    ensure!(
                        (hi + p.end_allowance_mm) / 1000.0 <= z.start_m
                            || (lo + p.end_allowance_mm) / 1000.0 >= z.end_m,
                        "Splice zone crosses a cut piece. Choose a clear full-width zone."
                    );
                }
            }
        }
        let request = LayRequest {
            id: p.id.clone(),
            marker_length_m: length,
            plies: p.plies,
            fabric_width_mm: p.job.fabric.width,
            shade_rule: ShadePolicy::SameGroup,
            eligible_roll_ids: p.roll_ids,
            allow_splicing: !p.splice_zones.is_empty(),
            splice_zones: p.splice_zones,
        };
        let settings = AllocationSettings {
            splice_overlap_m: p.splice_overlap_m,
            ..Default::default()
        };
        let spread = beta_marker_roll::allocate(&stock, &[request], &settings)?;
        beta_marker_roll::validate_plan(&stock, &spread)?;
        ensure!(
            spread.complete,
            "Insufficient compatible stock. No floor lay was saved."
        );
        // Pending defect work must never become an executable machine sheet.
        for seg in &spread.lays[0].segments {
            let r = stock
                .rolls
                .iter()
                .find(|r| r.id == seg.roll_id)
                .context("Unknown spread roll.")?;
            ensure!(
                !r.defects
                    .iter()
                    .any(|d| d.action != beta_marker_roll::DefectAction::Ignore
                        && d.along_m < seg.roll_start_m + seg.length_m
                        && d.along_m + d.length_mm / 1000.0 > seg.roll_start_m),
                "Consumed cloth includes a recorded defect. Resolve it in Rolls and replan before floor release."
            );
        }
        let lay = crate::integration::from_spread(
            &p.job,
            &p.marker,
            &stock,
            &spread.lays[0],
            &p.order,
            &p.colour,
        )?;
        let bundle_rules = BundleRules {
            pieces_per_bundle: p.pieces_per_bundle,
            ..Default::default()
        };
        let bundles = crate::bundles::bundle_lay(&lay, bundle_rules)?;
        let mut marker = p.marker;
        marker.length = checked.length;
        marker.piece_area = checked.piece_area;
        marker.efficiency = checked.efficiency;
        self.lays.push(FloorLay {
            id: p.id,
            order: p.order,
            colour: p.colour,
            date: p.date,
            table: p.table,
            status: LayStatus::Planned,
            job: p.job,
            marker,
            stock,
            spread,
            end_allowance_mm: p.end_allowance_mm,
            bundles,
            bundle_rules,
            actuals: None,
            events: vec![],
            damage: vec![],
            recuts: vec![],
        });
        Ok(())
    }
    /// Advance one production stage. Measurements are entered once at spread completion.
    pub fn advance(
        &mut self,
        id: &str,
        status: LayStatus,
        actuals: Option<Actuals>,
        date: &str,
        operator: &str,
        note: &str,
    ) -> Result<()> {
        intake::date(date)?;
        intake::label(operator, "operator")?;
        let lay = self
            .lays
            .iter_mut()
            .find(|l| l.id == id)
            .context("Unknown lay.")?;
        ensure!(
            matches!(
                (lay.status, status),
                (LayStatus::Planned, LayStatus::Spread)
                    | (LayStatus::Spread, LayStatus::Cut)
                    | (LayStatus::Cut, LayStatus::Bundled)
            ),
            "Lay status must follow planned → spread → cut → bundled."
        );
        if status == LayStatus::Spread {
            let a = actuals.context("Enter measured plies, roll issue and returned ends.")?;
            validate_actuals(lay, &a)?;
            let physical = physical_lay(lay, a.plies)?;
            lay.bundles = crate::bundles::bundle_lay(&physical, lay.bundle_rules)?;
            lay.actuals = Some(a);
        } else {
            ensure!(
                actuals.is_none(),
                "Measurements are frozen after spread completion."
            );
        }
        lay.status = status;
        lay.events.push(LayEvent {
            status,
            date: date.into(),
            operator: operator.into(),
            note: note.into(),
        });
        Ok(())
    }
    fn find_bundle(&self, p: &ScanPayload) -> Result<&Bundle> {
        self.lays
            .iter()
            .flat_map(|l| &l.bundles)
            .find(|b| b.order == p.o && b.lay == p.l && b.number == p.b)
            .context("Unknown bundle. Prepare or restore the lay before scanning.")
    }
    /// Verify the full QR against the registered bundle and save one issue event.
    pub fn issue(
        &mut self,
        payload: &str,
        event_id: &str,
        date: &str,
        line: &str,
        operator: &str,
    ) -> Result<bool> {
        intake::date(date)?;
        for (s, n) in [
            (event_id, "event id"),
            (line, "line"),
            (operator, "operator"),
        ] {
            intake::label(s, n)?;
        }
        let p = ScanPayload::decode(payload)?;
        ensure!(
            p.piece.is_none(),
            "Scan a bundle ticket, not an individual piece sticker."
        );
        ensure!(
            ScanPayload::bundle(self.find_bundle(&p)?)? == p,
            "QR data differs from the registered bundle. Check the ticket."
        );
        ensure!(
            self.lays
                .iter()
                .any(|l| l.id == p.l && l.order == p.o && l.status == LayStatus::Bundled),
            "Finish bundling this lay before issuing it to sewing."
        );
        if let Some(old) = self
            .issues
            .iter()
            .find(|s| s.payload.o == p.o && s.payload.l == p.l && s.payload.b == p.b)
        {
            ensure!(
                old.line == line && old.payload == p,
                "Bundle was already issued to another line. A transfer needs a separate reviewed workflow."
            );
            return Ok(false);
        }
        ensure!(
            !self.issues.iter().any(|s| s.event_id == event_id),
            "Issue event id already exists."
        );
        self.issues.push(SewingIssue {
            event_id: event_id.into(),
            date: date.into(),
            line: line.into(),
            operator: operator.into(),
            payload: p,
        });
        Ok(true)
    }
    /// Record one exact damaged physical piece, without creating a duplicate request.
    pub fn damage(
        &mut self,
        lay_id: &str,
        id: &str,
        bundle: u32,
        piece: crate::bundles::PieceKey,
        reason: &str,
    ) -> Result<()> {
        let l = self
            .lays
            .iter_mut()
            .find(|l| l.id == lay_id)
            .context("Unknown lay.")?;
        ensure!(
            matches!(l.status, LayStatus::Cut | LayStatus::Bundled),
            "Record cut damage after the lay is cut."
        );
        ensure!(
            !l.damage.iter().any(|d| d.id == id || d.piece == piece),
            "This damage id or physical piece is already recorded."
        );
        let b = l
            .bundles
            .iter()
            .find(|b| b.number == bundle)
            .context("Unknown bundle.")?;
        l.damage
            .push(Damage::record(id.into(), b, piece, reason.into())?);
        Ok(())
    }
    /// Schedule exact shade-separated jobs with the existing cutroom engine.
    pub fn schedule(&mut self, lay_id: &str, batch: &str) -> Result<()> {
        let l = self
            .lays
            .iter_mut()
            .find(|l| l.id == lay_id)
            .context("Unknown lay.")?;
        let jobs = crate::recuts::schedule_recuts(&l.job, &l.bundles, &mut l.damage, batch)?;
        ensure!(!jobs.is_empty(), "No open damaged pieces to schedule.");
        l.recuts.extend(jobs);
        Ok(())
    }
    /// Area-weighted dashboard for scheduled lays on one calendar date.
    pub fn dashboard(&self, date: &str) -> Result<Dashboard> {
        intake::date(date)?;
        let mut d = Dashboard {
            date: date.into(),
            lays: 0,
            by_status: BTreeMap::new(),
            planned_m: 0.0,
            measured_lays: 0,
            measured_plan_m: 0.0,
            used_m: 0.0,
            variance_m: 0.0,
            returned_m: 0.0,
            planned_end_bits_m: 0.0,
            marker_efficiency_pct: None,
            cutting_efficiency_pct: None,
            open_recuts: 0,
            replaced: 0,
            issued_bundles: 0,
            issued_parts: 0,
        };
        let (mut marker_area, mut piece_area, mut actual_area, mut cut_area) = (0.0, 0.0, 0.0, 0.0);
        for l in self.lays.iter().filter(|l| l.date == date) {
            d.lays += 1;
            *d.by_status
                .entry(format!("{:?}", l.status).to_lowercase())
                .or_default() += 1;
            d.planned_m += l.spread.total_used_m;
            d.planned_end_bits_m += l.spread.total_end_bits_m;
            let layers = if l.job.fabric.layout == Layout::Open {
                1.0
            } else {
                2.0
            };
            let plies = f64::from(l.spread.lays[0].allocated_plies);
            marker_area += l.marker.length * l.job.fabric.width * layers * plies;
            piece_area += l.marker.piece_area * layers * plies;
            if let Some(a) = &l.actuals {
                let used = a
                    .rolls
                    .iter()
                    .map(|r| r.issued_m - r.returned_m)
                    .sum::<f64>();
                d.measured_lays += 1;
                d.measured_plan_m += l.spread.total_used_m;
                d.used_m += used;
                d.returned_m += a.rolls.iter().map(|r| r.returned_m).sum::<f64>();
                // Each measured roll's own width weights cutting yield. Width trim counts as loss.
                for r in &a.rolls {
                    let width = l
                        .stock
                        .rolls
                        .iter()
                        .find(|s| s.id == r.roll_id)
                        .context("Unknown actual roll.")?
                        .usable_width_mm;
                    actual_area += (r.issued_m - r.returned_m) * 1000.0 * width * layers;
                }
                cut_area += l.marker.piece_area * layers * f64::from(a.plies);
            }
            d.open_recuts += l
                .damage
                .iter()
                .filter(|r| r.status != RecutStatus::Replaced)
                .count();
            d.replaced += l
                .damage
                .iter()
                .filter(|r| r.status == RecutStatus::Replaced)
                .count();
        }
        for i in self.issues.iter().filter(|i| i.date == date) {
            d.issued_bundles += 1;
            d.issued_parts += u64::from(i.payload.q);
        }
        d.variance_m = d.used_m - d.measured_plan_m;
        if marker_area > 0.0 {
            d.marker_efficiency_pct = Some(piece_area / marker_area * 100.0);
        }
        if actual_area > 0.0 {
            d.cutting_efficiency_pct = Some(cut_area / actual_area * 100.0);
        }
        Ok(d)
    }
}
fn physical_lay(l: &FloorLay, plies: u32) -> Result<crate::bundles::Lay> {
    let mut physical = crate::integration::from_spread(
        &l.job,
        &l.marker,
        &l.stock,
        &l.spread.lays[0],
        &l.order,
        &l.colour,
    )?;
    physical.plies.retain(|r| r.first <= plies);
    for r in &mut physical.plies {
        r.last = r.last.min(plies);
    }
    Ok(physical)
}
fn validate_actuals(l: &FloorLay, a: &Actuals) -> Result<()> {
    ensure!(
        a.plies > 0 && a.plies <= l.spread.lays[0].allocated_plies,
        "Actual plies must be positive and cannot exceed the released lay. Replan extra plies."
    );
    ensure!(
        a.plies == l.spread.lays[0].allocated_plies || !a.note.trim().is_empty(),
        "Explain the shorter lay in the measurement note. Reprint the reduced bundle set before issue."
    );
    let opened: BTreeSet<_> = l
        .spread
        .rolls
        .iter()
        .filter(|r| r.opened)
        .map(|r| r.roll_id.as_str())
        .collect();
    let mut seen = BTreeSet::new();
    let mut used = 0.0;
    for r in &a.rolls {
        ensure!(
            seen.insert(r.roll_id.as_str()) && opened.contains(r.roll_id.as_str()),
            "Actual roll must occur once and belong to this released lay."
        );
        finite_nonnegative(r.issued_m, "Issued metres")?;
        finite_nonnegative(r.returned_m, "Returned metres")?;
        let stock = l
            .stock
            .rolls
            .iter()
            .find(|s| s.id == r.roll_id)
            .context("Unknown actual roll.")?;
        ensure!(
            r.returned_m <= r.issued_m && r.issued_m <= stock.measured_length_m + 1e-8,
            "Returned ends exceed issue, or issue exceeds measured roll stock."
        );
        used += r.issued_m - r.returned_m;
    }
    ensure!(
        seen == opened,
        "Enter issue and returns for every opened roll."
    );
    ensure!(
        used * 1000.0 + 0.01 >= l.marker.length * f64::from(a.plies),
        "Measured fabric cannot cover the actual marker plies. Check units and returns."
    );
    Ok(())
}
