// SPDX-License-Identifier: AGPL-3.0-or-later
//! `cargo xtask golden`: the golden file audit (work breakdown slice 6 review
//! checklist; decision record 0020 item 14).
//!
//! Runs the `golden_audit` module of the application's integration test binary
//! (`crates/openchuck-app/tests/golden_audit.rs`, folded into `tests/integration.rs`
//! by decision 0516), which reads the committed
//! G-code files and checks them against things that do not come from the post
//! or the path library: the post conventions, the static checks, the exact
//! analytic parts (through the independent kinematic oracle and gouge
//! checker), the contact feed under the controller's rule, and agreement
//! between the Z1 and LinuxCNC programs. It also runs with the ordinary test
//! suite; this command prints what it measured.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::Findings;

/// The application test target that holds the audit (decision 0516: one integration binary).
const TARGET: &str = "integration";

/// The audit's module in [`TARGET`]; its tests are named `golden_audit::...`.
const MODULE: &str = "golden_audit";

/// The `cargo test` arguments of the audit: the audit module's tests in the integration binary.
fn audit_args() -> Vec<String> {
    [
        "test",
        "--quiet",
        "-p",
        "openchuck-app",
        "--test",
        TARGET,
        "--",
        &format!("{MODULE}::"),
        "--nocapture",
        "--test-threads=1",
    ]
    .iter()
    .map(|a| (*a).to_string())
    .collect()
}

/// Runs the audit and returns a finding if it fails.
pub fn check(root: &Path) -> Result<Findings> {
    let output = Command::new("cargo")
        .args(audit_args())
        .current_dir(root)
        .output()
        .context("cannot run cargo test")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout
        .lines()
        .filter(|l| l.starts_with("  ") || l.contains(".nc") || l.contains(".ngc"))
    {
        println!("{line}");
    }
    if output.status.success() {
        return Ok(Vec::new());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let why: Vec<&str> = stderr
        .lines()
        .chain(stdout.lines())
        .filter(|l| {
            l.contains("panicked at")
                || (!l.starts_with(' ') && l.contains(".nc") || l.contains(".ngc:"))
        })
        .take(6)
        .collect();
    Ok(vec![format!(
        "the golden audit failed: {}",
        why.join(" | ")
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn app() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .join("crates/openchuck-app")
    }

    /// Issue #882: the command named a test target that decision 0516 folded away. The target it
    /// names must be one the application's manifest declares, and the module it filters on must
    /// be a module of that target.
    #[test]
    fn golden_audit_runs_a_module_of_a_declared_test_target() {
        let args = audit_args();
        let at = args.iter().position(|a| a == "--test").expect("--test");
        let target = args.get(at + 1).expect("a target");
        let manifest: toml::Table = std::fs::read_to_string(app().join("Cargo.toml"))
            .expect("manifest")
            .parse()
            .expect("toml");
        let path = manifest
            .get("test")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
            .find(|t| t.get("name").and_then(toml::Value::as_str) == Some(target.as_str()))
            .and_then(|t| t.get("path").and_then(toml::Value::as_str))
            .unwrap_or_else(|| panic!("the manifest declares no test target {target}"));
        let source = std::fs::read_to_string(app().join(path)).expect("target source");
        assert!(
            source.contains(&format!("mod {MODULE};")),
            "{path} has no module {MODULE}"
        );
        assert!(
            args.iter().any(|a| a == &format!("{MODULE}::")),
            "the filter is not the module: {args:?}"
        );
    }
}
