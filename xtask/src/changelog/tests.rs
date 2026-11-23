// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests of the changelog flow (decision record CHANGELOG-FLOW-A; NFR-12; NFR-19; INT-07; decision
//! 0400 section R). Each test is named after the record (`changelog_flow`) or the requirement.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;

const NOTICE: &str = "Generated G-code must be verified by the user before it is run on a machine. \
OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.";
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
const REPO: &str = "OpenChuckCAM/OpenChuckCAM";
const INTRO: &str = "The *latest* version of this guide describes the nightly build, the \
[development build](start/install.md#download-a-development-build) made each night from the main branch.";
const TAIL: &str = "Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM \
does not guarantee that a program is safe or correct for any machine, tool or setup.";

/// A release-notes page shaped like `user-docs/src/release-notes.md`.
fn notes_with(dev: &[&str], releases: &[&str]) -> String {
    let mut out: Vec<String> = vec![
        "# Release notes".into(),
        String::new(),
        "What changed.".into(),
        String::new(),
    ];
    for v in releases {
        out.extend([
            format!("## {v}"),
            String::new(),
            format!("The {v} release."),
            String::new(),
            "### Known limitations".into(),
            String::new(),
            "- **Old.** An old limitation.".into(),
            String::new(),
            TAIL.into(),
            String::new(),
        ]);
    }
    out.extend([
        "## Development version".into(),
        String::new(),
        INTRO.into(),
        String::new(),
    ]);
    out.extend(dev.iter().map(|e| (*e).to_string()));
    if !dev.is_empty() {
        out.push(String::new());
    }
    out.extend([TAIL.into(), String::new()]);
    out.join("\n")
}

fn notes(dev: &[&str]) -> String {
    notes_with(dev, &["0.55.0"])
}

fn files(list: &[&str]) -> Vec<String> {
    list.iter().map(|f| (*f).to_string()).collect()
}

// ------------------------------------------------------------------------------- the entries

#[test]
fn changelog_flow_entries_are_the_bullets_of_the_development_section() {
    let text = notes(&[
        "- **Keyboard.** Tab follows reading order.",
        "- **Facing.** Works on a STEP part.",
    ]);
    assert_eq!(
        development_entries(&text),
        vec![
            "- **Keyboard.** Tab follows reading order.",
            "- **Facing.** Works on a STEP part."
        ]
    );
}

#[test]
fn changelog_flow_bullets_of_released_sections_are_not_entries() {
    assert!(development_entries(&notes(&[])).is_empty());
}

#[test]
fn changelog_flow_an_entry_keeps_its_continuation_lines_and_sub_bullets() {
    let text = notes(&[
        "- **Facing.** Works on a STEP part,",
        "  bounded from its exact faces.",
        "  - Flat and round shapes.",
        "- **Two.** Second.",
    ]);
    let entries = development_entries(&text);
    assert_eq!(entries.len(), 2);
    assert!(entries[0].contains("bounded from its exact faces."));
    assert!(entries[0].contains("  - Flat and round shapes."));
}

#[test]
fn changelog_flow_an_indented_paragraph_after_a_blank_line_stays_in_its_entry() {
    let text = notes(&["- **A.** One.", "", "  More about A.", "- **B.** Two."]);
    let entries = development_entries(&text);
    assert_eq!(entries.len(), 2);
    assert!(entries[0].ends_with("  More about A."));
}

#[test]
fn changelog_flow_no_development_section_means_no_entries() {
    assert!(development_entries("# Release notes\n\n## 0.55.0\n\n- a\n").is_empty());
}

#[test]
fn changelog_flow_latest_release_is_the_highest_version_heading() {
    assert_eq!(
        latest_release(&notes_with(&[], &["0.60.0", "0.55.0"])).as_deref(),
        Some("0.60.0")
    );
    assert_eq!(
        latest_release(&notes_with(&[], &["0.9.0", "0.10.0"])).as_deref(),
        Some("0.10.0")
    );
    assert_eq!(latest_release(&notes_with(&[], &[])), None);
}

