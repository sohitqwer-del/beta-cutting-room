//! Cancellable solver process. The parent can kill the actual computation.
use crate::api::{self, Method, Reply, Route};
use serde_json::json;
use std::io::{BufRead, Read, Write};

pub(crate) fn run() -> anyhow::Result<()> {
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .take(52 * 1024 * 1024)
        .read_line(&mut line)?;
    let input: serde_json::Value = serde_json::from_str(&line)?;
    let path = input["path"].as_str().unwrap_or("");
    let body = serde_json::to_vec(&input["body"])?;
    if ["/api/pipeline", "/api/plan/pipeline"].contains(&path) {
        let reply = match crate::pipeline::run(&body) {
            Ok(value) => Reply::json(value),
            Err(e) => Reply::error(400, &format!("{e:#}")),
        };
        emit(json!({"reply":reply}));
        return Ok(());
    }
    let reply = match api::route(&Method::Post, path, &body) {
        Route::Nest(input) => {
            let width = input.job.fabric.width;
            let area = progress_area(&input.job);
            let result = if beta_marker_match::needs_rules(&input.job) {
                beta_marker_match::nest_with_rules(&input.job, &input.options)
            } else {
                beta_marker_nest::nest_with_progress(&input.job, &input.options, |p| {
                    emit(
                        json!({"progress":{"running":true,"elapsed_s":p.seconds,"best_length_mm":p.length,"best_efficiency":area.map(|a| a/(width*p.length))}}),
                    );
                })
            };
            match result {
                Ok(r) => Reply::json(r),
                Err(e) => Reply::error(400, &format!("{e:#}")),
            }
        }
        Route::Plan(input) => match beta_marker_plan::plan(&input.order, &input.options) {
            Ok(r) => Reply::json(r),
            Err(e) => Reply::error(400, &format!("{e:#}")),
        },
        Route::Reply(r) => r,
        Route::Status => Reply::error(400, "Worker needs a search request"),
    };
    emit(json!({"reply":reply}));
    Ok(())
}
fn emit(value: serde_json::Value) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{value}");
    let _ = out.flush();
}
pub(crate) fn progress_area(job: &beta_marker_core::Job) -> Option<f64> {
    if job.fabric.layout != beta_marker_core::Layout::Open {
        return None;
    }
    let job = beta_marker_core::rules::effective_job(job);
    Some(
        beta_marker_core::demands(&job)
            .iter()
            .map(|d| {
                let p = beta_marker_core::Placement {
                    piece_id: d.piece_id.clone(),
                    mirrored: d.mirrored,
                    ..Default::default()
                };
                beta_marker_core::geometry::area(&beta_marker_core::rules::placed_outline(
                    &job.pieces[d.piece_index],
                    &p,
                )) * f64::from(d.count)
            })
            .sum(),
    )
}

/// Close-on-drop Windows job ensures a crashed parent cannot leave a solver.
#[cfg(windows)]
pub(crate) struct ProcessJob(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl ProcessJob {
    pub fn attach(child: &std::process::Child) -> anyhow::Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
        // All pointers refer to initialized local structures; handles stay owned
        // until Drop, and the child's process handle remains live during this call.
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of_val(&info) as u32,
            ) == 0
                || AssignProcessToJobObject(job, child.as_raw_handle()) == 0
            {
                let error = std::io::Error::last_os_error();
                CloseHandle(job);
                return Err(error.into());
            }
            Ok(Self(job))
        }
    }
}
#[cfg(windows)]
impl Drop for ProcessJob {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
