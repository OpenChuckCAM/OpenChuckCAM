// SPDX-License-Identifier: AGPL-3.0-or-later
//! Repository automation (architecture section 12, work breakdown slice 0 item 6).
//!
//! Run as `cargo xtask <check>`:
//!
//! - `deps`: dependency rules from architecture sections 4 and 12 (NFR-01, NFR-09, INT-01).
//! - `licences`: SPDX headers on source files and the licence field of every crate (NFR-10).
//! - `records`: every third-party dependency is pinned, declared once in the
//!   workspace and recorded in `docs/dependencies.md` (NFR-14); the open-items register and
//!   the decision index have no duplicate, misplaced or out-of-order rows (decision 0130).
//! - `trace`: the traceability report from requirement-ID test names (NFR-08).
//! - `testdata`: the test-data size policy.
//! - `conflicts`: no unresolved merge conflict markers in tracked files.
//! - `publish`: the library family's manifests are ready for crates.io (decision 0138, OI-302).
//! - `coverage`: line coverage per crate from an lcov file against the target (not part of `all`).
//! - `all`: every check above.
//!
//! - `icon`: writes the application icon's PNG sizes, `.ico` and `.res` from its SVG;
//!   `icon --check` reports committed files that differ (decision 0542; not part of `all`).
//! - `golden`: the golden file audit, which checks the committed G-code against the exact parts.
//! - `replay`: gate 3, LinuxCNC programs in the headless simulator (needs LinuxCNC; not part of `all`).
//! - `release-gate`: the release gate (slice 20 item 9): `deps`, `cargo deny`, the golden G-code
//!   comparison and `replay`, all of which must pass (needs LinuxCNC, cargo-deny and the STEP
//!   kernel's toolchain; not part of `all`).
//! - `macos-app`: the macOS application bundle `OpenChuckCAM.app` (decision 0541; not part of `all`).
//! - `examples-zip`: the zip of the example projects for the nightly build and the release (lane
//!   EXAMPLEZIP; not part of `all`).
//! - `changelog`: the changelog since the last release (decision record CHANGELOG-FLOW-A): the
//!   nightly notes, the pull request check, and the release step (not part of `all`).
//! - `glibc-floor`: the Linux binaries need no newer glibc or libstdc++ than the stated floor
//!   (decision 0541; not part of `all`).

#![allow(clippy::print_stdout, clippy::print_stderr)]

mod changelog;
mod conflicts;
mod coverage;
mod deps;
mod examples_zip;
mod glibc_floor;
mod golden;
mod icon;
mod licences;
mod macos_app;
mod messages;
mod publish;
mod records;
mod registers;
mod release_gate;
mod replay;
mod testdata;
mod topics;
mod trace;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "xtask", about = "OpenChuckCAM repository checks")]
struct Cli {
    #[command(subcommand)]
    check: Check,
}