// --------------------------------------------------------------------------- nightly notes

fn make(text: &str) -> String {
    nightly_notes(text, NOTICE, SHA, "20261123", REPO, GUIDE, NOTES_LIMIT).expect("notes")
}

#[test]
fn changelog_flow_notes_list_the_development_entries_since_the_last_release() {
    let out = make(&notes(&["- **Keyboard.** Tab follows reading order."]));
    assert!(out.contains("since the last release, 0.55.0"), "{out}");
    assert!(out.contains("- **Keyboard.** Tab follows reading order."));
    assert!(out.contains(&format!("`main` at {SHA}")));
    assert!(out.contains("https://openchuck.cam/latest/release-notes.html#development-version"));
}

#[test]
fn req_nfr_12_notes_carry_the_gcode_notice_and_say_it_is_not_a_release() {
    let out = make(&notes(&["- **A.** One."]));
    assert!(out.contains(&format!("**{NOTICE}**")));
    assert!(out.contains("**This is not a release.**"));
    let rest = out.to_lowercase().replace(&NOTICE.to_lowercase(), "");
    for phrase in ["safe to run", "ready to run", "verified safe", "guaranteed"] {
        assert!(!rest.contains(phrase), "{phrase}");
    }
}

#[test]
fn changelog_flow_an_empty_section_says_nothing_user_visible_changed() {
    let out = make(&notes(&[]));
    assert!(
        out.contains("Nothing user-visible has changed since the last release, 0.55.0."),
        "{out}"
    );
    assert!(!out.contains("\n\n\n"));
}

#[test]
fn changelog_flow_no_release_yet_says_so() {
    let out = make(&notes_with(&["- **A.** One."], &[]));
    assert!(out.contains("There has been no release yet"));
    assert!(out.contains("- **A.** One."));
}

#[test]
fn changelog_flow_relative_guide_links_point_at_the_latest_guide() {
    let out = make(&notes(&[
        "- **Keys.** See [Keyboard](reference/keyboard.md) and \
         [install](start/install.md#download-a-development-build), \
         [issue](https://github.com/x/y/issues/1), [up](./a/../b/c.md) and [here](#development-version).",
    ]));
    assert!(
        out.contains("[Keyboard](https://openchuck.cam/latest/reference/keyboard.html)"),
        "{out}"
    );
    assert!(out.contains(
        "(https://openchuck.cam/latest/start/install.html#download-a-development-build)"
    ));
    assert!(out.contains("[issue](https://github.com/x/y/issues/1)"));
    assert!(out.contains("[up](https://openchuck.cam/latest/b/c.html)"));
    assert!(
        out.contains("[here](https://openchuck.cam/latest/release-notes.html#development-version)")
    );
}

#[test]
fn changelog_flow_issue_references_are_kept() {
    assert!(make(&notes(&["- **Keys.** Fixed (#577, #578)."])).contains("(#577, #578)"));
}

#[test]
fn changelog_flow_library_changelogs_are_linked_at_the_built_commit() {
    let out = make(&notes(&["- **A.** One."]));
    for c in [
        "openchuck-types",
        "openchuck-path",
        "openchuck-post",
        "openchuck-gcode",
    ] {
        assert!(out.contains(&format!(
            "https://github.com/{REPO}/blob/{SHA}/crates/{c}/CHANGELOG.md"
        )));
    }
}

#[test]
fn changelog_flow_notes_are_bounded_and_say_where_the_rest_is() {
    let entries: Vec<String> = (0..200)
        .map(|i| format!("- **Entry {i}.** {}", "words ".repeat(40)))
        .collect();
    let refs: Vec<&str> = entries.iter().map(String::as_str).collect();
    let out =
        nightly_notes(&notes(&refs), NOTICE, SHA, "20261123", REPO, GUIDE, 6000).expect("notes");
    assert!(out.chars().count() <= 6000);
    assert!(out.contains("- **Entry 0.**"));
    assert!(!out.contains("- **Entry 199.**"));
    assert!(out.contains(" more changes are in the [release notes"));
}

