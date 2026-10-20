//! Persistent production queue. Registration is left to the CLI entry owner.
use anyhow::{Context, Result, ensure};
use beta_marker_queue::{Queue, RunOptions, Submission, TimeTier};
use std::{fs::File, io::BufReader, path::PathBuf};

/// Persistent queue commands, usable directly by the desktop/CLI integration.
#[derive(clap::Args)]
pub struct Args {
    /// Queue folder. Defaults to BETA/Marker/queue in the OS user-data directory.
    #[arg(long, global = true)]
    pub folder: Option<PathBuf>,
    /// Queue operation.
    #[command(subcommand)]
    pub command: Command,
}

/// Offline queue operations.
#[derive(clap::Subcommand)]
pub enum Command {
    /// Add an immutable Submission JSON (input geometry is embedded).
    Add {
        /// Submission JSON: id, name, priority, time and typed task.
        input: PathBuf,
        /// Override the scheduling priority. Higher numbers run first.
        #[arg(long)]
        priority: Option<i32>,
        /// Override time tier: draft/costing/production/overnight.
        #[arg(long, value_enum)]
        tier: Option<Tier>,
        /// Custom overnight search seconds. Requires --tier overnight.
        #[arg(long)]
        seconds: Option<f64>,
    },
    /// List job status, priority, summary and last failure as JSON.
    List,
    /// Run queued jobs in priority order; failures do not block other jobs.
    Run {
        /// Total engine-worker limit. Planner jobs need at least 2.
        #[arg(long, default_value_t = 2)]
        workers: usize,
        /// Reserved checkpoint interval. Current execution stays continuous.
        #[arg(long, default_value_t = 60.0)]
        slice_seconds: f64,
    },
    /// Cancel a queued/running/paused job, keeping its best valid checkpoint.
    Cancel {
        /// Stable queue job id.
        id: String,
    },
    /// Pause a queued/running job, keeping its checkpoint.
    Pause {
        /// Stable queue job id.
        id: String,
    },
    /// Resume a paused/cancelled/failed job from its checked checkpoint.
    Resume {
        /// Stable queue job id.
        id: String,
    },
    /// Read one durable job record and its result location.
    Status {
        /// Stable queue job id.
        id: String,
    },
}

/// Built-in commercial search tiers.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Tier {
    /// 20 search seconds.
    Draft,
    /// 60 search seconds.
    Costing,
    /// 300 search seconds.
    Production,
    /// Custom --seconds.
    Overnight,
}

fn print(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
/// Dispatch a queue operation. Jobs carry typed operations, never shell commands.
pub fn run(args: Args) -> Result<()> {
    let folder = match args.folder {
        Some(path) => path,
        None => default_folder()?,
    };
    let queue = Queue::open(folder)?;
    match args.command {
        Command::Add {
            input,
            priority,
            tier,
            seconds,
        } => {
            let mut request: Submission = serde_json::from_reader(BufReader::new(
                File::open(&input).with_context(|| format!("Read {}", input.display()))?,
            ))
            .context("Invalid queue submission JSON")?;
            if let Some(priority) = priority {
                request.priority = priority;
            }
            ensure!(
                seconds.is_none() || matches!(tier, Some(Tier::Overnight)),
                "--seconds requires --tier overnight."
            );
            if let Some(tier) = tier {
                request.time = match tier {
                    Tier::Draft => TimeTier::Draft,
                    Tier::Costing => TimeTier::Costing,
                    Tier::Production => TimeTier::Production,
                    Tier::Overnight => TimeTier::Overnight {
                        seconds: seconds.context("Overnight needs --seconds")?,
                    },
                };
            }
            print(&queue.add(request)?)
        }
        Command::List => {
            let (jobs, unreadable) = queue.list()?;
            print(&serde_json::json!({"jobs":jobs,"unreadable":unreadable}))
        }
        Command::Run {
            workers,
            slice_seconds,
        } => {
            let summary = queue.run(
                RunOptions {
                    max_workers: workers,
                    slice_seconds,
                },
                |event| {
                    eprintln!(
                        "{} {:?}: {}{}",
                        event.job_id,
                        event.status,
                        event.message,
                        if event.checkpoint_saved {
                            " [checkpoint saved]"
                        } else {
                            ""
                        }
                    )
                },
            )?;
            print(&summary)?;
            ensure!(
                summary.failed.is_empty() && summary.unreadable.is_empty(),
                "Queue drained with isolated failures; inspect status and summary.json."
            );
            Ok(())
        }
        Command::Cancel { id } => print(&queue.cancel(&id)?),
        Command::Pause { id } => print(&queue.pause(&id)?),
        Command::Resume { id } => print(&queue.resume(&id)?),
        Command::Status { id } => print(&queue.status(&id)?),
    }
}

fn default_folder() -> Result<PathBuf> {
    if cfg!(target_os = "windows") {
        return std::env::var_os("LOCALAPPDATA")
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .map(|p| p.join("BETA/Marker/queue"))
            .context("Supply --folder when LOCALAPPDATA is unavailable.");
    }
    if let Some(root) = std::env::var_os("XDG_DATA_HOME").filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(root).join("beta-marker/queue"));
    }
    let root = PathBuf::from(
        std::env::var_os("HOME")
            .filter(|p| !p.is_empty())
            .context("Supply --folder when the user data directory is unavailable.")?,
    );
    Ok(if cfg!(target_os = "macos") {
        root.join("Library/Application Support/BETA/Marker/queue")
    } else {
        root.join(".local/share/beta-marker/queue")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    #[derive(Parser)]
    struct Harness {
        #[command(flatten)]
        queue: Args,
    }
    #[test]
    fn queue_commands_parse_without_main_registration() {
        let a = Harness::try_parse_from([
            "beta-marker queue",
            "--folder",
            "queue",
            "run",
            "--workers",
            "2",
            "--slice-seconds",
            "0.05",
        ])
        .expect("run args");
        assert!(matches!(a.queue.command, Command::Run { workers: 2, .. }));
        for action in ["pause", "resume", "cancel", "status"] {
            assert!(Harness::try_parse_from(["queue", action, "job-1"]).is_ok());
        }
        assert!(
            Harness::try_parse_from([
                "queue",
                "add",
                "job.json",
                "--tier",
                "overnight",
                "--seconds",
                "120"
            ])
            .is_ok()
        );
    }
}
