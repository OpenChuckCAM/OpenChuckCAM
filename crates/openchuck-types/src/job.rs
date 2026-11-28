// SPDX-License-Identifier: AGPL-3.0-or-later
//! The job context passed into every long-running call (INT-04, UI-09).
//!
//! Libraries never spawn threads of their own or report progress any other
//! way (architecture section 8): a host passes a [`JobContext`], checks the
//! [`CancelToken`] it holds, and receives [`Progress`] through a [`ProgressSink`].

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{Error, Result};

/// A shared flag that asks a running job to stop.
///
/// Clones share the flag. Cancellation is cooperative: the job calls
/// [`CancelToken::check`] at convenient points and returns [`Error::Cancelled`].
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A token that is not cancelled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks the job to stop.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    /// True once [`CancelToken::cancel`] has been called on any clone.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    /// Returns [`Error::Cancelled`] once the token is cancelled.
    pub fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// How far a job has got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    /// What the job is doing, for display.
    pub stage: String,
    /// Units of work done.
    pub done: u64,
    /// Units of work in total, if known.
    pub total: Option<u64>,
}

/// Receives progress reports. Implementations must be cheap and must not block.
pub trait ProgressSink: Send + Sync {
    /// Called with each progress report.
    fn report(&self, progress: Progress);

    /// Opt in to diagnostic stage timings. Disabled hosts incur no clock reads.
    fn wants_timings(&self) -> bool {
        false
    }

    /// A completed scope, including scopes exited on error. Nested or parallel
    /// scopes overlap; durations must not be summed as job wall time.
    fn timing(&self, _stage: &str, _elapsed: std::time::Duration) {}
}

/// A sink that discards progress.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoProgress;

impl ProgressSink for NoProgress {
    fn report(&self, _progress: Progress) {}
}

/// The most memory a job may account for, as its host sets it (issue
/// #847): a limit the host keeps itself, from the computer's memory and the
/// user's setting, not a measure of what the system will give. Work that
/// counts its memory (adaptive spacing's points) stops the job with an
/// error before it would pass it, and nothing is made. It never changes
/// what a job that finishes makes (GC-07): only whether it finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryLimit {
    /// The limit, bytes.
    pub bytes: u64,
}

/// Everything a long-running library call needs from its host.
#[derive(Clone, Copy)]
pub struct JobContext<'a> {
    /// Cancellation flag.
    pub cancel: &'a CancelToken,
    /// Where progress goes.
    pub progress: &'a dyn ProgressSink,
    /// The most memory the job may account for; `None`, no limit (the
    /// host could not read the computer's memory, or a test).
    pub memory: Option<MemoryLimit>,
}

impl std::fmt::Debug for JobContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobContext")
            .field("cancel", self.cancel)
            .field("memory", &self.memory)
            .finish_non_exhaustive()
    }
}

impl<'a> JobContext<'a> {
    /// A context from a token and a sink, with no memory limit.
    pub fn new(cancel: &'a CancelToken, progress: &'a dyn ProgressSink) -> Self {
        Self {
            cancel,
            progress,
            memory: None,
        }
    }

    /// This context with the memory limit `memory`.
    #[must_use]
    pub fn with_memory(mut self, memory: Option<MemoryLimit>) -> Self {
        self.memory = memory;
        self
    }

    /// Returns [`Error::Cancelled`] once the job is cancelled.
    pub fn check_cancelled(&self) -> Result<()> {
        self.cancel.check()
    }

    /// Measure an optional diagnostic scope. Nested and parallel scopes overlap.
    pub fn time_stage<'b>(&'b self, stage: &'b str) -> StageTimer<'b> {
        StageTimer {
            sink: self.progress,
            stage,
            started: self.progress.wants_timings().then(std::time::Instant::now),
        }
    }

    /// Reports progress.
    pub fn report(&self, stage: &str, done: u64, total: Option<u64>) {
        self.progress.report(Progress {
            stage: stage.to_owned(),
            done,
            total,
        });
    }
}

/// A diagnostic scope reported to the host when it ends, including early returns.
/// It does not affect cancellation, progress counters or generated output.
pub struct StageTimer<'a> {
    sink: &'a dyn ProgressSink,
    stage: &'a str,
    started: Option<std::time::Instant>,
}
impl std::fmt::Debug for StageTimer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StageTimer")
            .field("stage", &self.stage)
            .field("started", &self.started)
            .finish_non_exhaustive()
    }
}
impl Drop for StageTimer<'_> {
    fn drop(&mut self) {
        if let Some(started) = self.started {
            self.sink.timing(self.stage, started.elapsed());
        }
    }
}