#[test]
fn changelog_flow_a_limit_too_small_for_the_fixed_text_is_an_error() {
    assert!(nightly_notes(&notes(&[]), NOTICE, SHA, "d", REPO, GUIDE, 500).is_err());
}

#[test]
fn changelog_flow_notes_are_deterministic() {
    let text = notes(&["- **A.** One.", "- **B.** Two."]);
    assert_eq!(make(&text), make(&text));
}

#[test]
fn changelog_flow_the_default_limit_is_below_githubs() {
    const { assert!(NOTES_LIMIT <= 100_000) };
}

// --------------------------------------------------------------------- pull request check

/// The table of example diffs: (changed files, adds a Development entry, escape hatch, passes).
const DIFFS: &[(&[&str], bool, Option<&str>, bool)] = &[
    // Docs and registers only: never triggered.
    (
        &["docs/decisions/0700-x.md", "docs/open-items.md"],
        false,
        None,
        true,
    ),
    (&["AGENTS.md", "CLAUDE.md", "README.md"], false, None, true),
    (
        &["docs/releasing.md", ".github/workflows/ci.yml"],
        false,
        None,
        true,
    ),
    // A crate changelog alone (a lane's changelog-only edit): not triggered.
    (&["crates/openchuck-app/CHANGELOG.md"], false, None, true),
    (
        &[
            "crates/openchuck-path/CHANGELOG.md",
            "Cargo.toml",
            "Cargo.lock",
        ],
        false,
        None,
        true,
    ),
    // Tests, benches, examples, the test kit, xtask and testdata outside the examples.
    (
        &[
            "crates/openchuck-path/tests/foo.rs",
            "crates/openchuck-app/tests/integration.rs",
        ],
        false,
        None,
        true,
    ),
    (
        &[
            "crates/openchuck-testkit/src/lib.rs",
            "xtask/src/records.rs",
            "testdata/golden/a.ngc",
        ],
        false,
        None,
        true,
    ),
    (
        &[
            "crates/openchuck-path/benches/b.rs",
            "crates/openchuck-post/examples/e.rs",
        ],
        false,
        None,
        true,
    ),
    // The release notes alone (the thing being added).
    (&["user-docs/src/release-notes.md"], true, None, true),
    // The screenshot records are read by the checks, not shown to users.
    (
        &[
            "user-docs/src/tutorial/images/CAPTURED.toml",
            "user-docs/src/examples/images/flutes/CAPTURED.toml",
        ],
        false,
        None,
        true,
    ),
    // A new or retaken screenshot is a change to the guide.
    (
        &["user-docs/src/tutorial/images/step-1.png"],
        false,
        None,
        false,
    ),
    // The guide's tools and theme are not the guide's text.
    (
        &["user-docs/tools/check.py", "user-docs/theme/style.css"],
        false,
        None,
        true,
    ),
    // Application source and assets: triggered; an entry or the escape hatch passes.
    (&["apps/openchuck-ui/src/app.rs"], false, None, false),
    (
        &[
            "apps/openchuck-ui/src/app.rs",
            "user-docs/src/release-notes.md",
        ],
        true,
        None,
        true,
    ),
    (
        &["apps/openchuck-ui/src/app.rs"],
        false,
        Some("No user-visible change: internal refactor"),
        true,
    ),
    (&["apps/openchuck-ui/assets/icon.png"], false, None, false),
    (&["apps/openchuck-cli/src/main.rs"], false, None, false),
    // An application crate (not the library family): an entry is enough.
    (
        &[
            "crates/openchuck-app/src/lib.rs",
            "user-docs/src/release-notes.md",
        ],
        true,
        None,
        true,
    ),
    // A library-family crate: an entry and that crate's changelog.
    (
        &[
            "crates/openchuck-path/src/lib.rs",
            "user-docs/src/release-notes.md",
        ],
        true,
        None,
        false,
    ),
    (
        &[
            "crates/openchuck-path/src/lib.rs",
            "user-docs/src/release-notes.md",
            "crates/openchuck-path/CHANGELOG.md",
        ],
        true,
        None,
        true,
    ),
    (
        &[
            "crates/openchuck-path/src/lib.rs",
            "crates/openchuck-path/CHANGELOG.md",
        ],
        false,
        None,
        false,
    ),
    (
        &["crates/openchuck-path/src/lib.rs"],
        false,
        Some("No user-visible change: a private helper"),
        true,
    ),
    // The built-in posts belong to openchuck-post.
    (
        &[
            "crates/openchuck-post/posts/z1.toml",
            "user-docs/src/release-notes.md",
            "crates/openchuck-post/CHANGELOG.md",
        ],
        true,
        None,
        true,
    ),
    (
        &[
            "crates/openchuck-post/posts/z1.toml",
            "user-docs/src/release-notes.md",
        ],
        true,
        None,
        false,
    ),
    // The example projects (the examples download) and the guide's pages.
    (
        &["testdata/examples/baluster/project.ocp"],
        false,
        None,
        false,
    ),
    (&["user-docs/src/start/install.md"], false, None, false),
    (
        &["user-docs/src/start/install.md"],
        false,
        Some("No user-visible change: a typo in the guide"),
        true,
    ),
];

