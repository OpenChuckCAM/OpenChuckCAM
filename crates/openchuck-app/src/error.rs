// SPDX-License-Identifier: AGPL-3.0-or-later
use std::path::PathBuf;

/// An error from the application layer, worded for the user (NFR-07).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AppError {
    /// The project folder does not exist.
    #[error("project folder '{}' does not exist", .0.display())]
    ProjectNotFound(PathBuf),
    /// The path exists but is not a folder.
    #[error("'{}' is not a project folder", .0.display())]
    NotAProjectFolder(PathBuf),
    /// The folder exists but holds no project (no `project.toml`). Front ends
    /// offer to create one with [`Application::create`](crate::Application::create).
    #[error("'{}' has no project.toml, so it is not a project yet", .0.display())]
    NoProject(PathBuf),
    /// A file could not be read or written.
    #[error("cannot access '{}': {source}", path.display())]
    Io {
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The worker pool could not be started.
    #[error("cannot start the worker pool: {0}")]
    WorkerPool(String),
    /// A project edit or check failed.
    #[error(transparent)]
    Project(#[from] openchuck_project::ProjectError),
    /// A model file could not be imported.
    #[error(transparent)]
    Import(#[from] openchuck_import::ImportError),
    /// A model file cannot be referenced from the project folder.
    #[error("'{}' cannot be referenced from the project folder: {reason}", path.display())]
    ModelPath {
        /// The model file.
        path: PathBuf,
        /// Why not.
        reason: String,
    },
    /// A linked model cannot be copied into the project folder (decision
    /// record 0307 item 5); the message says why and what to do.
    #[error("{0}")]
    ModelCopy(String),
    /// More than one model has this recorded name.
    #[error("more than one model is called '{0}'; give its ID or its path as the project lists it")]
    AmbiguousModel(String),
    /// No model has this ID or path.
    #[error("the project has no model '{0}' (give its ID or its path as the project lists it)")]
    UnknownModel(String),
    /// No setup has this ID or name.
    #[error("the project has no setup '{0}' (give its ID or its name)")]
    UnknownSetup(String),
    /// The project has its one setup; more are SET-07 (P2).
    #[error("more than one setup per project is not supported yet")]
    OneSetupOnly,
    /// A setup needs a part, so it cannot be added before a model is imported.
    #[error("import a model first: a setup needs a part")]
    NoModelForSetup,
    /// No operation has this ID.
    #[error("the project has no operation '{0}'")]
    UnknownOperation(String),
    /// No selection set has this ID.
    #[error("the project has no selection set '{0}'")]
    UnknownSelection(String),
    /// Playback cannot be set up as asked (SIM-02, SIM-04).
    #[error("playback: {0}")]
    Playback(String),
    /// The part's model file is not the one the simulation was made with, so
    /// its deviation colours cannot be given to the file's triangles
    /// (decision 0492). The text names the model.
    #[error("the part '{0}' has changed since this simulation")]
    PartChanged(String),
    /// A selection cannot be made or changed as asked (UI-04, UI-05).
    #[error("selection '{name}': {reason}")]
    Selection {
        /// The set's name.
        name: String,
        /// Why not.
        reason: String,
    },
    /// An operation cannot be generated, refused by a check that names its
    /// topic, in the one form of decision record 0536 section 3: "<Topic>
    /// (operation '<name>'): <text>." Every refusal to generate takes it
    /// (decision record 0644).
    #[error("{}", refusal_display(topic, name, text))]
    Refused {
        /// The check's topic.
        topic: String,
        /// The operation's name.
        name: String,
        /// What is wrong, why, and what to change.
        text: String,
        /// The key of the operation's setting to change, if one does: Show
        /// setting opens it (decision record 0536 section 4).
        setting: Option<String>,
    },
    /// An operation was stopped before it would pass the memory a toolpath
    /// job may use (issue #847): nothing was made for it. Its words name
    /// where the limit is set and, where it was raised, the operation's
    /// Adaptive work limit as the likely cause (the owner's wording).
    #[error("{}", memory_stop_words(name, *reached, *limit, *work_limit, *memory))]
    MemoryStop {
        /// The operation's name.
        name: String,
        /// What the job had accounted for when it stopped, bytes.
        reached: u64,
        /// The job's limit, bytes.
        limit: u64,
        /// The operation's Adaptive work limit, geometry queries.
        work_limit: u64,
        /// The jobs' memory limit as the host set it, when it says.
        memory: Option<crate::memory::JobMemory>,
    },
    /// The tool numbers cannot be used as asked (GC-14).
    #[error("tool numbers: {0}")]
    ToolNumbers(String),
    /// The files were made before the project last changed, so they are not
    /// written (decision 0189).
    #[error(
        "the project changed after the files were made, so they must be made again before writing"
    )]
    OutOfDate,
    /// The toolpath could not be posted to G-code.
    #[error("cannot post the G-code: {0}")]
    Post(String),

    /// A file name template cannot name the files, or two names would be the
    /// same file (issues 308 to 310). The text is a sentence for the user.
    #[error("{0}")]
    FileNames(String),
    /// The pathing library reported an error.
    #[error(transparent)]
    Path(#[from] openchuck_path::PathError),
    /// An engine library reported an error.
    #[error(transparent)]
    Engine(#[from] openchuck_types::Error),
    /// The pipeline cache could not key an operation's inputs.
    #[error("pipeline cache: {0}")]
    Cache(String),
    /// The job hit an internal error and stopped. This is a defect; please report it.
    #[error("the job '{0}' stopped on an internal error; please report this as a defect")]
    JobPanicked(String),
    /// The job stopped without a result.
    #[error("the job '{0}' stopped without a result")]
    JobLost(String),
}

impl AppError {
    /// Whether the job was cancelled on request (UI-09), as opposed to
    /// failing. Decided by the kind of error, never by its words: an
    /// operation's name appears in messages and is the user's to choose.
    pub fn is_cancelled(&self) -> bool {
        matches!(
            self,
            Self::Engine(openchuck_types::Error::Cancelled)
                | Self::Path(openchuck_path::PathError::Cancelled)
                | Self::Import(openchuck_import::ImportError::Cancelled)
        )
    }

    /// The refusal of `operation` by the check `topic`, for `text`, with no
    /// setting to show (decision record 0644).
    pub fn refused(
        topic: impl Into<String>,
        operation: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self::Refused {
            topic: topic.into(),
            name: operation.into(),
            text: text.into(),
            setting: None,
        }
    }

    /// As it is, with the jobs' memory limit `memory` in a memory stop, so
    /// its words say where the limit is set.
    #[must_use]
    pub fn with_memory(self, memory: Option<crate::memory::JobMemory>) -> Self {
        match self {
            Self::MemoryStop {
                name,
                reached,
                limit,
                work_limit,
                ..
            } => Self::MemoryStop {
                name,
                reached,
                limit,
                work_limit,
                memory,
            },
            other => other,
        }
    }

    /// As it is, with Show setting opening the operation's setting `key`
    /// when it is a refusal.
    #[must_use]
    pub fn with_setting(self, key: impl Into<String>) -> Self {
        match self {
            Self::Refused {
                topic, name, text, ..
            } => Self::Refused {
                topic,
                name,
                text,
                setting: Some(key.into()),
            },
            other => other,
        }
    }

    /// The error in plain words for a panel (specification section 15): a
    /// refused parameter value says what is wrong with it, without the
    /// parameter's name in the strategy's schema, which the panel shows
    /// beside the field already.
    pub fn user_message(&self) -> String {
        match self {
            Self::Path(openchuck_path::PathError::Parameter { reason, .. }) => {
                let mut chars = reason.chars();
                let mut out: String = chars
                    .next()
                    .map(|c| c.to_uppercase().chain(chars).collect())
                    .unwrap_or_default();
                if !out.ends_with(['.', '!', '?']) {
                    out.push('.');
                }
                out
            }
            other => other.to_string(),
        }
    }
}

/// A refusal in the one form, naming the operation when it has a name (an
/// operation refused for having none is named by nothing).
fn refusal_display(topic: &str, name: &str, text: &str) -> String {
    let context = if name.is_empty() {
        crate::lead_in::Context::default()
    } else {
        crate::lead_in::Context::operation(name)
    };
    crate::lead_in::in_form(topic, &context, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn stop(work_limit: u64, origin: crate::memory::MemoryOrigin) -> String {
        AppError::MemoryStop {
            name: "Operation 1".into(),
            reached: 31 * GIB,
            limit: 32 * GIB,
            work_limit,
            memory: None,
        }
        .with_memory(Some(crate::memory::JobMemory {
            percent: 50,
            bytes: 32 * GIB,
            origin,
        }))
        .to_string()
    }

    /// Issue #847: the owner's approved words for a memory stop, where the
    /// Adaptive work limit was raised and where it is at its default, as
    /// Settings and as the command line set the limit.
    #[test]
    fn req_nfr_07_req_nfr_12_a_memory_stop_says_what_happened_and_what_to_change() {
        use crate::memory::MemoryOrigin::{CommandLine, Settings};
        assert_eq!(
            stop(500_000_000, Settings),
            "Operation 'Operation 1' was stopped: it would need more memory than toolpath jobs \
             may use on this computer (it had reached about 31 GB; the limit is 32 GB, 50 % of \
             this computer's memory, set in Settings, 'Memory a toolpath job may use'). No \
             toolpath or G-code was made for this operation, and the project is unchanged. \
             Adaptive work limit (Advanced) is 500,000,000, above its default of 1,000,000, and \
             is the likely cause: lower it, or use a larger Allowed excess, a larger Smallest \
             spacing or a smaller Stepover. You can also allow jobs more memory in Settings."
        );
        assert_eq!(
            stop(1_000_000, Settings),
            "Operation 'Operation 1' was stopped: it would need more memory than toolpath jobs \
             may use on this computer (it had reached about 31 GB; the limit is 32 GB, 50 % of \
             this computer's memory, set in Settings, 'Memory a toolpath job may use'). No \
             toolpath or G-code was made for this operation, and the project is unchanged. \
             Adaptive work limit is at its default; this part needs more passes than fit in the \
             memory allowed: use a larger Allowed excess, a larger Smallest spacing or a smaller \
             Stepover, limit the operation to a smaller selection, close other programs, or \
             allow jobs more memory in Settings ('Memory a toolpath job may use')."
        );
        let raised = stop(500_000_000, CommandLine);
        assert!(
            raised.contains("50 % of this computer's memory, set with --job-memory)")
                && raised.ends_with("You can also allow jobs more memory with --job-memory."),
            "{raised}"
        );
        let default = stop(1_000_000, CommandLine);
        assert!(
            default.ends_with("close other programs, or allow jobs more memory with --job-memory."),
            "{default}"
        );
    }

    /// Specification section 15: no schema field names in what a panel reads.
    #[test]
    fn req_nfr_15_a_refused_parameter_reads_as_a_sentence() {
        let error = AppError::from(openchuck_path::PathError::parameter(
            "stepover_mm",
            "must be above 0 mm",
        ));
        assert_eq!(error.user_message(), "Must be above 0 mm.");
        let cancelled = AppError::Path(openchuck_path::PathError::Cancelled);
        assert_eq!(cancelled.user_message(), cancelled.to_string());
    }
}

/// The words of a memory stop (issue #847, the owner's approved wording).
fn memory_stop_words(
    name: &str,
    reached: u64,
    limit: u64,
    work_limit: u64,
    memory: Option<crate::memory::JobMemory>,
) -> String {
    use crate::memory::{MemoryOrigin, size_words};
    let origin = memory.map(|m| m.origin);
    let set = match memory {
        Some(m) => match m.origin {
            MemoryOrigin::Settings => format!(
                ", {} % of this computer's memory, set in Settings, 'Memory a toolpath job may \
                 use'",
                m.percent
            ),
            MemoryOrigin::CommandLine => format!(
                ", {} % of this computer's memory, set with --job-memory",
                m.percent
            ),
        },
        None => String::new(),
    };
    let head = format!(
        "Operation '{name}' was stopped: it would need more memory than toolpath jobs may use \
         on this computer (it had reached about {}; the limit is {}{set}). No toolpath or \
         G-code was made for this operation, and the project is unchanged.",
        size_words(reached),
        size_words(limit)
    );
    let default = openchuck_path::adaptive::DEFAULT_WORK_LIMIT;
    if work_limit > default {
        let more = match origin {
            Some(MemoryOrigin::CommandLine) => {
                " You can also allow jobs more memory with --job-memory."
            }
            _ => " You can also allow jobs more memory in Settings.",
        };
        format!(
            "{head} Adaptive work limit (Advanced) is {}, above its default of {}, and is the \
             likely cause: lower it, or use a larger Allowed excess, a larger Smallest spacing or \
             a smaller Stepover.{more}",
            thousands(work_limit),
            thousands(default)
        )
    } else {
        let more = match origin {
            Some(MemoryOrigin::CommandLine) => "allow jobs more memory with --job-memory",
            _ => "allow jobs more memory in Settings ('Memory a toolpath job may use')",
        };
        format!(
            "{head} Adaptive work limit is at its default; this part needs more passes than fit \
             in the memory allowed: use a larger Allowed excess, a larger Smallest spacing or a \
             smaller Stepover, limit the operation to a smaller selection, close other programs, \
             or {more}."
        )
    }
}

/// `n` with a comma between each three digits.
fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}
