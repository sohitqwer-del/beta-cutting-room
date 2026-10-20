//! CSV and plain-text reports; every field is escaped as text, without I/O.

use crate::{InspectionReport, SpreadingPlan};

fn csv(value: &str) -> String {
    // Protect identifier cells too: CSV quotes do not disable spreadsheet formulas.
    if value.trim_start().starts_with(['=', '+', '-', '@']) || value.starts_with(['\t', '\r', '\n'])
    {
        return format!("\"'{}\"", value.replace('"', "\"\""));
    }
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.into()
    }
}

impl SpreadingPlan {
    /// One row per lay/roll. Spliced plies may appear under more than one roll.
    /// End bits occur only at the roll's last use, so the column can be summed.
    pub fn to_csv(&self) -> String {
        let mut out = String::from("lay,roll,plies,used_m,end_bit_m,shade_group\n");
        for l in &self.lays {
            for r in &l.allocations {
                out.push_str(&format!(
                    "{},{},{},{:.6},{:.6},{}\n",
                    csv(&l.lay.id),
                    csv(&r.roll_id),
                    r.plies,
                    r.used_m,
                    r.end_bit_m,
                    csv(&r.shade_group)
                ));
            }
        }
        out
    }

    /// Tab-separated plain text, with control characters removed from labels.
    pub fn to_text(&self) -> String {
        let mut out = String::from("Lay\tRoll\tPlies\tUsed m\tEnd bit m\tShade group\n");
        for l in &self.lays {
            for r in &l.allocations {
                out.push_str(&format!(
                    "{}\t{}\t{}\t{:.3}\t{:.3}\t{}\n",
                    clean(&l.lay.id),
                    clean(&r.roll_id),
                    r.plies,
                    r.used_m,
                    r.end_bit_m,
                    clean(&r.shade_group)
                ));
            }
        }
        out.push_str(&format!(
            "Complete: {}; end bits: {:.3} m; overlaps: {:.3} m; total waste exposure: {:.3} m\n",
            self.complete, self.total_end_bits_m, self.splice_overlap_waste_m, self.total_waste_m
        ));
        for warning in &self.warnings {
            out.push_str(&format!("Warning: {}\n", clean(warning)));
        }
        out
    }
}

fn clean(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

impl InspectionReport {
    /// Separate roll and lot rows; pass/fail uses the recorded policy and unit.
    pub fn to_csv(&self) -> String {
        let mut out = String::from(
            "scope,id,lot,dye_lot,raw_points,scored_points,area_m2,points_per_100_sq_yd,points_per_100_m2,passed,failed_rolls\n",
        );
        for r in &self.rolls {
            let s = &r.score;
            out.push_str(&format!(
                "roll,{},{},{},{},{},{:.6},{:.6},{:.6},{},{}\n",
                csv(&r.roll_id),
                csv(&r.lot),
                csv(&r.dye_lot),
                s.raw_points,
                s.scored_points,
                s.area_m2,
                s.points_per_100_sq_yd,
                s.points_per_100_m2,
                s.passed,
                usize::from(!s.passed)
            ));
        }
        for l in &self.lots {
            let s = &l.score;
            out.push_str(&format!(
                "lot,,{},{},{},{},{:.6},{:.6},{:.6},{},{}\n",
                csv(&l.lot),
                csv(&l.dye_lot),
                s.raw_points,
                s.scored_points,
                s.area_m2,
                s.points_per_100_sq_yd,
                s.points_per_100_m2,
                s.passed,
                l.failed_rolls
            ));
        }
        out
    }

    /// Aligned-by-tabs inspection summary with explicit score units.
    pub fn to_text(&self) -> String {
        let mut out = format!(
            "Limit: {:.3} {:?}; point cap: {:?}\nScope\tId\tLot\tDye lot\tPoints\t/100 sq yd\t/100 m2\tPass\tFailed rolls\n",
            self.settings.limit, self.settings.unit, self.settings.point_cap
        );
        for r in &self.rolls {
            out.push_str(&format!(
                "Roll\t{}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{}\t{}\n",
                clean(&r.roll_id),
                clean(&r.lot),
                clean(&r.dye_lot),
                r.score.scored_points,
                r.score.points_per_100_sq_yd,
                r.score.points_per_100_m2,
                r.score.passed,
                usize::from(!r.score.passed)
            ));
        }
        for l in &self.lots {
            out.push_str(&format!(
                "Lot\t\t{}\t{}\t{}\t{:.3}\t{:.3}\t{}\t{}\n",
                clean(&l.lot),
                clean(&l.dye_lot),
                l.score.scored_points,
                l.score.points_per_100_sq_yd,
                l.score.points_per_100_m2,
                l.score.passed,
                l.failed_rolls
            ));
        }
        out
    }
}