fn run_rule(changed: &[&str], adds_entry: bool, hatch: Option<&str>) -> Findings {
    let base = notes(&["- **Old.** Already there."]);
    let head = if adds_entry {
        notes(&["- **Old.** Already there.", "- **New.** Something new."])
    } else {
        base.clone()
    };
    let body = match hatch {
        Some(h) => format!("## Summary\n\nWork.\n\n{h}\n"),
        None => "## Summary\n\nWork.\n".to_string(),
    };
    pr_problems(&files(changed), &base, &head, &body)
}

#[test]
fn changelog_flow_the_table_of_example_diffs() {
    assert_eq!(DIFFS.len(), 27);
    for (changed, adds, hatch, ok) in DIFFS {
        let problems = run_rule(changed, *adds, *hatch);
        assert_eq!(
            problems.is_empty(),
            *ok,
            "{changed:?} {hatch:?}: {problems:?}"
        );
    }
}

#[test]
fn changelog_flow_a_changed_entry_counts_as_an_entry() {
    let base = notes(&["- **Facing.** Works on STL."]);
    let head = notes(&["- **Facing.** Works on STL and STEP."]);
    assert!(pr_problems(&files(&["apps/openchuck-ui/src/a.rs"]), &base, &head, "").is_empty());
}

#[test]
fn changelog_flow_removing_an_entry_is_not_an_entry() {
    let base = notes(&["- **A.** One.", "- **B.** Two."]);
    let head = notes(&["- **A.** One."]);
    assert!(!pr_problems(&files(&["apps/openchuck-ui/src/a.rs"]), &base, &head, "").is_empty());
}

#[test]
fn changelog_flow_the_escape_hatch_needs_a_reason() {
    for body in [
        "No user-visible change:",
        "No user-visible change:   ",
        "No user-visible change",
        "- No user-visible change: <reason>",
    ] {
        assert!(
            !pr_problems(
                &files(&["apps/openchuck-ui/src/a.rs"]),
                &notes(&[]),
                &notes(&[]),
                body
            )
            .is_empty(),
            "{body}"
        );
    }
}

#[test]
fn changelog_flow_the_escape_hatch_may_be_a_bullet_or_bold() {
    for body in [
        "- No user-visible change: tests only",
        "**No user-visible change:** tests only",
        "no user-visible change: tests only",
        "  * No user-visible change: tests only",
        "Summary.\r\nNo user-visible change: tests only\r\n",
    ] {
        assert!(
            pr_problems(
                &files(&["apps/openchuck-ui/src/a.rs"]),
                &notes(&[]),
                &notes(&[]),
                body
            )
            .is_empty(),
            "{body}"
        );
    }
}