#[derive(Debug, Subcommand)]
enum Check {
    /// Dependency rules (architecture sections 4 and 12), and the message
    /// rules (decision record 0536), which CI runs with them.
    Deps,
    /// The message rules alone: messages built outside `Message::about`, and
    /// words that claim safety (decision record 0536 item 6.3; NFR-12).
    Messages,
    /// SPDX headers and crate licence fields.
    Licences,
    /// Dependency pins, single declaration and NFR-14 records.
    Records,
    /// Traceability report from requirement-ID test names.
    Trace {
        /// Read test names from this file (one per line) instead of running `cargo test -- --list`.
        #[arg(long)]
        tests_from: Option<PathBuf>,
        /// Write the Markdown report here.
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Test-data size policy.
    Testdata,
    /// Unresolved merge conflict markers in tracked files.
    Conflicts,
    /// The library family's manifests are ready for crates.io: metadata, `include` lists,
    /// embedded files inside the crate, and no unpublishable dependency kept in a manifest.
    Publish,
    /// Line coverage per crate from an lcov file against the target in `xtask/coverage.toml`.
    Coverage {
        /// The lcov file (default `lcov.info` at the workspace root).
        lcov: Option<PathBuf>,
        /// Write the Markdown table here (default `target/coverage-summary.md`).
        #[arg(long)]
        report: Option<PathBuf>,
        /// Fail when a gated crate is under its threshold (ignored while `enforce = false`
        /// in `xtask/coverage.toml`).
        #[arg(long)]
        enforce: bool,
    },
    /// Audit the golden G-code files against the exact parts, the post conventions and the
    /// controller's feed rule, so they need no reading by hand.
    Golden,
    /// Gate 3: run LinuxCNC programs in the simulator and compare with the replay.
    /// With no program, every LinuxCNC golden program is run. Needs LinuxCNC:
    /// `--docker IMAGE`, `LINUXCNC_RIP`, or `linuxcnc` on the PATH.
    Replay {
        /// Programs to run (LinuxCNC dialect).
        programs: Vec<PathBuf>,
        /// Run LinuxCNC in this Docker image (see docker/linuxcnc).
        #[arg(long)]
        docker: Option<String>,
        /// Keep each simulator report in this folder.
        #[arg(long)]
        keep_reports: Option<PathBuf>,
        /// Largest per-line difference allowed, mm or degrees.
        #[arg(long, default_value_t = replay::TOLERANCE)]
        tolerance: f64,
        /// Give up on one program after this many seconds.
        #[arg(long, default_value_t = replay::TIMEOUT_SECONDS)]
        timeout: u64,
        /// Run only one share of the programs, `I/N` (for example `2/3`): the programs are
        /// dealt over N runners by expected run time, so every program belongs to exactly one
        /// share. CI runs one share on each of N runners.
        #[arg(long, value_name = "I/N")]
        shard: Option<String>,
    },
    /// The release gate: dependency rules, `cargo deny`, the golden G-code comparison and the
    /// LinuxCNC replay of every golden program. Fails if any of them fails; none can be skipped.
    /// Needs LinuxCNC (as for `replay`), cargo-deny and the STEP kernel's toolchain.
    ReleaseGate {
        /// Run LinuxCNC in this Docker image (see docker/linuxcnc).
        #[arg(long)]
        docker: Option<String>,
        /// Keep each simulator report in this folder.
        #[arg(long)]
        keep_reports: Option<PathBuf>,
        /// The tag being released (`lib-v<version>` or `app-v<version>`): it must name the
        /// workspace version.
        #[arg(long)]
        tag: Option<String>,
    },
    /// The application icon (decision 0542): render `apps/openchuck-ui/assets/icon/icon.svg`
    /// to the committed PNG sizes and write `icon.ico` and `icon.res` from them.
    Icon {
        /// Write nothing; report each committed file that differs from what the SVG gives.
        #[arg(long)]
        check: bool,
    },
    /// Build the macOS application bundle `OpenChuckCAM.app` from the Info.plist template, with
    /// `openchuck-ui` and the STEP reader inside it. Not signed or notarised.
    MacosApp {
        /// The `openchuck-ui` executable.
        #[arg(long)]
        ui: PathBuf,
        /// The `openchuck-step-worker` executable.
        #[arg(long)]
        worker: PathBuf,
        /// The folder to write `OpenChuckCAM.app` into (an existing bundle there is replaced).
        #[arg(long)]
        out: PathBuf,
        /// The bundle version (default: the workspace version).
        #[arg(long)]
        version: Option<String>,
        /// Modification time of every file in the bundle, seconds since 1970 (default:
        /// `SOURCE_DATE_EPOCH`).
        #[arg(long)]
        mtime: Option<u64>,
        /// A 512 by 512 PNG for the icon (default: no icon).
        #[arg(long)]
        icon: Option<PathBuf>,
    },
    /// Fail if a Linux binary needs a newer symbol version than the floor of its family.
    GlibcFloor {
        /// A floor, `FAMILY=version`, for example `GLIBC=2.35` (repeat for each family).
        #[arg(long = "floor", required = true)]
        floors: Vec<String>,
        /// The binaries to check.
        #[arg(required = true)]
        binaries: Vec<PathBuf>,
    },
    /// Zip the example projects under `testdata/examples/` for the nightly build and the release
    /// (lane EXAMPLEZIP): sorted, fixed times, stored; fails on anything outside the examples
    /// folder or a file type not on the allow-list. Not part of `all`.
    ExamplesZip {
        /// The folder every entry sits in, also the zip's base name (for example
        /// `openchuckcam-examples-2026.1.0`).
        #[arg(long)]
        name: String,
        /// Write the zip here.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Only check this finished zip (the same checks as on writing).
        #[arg(long, conflicts_with = "out")]
        check: Option<PathBuf>,
    },
    /// The changelog since the last release: the *Development version* section of
    /// `user-docs/src/release-notes.md` (decision record CHANGELOG-FLOW-A).
    Changelog {
        #[command(subcommand)]
        command: ChangelogCommand,
    },
    /// Every check.
    All,
}

#[derive(Debug, Subcommand)]
enum ChangelogCommand {
    /// Write the notes of the nightly pre-release from the release notes at this commit.
    NightlyNotes {
        /// The built commit.
        #[arg(long)]
        sha: String,
        /// The build date (YYYYMMDD).
        #[arg(long)]
        date: String,
        /// The repository, `OWNER/NAME`, for the links to the library changelogs.
        #[arg(long, default_value = "OpenChuckCAM/OpenChuckCAM")]
        repo: String,
        /// Write the notes here (default: standard output).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Check a pull request: a Development version entry (and a library crate's CHANGELOG.md),
    /// or a line "No user-visible change: <reason>" in its description, read from `$PR_BODY`.
    CheckPr {
        /// The pull request's base commit.
        #[arg(long)]
        base: String,
        /// The pull request's head.
        #[arg(long, default_value = "HEAD")]
        head: String,
    },
    /// Move the Development version entries into a new draft section for VERSION.
    StartRelease {
        /// The version being released (X.Y.Z).
        version: String,
    },
    /// The release-notes checks of a release: the version's section is written and the
    /// Development version is empty.
    CheckRelease {
        /// The version being released (X.Y.Z).
        version: String,
    },
}

/// Findings from one check: empty means it passed.
pub type Findings = Vec<String>;

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<bool> {
    let cli = Cli::parse();
    let root = workspace_root()?;
    let checks: Vec<(&str, Findings)> = match cli.check {
        Check::Deps => vec![
            ("deps", deps::check(&root)?),
            ("messages", messages::check(&root)?),
            ("topics", topics::check(&root)?),
        ],
        Check::Messages => vec![("messages", messages::check(&root)?)],
        Check::Licences => vec![("licences", licences::check(&root)?)],
        Check::Records => vec![("records", records::check(&root)?)],
        Check::Trace { tests_from, report } => {
            vec![(
                "trace",
                trace::check(&root, tests_from.as_deref(), report.as_deref())?,
            )]
        }
        Check::Testdata => vec![("testdata", testdata::check(&root)?)],
        Check::Conflicts => vec![("conflicts", conflicts::check(&root)?)],
        Check::Publish => vec![("publish", publish::check(&root)?)],
        Check::Coverage {
            lcov,
            report,
            enforce,
        } => vec![(
            "coverage",
            coverage::check(&root, lcov.as_deref(), report.as_deref(), enforce)?,
        )],
        Check::Golden => vec![("golden", golden::check(&root)?)],
        Check::Icon { check: true } => vec![("icon", icon::check(&root)?)],
        Check::Icon { check: false } => {
            icon::write(&root)?;
            return Ok(true);
        }
        Check::Replay {
            programs,
            docker,
            keep_reports,
            tolerance,
            timeout,
            shard,
        } => vec![(
            "replay",
            replay::check(
                &root,
                &replay::Options {
                    programs,
                    docker,
                    keep_reports,
                    tolerance,
                    timeout_seconds: timeout,
                    shard,
                },
            )?,
        )],
        Check::ReleaseGate {
            docker,
            keep_reports,
            tag,
        } => release_gate::check(
            &root,
            &release_gate::Options {
                docker,
                keep_reports,
                tag,
            },
        ),
        Check::MacosApp {
            ui,
            worker,
            out,
            version,
            mtime,
            icon,
        } => {
            let mtime = match mtime {
                Some(m) => m,
                None => std::env::var("SOURCE_DATE_EPOCH")
                    .context("give --mtime or set SOURCE_DATE_EPOCH (the commit time)")?
                    .parse()
                    .context("SOURCE_DATE_EPOCH is not a number of seconds")?,
            };
            let bundle = macos_app::build(
                &root,
                &macos_app::Options {
                    ui,
                    worker,
                    out,
                    version,
                    mtime,
                    icon,
                },
            )?;
            println!("wrote {}", bundle.display());
            vec![]
        }
        Check::ExamplesZip { name, out, check } => {
            examples_zip::run(&root, &name, out.as_ref(), check.as_ref())?;
            vec![]
        }
        Check::Changelog { command } => match command {
            ChangelogCommand::NightlyNotes {
                sha,
                date,
                repo,
                out,
            } => {
                let notes =
                    changelog::write_nightly_notes(&root, &sha, &date, &repo, out.as_deref())?;
                if out.is_none() {
                    print!("{notes}");
                }
                return Ok(true);
            }
            ChangelogCommand::CheckPr { base, head } => {
                let body = std::env::var("PR_BODY").unwrap_or_default();
                let findings = changelog::check_pr(&root, &base, &head, &body)?;
                if !findings.is_empty() {
                    println!(
                        "::error title=Changelog::This pull request needs a changelog entry or a reason why not (see below)."
                    );
                }
                vec![("changelog", findings)]
            }
            ChangelogCommand::StartRelease { version } => {
                let moved = changelog::start_release_file(&root, &version)?;
                println!(
                    "Moved {} into a draft section for {version} in {}. Write the section, then delete its draft comment.",
                    changelog::entries_in_words(moved),
                    changelog::NOTES_PATH
                );
                return Ok(true);
            }
            ChangelogCommand::CheckRelease { version } => {
                vec![("changelog", changelog::check_release(&root, &version)?)]
            }
        },
        Check::GlibcFloor { floors, binaries } => {
            vec![("glibc-floor", glibc_floor::check(&binaries, &floors)?)]
        }
        Check::All => vec![
            ("deps", deps::check(&root)?),
            ("messages", messages::check(&root)?),
            ("topics", topics::check(&root)?),
            ("licences", licences::check(&root)?),
            ("records", records::check(&root)?),
            ("testdata", testdata::check(&root)?),
            ("conflicts", conflicts::check(&root)?),
            ("publish", publish::check(&root)?),
            ("trace", trace::check(&root, None, None)?),
        ],
    };
    let mut ok = true;
    for (name, findings) in checks {
        if findings.is_empty() {
            println!("{name}: ok");
        } else {
            ok = false;
            println!("{name}: {} problem(s)", findings.len());
            for finding in findings {
                println!("  - {finding}");
            }
        }
    }
    Ok(ok)
}

fn workspace_root() -> Result<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map(Path::to_path_buf)
        .context("xtask must live one level below the workspace root")
}