#[test]
fn changelog_flow_the_escape_hatch_inside_a_sentence_does_not_count() {
    let body = "This says there is No user-visible change: really, but not on a line of its own.";
    assert!(
        !pr_problems(
            &files(&["apps/openchuck-ui/src/a.rs"]),
            &notes(&[]),
            &notes(&[]),
            body
        )
        .is_empty()
    );
}

#[test]
fn changelog_flow_the_failure_names_the_files_the_fix_and_the_escape_hatch() {
    let text = pr_problems(
        &files(&["crates/openchuck-path/src/lib.rs"]),
        &notes(&[]),
        &notes(&[]),
        "",
    )
    .join("\n");
    for needle in [
        "crates/openchuck-path/src/lib.rs",
        "## Development version",
        "user-docs/src/release-notes.md",
        "crates/openchuck-path/CHANGELOG.md",
        "No user-visible change: <reason>",
        "marked ready for review",
    ] {
        assert!(text.contains(needle), "{needle} not in {text}");
    }
}

#[test]
fn changelog_flow_a_long_list_of_files_is_shortened() {
    let many: Vec<String> = (0..15)
        .map(|i| format!("apps/openchuck-ui/src/f{i:02}.rs"))
        .collect();
    let text = pr_problems(&many, &notes(&[]), &notes(&[]), "").join("\n");
    assert!(text.contains("and 5 more"), "{text}");
}

#[test]
fn changelog_flow_an_entry_must_not_carry_internal_numbers() {
    let changed = files(&[
        "apps/openchuck-ui/src/a.rs",
        "user-docs/src/release-notes.md",
    ]);
    for entry in [
        "- **A.** Fixed (decision 0656).",
        "- **A.** See Decisions 0400 and later.",
        "- **A.** Closes OI-933.",
        "- **A.** Meets GC-07.",
        "- **A.** As NFR-12 says.",
    ] {
        let problems = pr_problems(&changed, &notes(&[]), &notes(&[entry]), "");
        assert!(
            problems.iter().any(|p| p.contains("internal")),
            "{entry}: {problems:?}"
        );
    }
    for entry in [
        "- **A.** Fixed (#577).",
        "- **A.** The decision 12 times.",
        "- **A.** Spindle at 12000 rpm on an M3-5 screw, A-12 degrees, GCODE-1.",
        "- **A.** Bolts of size AB-1234.",
    ] {
        assert!(
            pr_problems(&changed, &notes(&[]), &notes(&[entry]), "").is_empty(),
            "{entry}"
        );
    }
}

#[test]
fn changelog_flow_internal_numbers_fail_even_with_the_escape_hatch() {
    let changed = files(&["user-docs/src/release-notes.md"]);
    let problems = pr_problems(
        &changed,
        &notes(&[]),
        &notes(&["- **A.** (OI-1)."]),
        "No user-visible change: x",
    );
    assert_eq!(problems.len(), 1);
}

#[test]
fn changelog_flow_an_untouched_old_entry_with_numbers_is_not_this_pull_requests_problem() {
    let changed = files(&[
        "apps/openchuck-ui/src/a.rs",
        "user-docs/src/release-notes.md",
    ]);
    let base = notes(&["- **Old.** (decision 0001)."]);
    let head = notes(&["- **Old.** (decision 0001).", "- **New.** Fine."]);
    assert!(pr_problems(&changed, &base, &head, "").is_empty());
}

// ------------------------------------------------------------------------------- the release

#[test]
fn changelog_flow_a_release_with_its_section_and_an_empty_development_version_passes() {
    assert!(release_problems(&notes_with(&[], &["0.60.0", "0.55.0"]), "0.60.0").is_empty());
}

#[test]
fn changelog_flow_a_release_with_development_entries_left_fails() {
    let problems = release_problems(
        &notes_with(&["- **A.** One."], &["0.60.0", "0.55.0"]),
        "0.60.0",
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("Development version") && p.contains("1 entry")),
        "{problems:?}"
    );
}

#[test]
fn changelog_flow_a_release_without_its_section_fails() {
    let problems = release_problems(&notes(&[]), "0.60.0");
    assert!(
        problems.iter().any(|p| p.contains("0.60.0")),
        "{problems:?}"
    );
}

#[test]
fn changelog_flow_a_draft_section_not_yet_edited_fails() {
    let drafted = start_release(&notes(&["- **A.** One."]), "0.60.0").expect("drafted");
    let problems = release_problems(&drafted, "0.60.0");
    assert!(problems.iter().any(|p| p.contains("draft")), "{problems:?}");
}

#[test]
fn changelog_flow_start_release_matches_the_fixture() {
    let before = include_str!("release-notes-before.md");
    let expected = include_str!("release-notes-after-0.60.0.md");
    assert_eq!(start_release(before, "0.60.0").expect("drafted"), expected);
}

#[test]
fn changelog_flow_start_release_empties_the_development_version_and_keeps_its_text() {
    let after =
        start_release(&notes(&["- **A.** One.", "- **B.** Two."]), "0.60.0").expect("drafted");
    assert!(development_entries(&after).is_empty());
    assert!(after.contains(INTRO));
    assert!(after.trim_end().ends_with(TAIL));
    assert_eq!(latest_release(&after).as_deref(), Some("0.60.0"));
    let new = after.find("## 0.60.0").expect("new section");
    let old = after.find("## 0.55.0").expect("old section");
    assert!(new < old);
    let section = after.get(new..old).expect("section");
    assert!(section.contains("- **A.** One."));
    assert!(section.contains("- **B.** Two."));
    assert!(section.contains(TAIL));
}

#[test]
fn changelog_flow_start_release_with_nothing_to_move_says_so() {
    let after = start_release(&notes(&[]), "0.60.0").expect("drafted");
    assert!(after.contains("- Nothing user-visible has changed since the last release."));
}

#[test]
fn changelog_flow_start_release_refuses_an_existing_or_older_version() {
    for v in ["0.55.0", "0.54.0", "next", "1.2", "1.2.3.4"] {
        assert!(start_release(&notes(&["- **A.** One."]), v).is_err(), "{v}");
    }
}

#[test]
fn changelog_flow_start_release_refuses_a_page_with_no_development_section() {
    assert!(start_release("# Release notes\n\n## 0.55.0\n\n- a\n", "0.60.0").is_err());
}

// ---------------------------------------------------------- the commands, on a throwaway repository

struct Repo {
    dir: PathBuf,
}

impl Repo {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("xtask-changelog-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let repo = Self { dir };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.git(&["config", "user.email", "t@example.invalid"]);
        repo.git(&["config", "user.name", "t"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo.write(NOTES_PATH, &notes(&["- **Old.** Already there."]));
        repo.write(
            "crates/openchuck-types/src/notice.rs",
            "pub const VERIFY_GCODE_NOTICE: &str = \"Generated G-code must be verified by the user before it is run on a machine. \\\n    OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.\";\n",
        );
        repo.commit("base");
        repo
    }

    fn git(&self, args: &[&str]) -> String {
        let out = Command::new("git")
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("git");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.dir.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(path, text).expect("write");
    }

    fn commit(&self, message: &str) -> String {
        self.git(&["add", "."]);
        self.git(&["commit", "-q", "-m", message]);
        self.git(&["rev-parse", "HEAD"])
    }

    fn root(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn changelog_flow_check_pr_fails_then_passes_with_an_entry() {
    let repo = Repo::new("check-pr");
    let base = repo.git(&["rev-parse", "HEAD"]);
    repo.write("apps/openchuck-ui/src/a.rs", "fn a() {}\n");
    repo.commit("code");
    let problems = check_pr(repo.root(), &base, "HEAD", "Work.").expect("check");
    assert!(
        problems
            .iter()
            .any(|p| p.contains("apps/openchuck-ui/src/a.rs")),
        "{problems:?}"
    );
    assert!(
        check_pr(
            repo.root(),
            &base,
            "HEAD",
            "No user-visible change: internal"
        )
        .expect("check")
        .is_empty()
    );
    repo.write(
        NOTES_PATH,
        &notes(&["- **Old.** Already there.", "- **New.** New."]),
    );
    repo.commit("entry");
    assert!(
        check_pr(repo.root(), &base, "HEAD", "")
            .expect("check")
            .is_empty()
    );
}

#[test]
fn changelog_flow_check_pr_reads_only_the_pull_requests_own_changes() {
    let repo = Repo::new("merge-base");
    repo.git(&["checkout", "-q", "-b", "lane"]);
    repo.write("docs/x.md", "x\n");
    repo.commit("docs");
    repo.git(&["checkout", "-q", "main"]);
    repo.write("apps/openchuck-ui/src/a.rs", "fn a() {}\n");
    let main = repo.commit("other lane");
    repo.git(&["checkout", "-q", "lane"]);
    assert!(
        check_pr(repo.root(), &main, "HEAD", "")
            .expect("check")
            .is_empty()
    );
}

#[test]
fn changelog_flow_check_pr_with_an_unknown_base_is_an_error() {
    let repo = Repo::new("bad-base");
    assert!(check_pr(repo.root(), "no-such-commit", "HEAD", "").is_err());
}

#[test]
fn changelog_flow_nightly_notes_command_writes_the_file() {
    let repo = Repo::new("nightly");
    let out = repo.root().join("notes.md");
    write_nightly_notes(repo.root(), SHA, "20261123", REPO, Some(&out)).expect("notes");
    let text = std::fs::read_to_string(&out).expect("read");
    assert!(text.contains("- **Old.** Already there."));
    assert!(text.contains(NOTICE));
}

#[test]
fn changelog_flow_the_notice_is_read_from_the_program() {
    let repo = Repo::new("notice");
    assert_eq!(read_notice(repo.root()).expect("notice"), NOTICE);
    repo.write("crates/openchuck-types/src/notice.rs", "// nothing here\n");
    assert!(read_notice(repo.root()).is_err());
}

#[test]
fn changelog_flow_start_release_command_writes_and_check_release_reads() {
    let repo = Repo::new("release");
    assert_eq!(start_release_file(repo.root(), "0.60.0").expect("moved"), 1);
    let problems = check_release(repo.root(), "0.60.0").expect("check");
    assert!(problems.iter().any(|p| p.contains("draft")), "{problems:?}");
    let again = start_release_file(repo.root(), "0.60.0");
    assert!(again.is_err_and(|e| e.to_string().contains("0.60.0")));
}

#[test]
fn changelog_flow_the_real_release_notes_parse() {
    // The page in this repository: a Development version section and a released version.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("root");
    let text = std::fs::read_to_string(root.join(NOTES_PATH)).expect("release notes");
    assert!(latest_release(&text).is_some());
    let notice = read_notice(root).expect("notice");
    let out =
        nightly_notes(&text, &notice, SHA, "20261123", REPO, GUIDE, NOTES_LIMIT).expect("notes");
    assert!(out.contains(&notice));
}

#[test]
fn changelog_flow_the_first_release_takes_the_section_above_the_development_version() {
    let after = start_release(
        &notes_with(&["- **A.** One.", "- **B.** Two."], &[]),
        "0.1.0",
    )
    .expect("drafted");
    let new = after.find("## 0.1.0").expect("new section");
    let dev = after.find("## Development version").expect("dev section");
    assert!(new < dev);
    assert!(development_entries(&after).is_empty());
    let problems = release_problems(
        &notes_with(&["- **A.** One.", "- **B.** Two."], &["0.1.0"]),
        "0.1.0",
    );
    assert!(
        problems.iter().any(|p| p.contains("2 entries")),
        "{problems:?}"
    );
}
