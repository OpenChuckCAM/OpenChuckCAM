# SPDX-License-Identifier: AGPL-3.0-or-later
"""Tests of the changelog flow (decision record CHANGELOG-FLOW-A; NFR-12; decision 0400 section R).

Run with `python3 -m unittest discover -s user-docs/tools/tests`.
Each test is named after the record (`changelog_flow`) or the requirement it covers.
"""
import os
import subprocess
import sys
import tempfile
import textwrap
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

import changelog  # noqa: E402

NOTICE = (
    "Generated G-code must be verified by the user before it is run on a machine. "
    "OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup."
)
SHA = "0123456789abcdef0123456789abcdef01234567"
REPO = "OpenChuckCAM/OpenChuckCAM"
GUIDE = "https://openchuck.cam/latest/"

INTRO = ("The *latest* version of this guide describes the nightly build, the "
         "[development build](start/install.md#download-a-development-build) made each night from the main branch.")
TAIL = ("Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not guarantee "
        "that a program is safe or correct for any machine, tool or setup.")


def notes(dev_entries=(), releases=("0.55.0",), dev_extra=""):
    """A release-notes page shaped like user-docs/src/release-notes.md."""
    out = ["# Release notes", "", "What changed in each release.", ""]
    for v in releases:
        out += [f"## {v}", "", f"The {v} release.", "", "### Known limitations", "",
                "- **Old.** An old limitation.", "", TAIL, ""]
    out += ["## Development version", "", INTRO, ""]
    out += list(dev_entries)
    if dev_entries:
        out.append("")
    if dev_extra:
        out += [dev_extra, ""]
    out += [TAIL, ""]
    return "\n".join(out)


class DevelopmentEntries(unittest.TestCase):
    """The entries are the top-level bullets of the Development version section, nothing else."""

    def test_changelog_flow_entries_are_the_bullets_of_the_development_section(self):
        text = notes(["- **Keyboard.** Tab follows reading order.", "- **Facing.** Works on a STEP part."])
        self.assertEqual(changelog.development_entries(text),
                         ["- **Keyboard.** Tab follows reading order.", "- **Facing.** Works on a STEP part."])

    def test_changelog_flow_bullets_of_released_sections_are_not_entries(self):
        self.assertEqual(changelog.development_entries(notes()), [])

    def test_changelog_flow_an_entry_keeps_its_continuation_lines_and_sub_bullets(self):
        text = notes(["- **Facing.** Works on a STEP part,", "  bounded from its exact faces.",
                      "  - Flat and round shapes.", "- **Two.** Second."])
        entries = changelog.development_entries(text)
        self.assertEqual(len(entries), 2)
        self.assertIn("bounded from its exact faces.", entries[0])
        self.assertIn("  - Flat and round shapes.", entries[0])

    def test_changelog_flow_no_development_section_means_no_entries(self):
        self.assertEqual(changelog.development_entries("# Release notes\n\n## 0.55.0\n\n- a\n"), [])

    def test_changelog_flow_latest_release_is_the_highest_version_heading(self):
        self.assertEqual(changelog.latest_release(notes(releases=("0.60.0", "0.55.0"))), "0.60.0")
        self.assertEqual(changelog.latest_release(notes(releases=("0.9.0", "0.10.0"))), "0.10.0")
        self.assertIsNone(changelog.latest_release(notes(releases=())))


class NightlyNotes(unittest.TestCase):
    """The nightly pre-release notes say what changed since the last release (the owner's request, 23 Nov 2026)."""

    def make(self, text, limit=changelog.NOTES_LIMIT):
        return changelog.nightly_notes(text, NOTICE, SHA, "20261123", REPO, GUIDE, limit=limit)

    def test_changelog_flow_notes_list_the_development_entries_since_the_last_release(self):
        out = self.make(notes(["- **Keyboard.** Tab follows reading order."]))
        self.assertIn("since the last release, 0.55.0", out)
        self.assertIn("- **Keyboard.** Tab follows reading order.", out)
        self.assertIn(f"`main` at {SHA}", out)
        self.assertIn(GUIDE + "release-notes.html#development-version", out)

    def test_req_nfr_12_notes_carry_the_gcode_notice_and_say_it_is_not_a_release(self):
        out = self.make(notes(["- **A.** One."]))
        self.assertIn(f"**{NOTICE}**", out)
        self.assertIn("**This is not a release.**", out)
        for phrase in ("safe to run", "ready to run", "verified safe", "guaranteed"):
            self.assertNotIn(phrase, out.lower().replace(NOTICE.lower(), ""))

    def test_changelog_flow_an_empty_section_says_nothing_user_visible_changed(self):
        out = self.make(notes([]))
        self.assertIn("Nothing user-visible has changed since the last release, 0.55.0.", out)

    def test_changelog_flow_no_release_yet_says_so(self):
        out = self.make(notes(["- **A.** One."], releases=()))
        self.assertIn("There has been no release yet", out)
        self.assertIn("- **A.** One.", out)

    def test_changelog_flow_relative_guide_links_point_at_the_latest_guide(self):
        out = self.make(notes(["- **Keys.** See [Keyboard](reference/keyboard.md) and "
                               "[install](start/install.md#download-a-development-build), "
                               "[issue](https://github.com/x/y/issues/1) and [here](#development-version)."]))
        self.assertIn("[Keyboard](https://openchuck.cam/latest/reference/keyboard.html)", out)
        self.assertIn("(https://openchuck.cam/latest/start/install.html#download-a-development-build)", out)
        self.assertIn("[issue](https://github.com/x/y/issues/1)", out)
        self.assertIn("(https://openchuck.cam/latest/release-notes.html#development-version)", out)

    def test_changelog_flow_issue_references_link_to_the_tracker(self):
        out = self.make(notes(["- **Keys.** Fixed (#577, #578)."]))
        self.assertIn("(#577, #578)", out)

    def test_changelog_flow_library_changelogs_are_linked_at_the_built_commit(self):
        out = self.make(notes(["- **A.** One."]))
        for crate in ("openchuck-types", "openchuck-path", "openchuck-post", "openchuck-gcode"):
            self.assertIn(f"https://github.com/{REPO}/blob/{SHA}/crates/{crate}/CHANGELOG.md", out)

    def test_changelog_flow_notes_are_bounded_and_say_where_the_rest_is(self):
        entries = [f"- **Entry {i}.** " + "words " * 40 for i in range(200)]
        out = self.make(notes(entries), limit=6000)
        self.assertLessEqual(len(out), 6000)
        self.assertIn("- **Entry 0.**", out)
        self.assertNotIn("- **Entry 199.**", out)
        self.assertRegex(out, r"\d+ more changes are in the \[release notes")

    def test_changelog_flow_notes_are_deterministic(self):
        text = notes(["- **A.** One.", "- **B.** Two."])
        self.assertEqual(self.make(text), self.make(text))

    def test_changelog_flow_the_default_limit_is_below_githubs(self):
        self.assertLessEqual(changelog.NOTES_LIMIT, 100_000)


# (changed files, adds a development entry, changes a crate changelog, escape hatch, expected to pass)
# The table of example diffs the coordinator asked for: the trigger globs, and what passes.
DIFFS = [
    # Docs and registers only: never triggered.
    (["docs/decisions/0700-x.md", "docs/open-items.md"], False, None, None, True),
    (["AGENTS.md", "CLAUDE.md", "README.md"], False, None, None, True),
    (["docs/releasing.md", ".github/workflows/ci.yml"], False, None, None, True),
    # A crate changelog alone (a lane's changelog-only edit): not triggered.
    (["crates/openchuck-app/CHANGELOG.md"], False, None, None, True),
    (["crates/openchuck-path/CHANGELOG.md", "Cargo.toml", "Cargo.lock"], False, None, None, True),
    # Tests, benches, examples, the test kit, xtask and testdata outside the examples: not triggered.
    (["crates/openchuck-path/tests/foo.rs", "crates/openchuck-app/tests/integration.rs"], False, None, None, True),
    (["crates/openchuck-testkit/src/lib.rs", "xtask/src/records.rs", "testdata/golden/a.ngc"], False, None, None, True),
    (["crates/openchuck-path/benches/b.rs", "crates/openchuck-post/examples/e.rs"], False, None, None, True),
    # The release notes alone: not triggered (it is the thing being added).
    (["user-docs/src/release-notes.md"], True, None, None, True),
    # The screenshot records are read by the checks, not shown to users.
    (["user-docs/src/tutorial/images/CAPTURED.toml", "user-docs/src/examples/images/flutes/CAPTURED.toml"],
     False, None, None, True),
    # A new or retaken screenshot is a change to the guide.
    (["user-docs/src/tutorial/images/step-1.png"], False, None, None, False),
    # The guide's tools and theme are not the guide's text.
    (["user-docs/tools/check.py", "user-docs/theme/style.css"], False, None, None, True),
    # Application source: triggered; passes with an entry or the escape hatch.
    (["apps/openchuck-ui/src/app.rs"], False, None, None, False),
    (["apps/openchuck-ui/src/app.rs", "user-docs/src/release-notes.md"], True, None, None, True),
    (["apps/openchuck-ui/src/app.rs"], False, None, "No user-visible change: internal refactor", True),
    (["apps/openchuck-ui/assets/icon.png"], False, None, None, False),
    (["apps/openchuck-cli/src/main.rs"], False, None, None, False),
    # An application crate (not the library family): an entry is enough.
    (["crates/openchuck-app/src/lib.rs", "user-docs/src/release-notes.md"], True, None, None, True),
    # A library-family crate: an entry and that crate's changelog.
    (["crates/openchuck-path/src/lib.rs", "user-docs/src/release-notes.md"], True, None, None, False),
    (["crates/openchuck-path/src/lib.rs", "user-docs/src/release-notes.md", "crates/openchuck-path/CHANGELOG.md"],
     True, "crates/openchuck-path/CHANGELOG.md", None, True),
    (["crates/openchuck-path/src/lib.rs", "crates/openchuck-path/CHANGELOG.md"], False, None, None, False),
    (["crates/openchuck-path/src/lib.rs"], False, None, "No user-visible change: a private helper", True),
    # The built-in posts belong to openchuck-post.
    (["crates/openchuck-post/posts/z1.toml", "user-docs/src/release-notes.md", "crates/openchuck-post/CHANGELOG.md"],
     True, None, None, True),
    (["crates/openchuck-post/posts/z1.toml", "user-docs/src/release-notes.md"], True, None, None, False),
    # The example projects (shipped in the examples zip) and the guide's pages.
    (["testdata/examples/baluster/project.ocp"], False, None, None, False),
    (["user-docs/src/start/install.md"], False, None, None, False),
    (["user-docs/src/start/install.md"], False, None, "No user-visible change: a typo in the guide", True),
]


class PullRequestCheck(unittest.TestCase):
    """The light check of each pull request (CHANGELOG-FLOW-A, the rule and its escape hatch)."""

    def run_rule(self, files, adds_entry, hatch):
        base = notes(["- **Old.** Already there."])
        head = notes(["- **Old.** Already there.", "- **New.** Something new."]) if adds_entry else base
        body = f"## Summary\n\nWork.\n\n{hatch}\n" if hatch else "## Summary\n\nWork.\n"
        return changelog.pr_problems(files, base, head, body)

    def test_changelog_flow_the_table_of_example_diffs(self):
        for files, adds, _crate, hatch, ok in DIFFS:
            with self.subTest(files=files, hatch=hatch):
                problems = self.run_rule(files, adds, hatch)
                self.assertEqual(problems == [], ok, f"{files}: {problems}")

    def test_changelog_flow_a_changed_entry_counts_as_an_entry(self):
        base = notes(["- **Facing.** Works on STL."])
        head = notes(["- **Facing.** Works on STL and STEP."])
        self.assertEqual(changelog.pr_problems(["apps/openchuck-ui/src/a.rs"], base, head, ""), [])

    def test_changelog_flow_removing_an_entry_is_not_an_entry(self):
        base = notes(["- **A.** One.", "- **B.** Two."])
        head = notes(["- **A.** One."])
        self.assertNotEqual(changelog.pr_problems(["apps/openchuck-ui/src/a.rs"], base, head, ""), [])

    def test_changelog_flow_the_escape_hatch_needs_a_reason(self):
        for body in ("No user-visible change:", "No user-visible change:   ", "No user-visible change",
                     "- No user-visible change: <reason>"):
            with self.subTest(body=body):
                self.assertNotEqual(changelog.pr_problems(["apps/openchuck-ui/src/a.rs"], notes(), notes(), body), [])

    def test_changelog_flow_the_escape_hatch_may_be_a_bullet_or_bold(self):
        for body in ("- No user-visible change: tests only", "**No user-visible change:** tests only",
                     "no user-visible change: tests only", "  * No user-visible change: tests only"):
            with self.subTest(body=body):
                self.assertEqual(changelog.pr_problems(["apps/openchuck-ui/src/a.rs"], notes(), notes(), body), [])

    def test_changelog_flow_the_escape_hatch_inside_a_sentence_does_not_count(self):
        body = "This says there is No user-visible change: really, but not on a line of its own."
        self.assertNotEqual(changelog.pr_problems(["apps/openchuck-ui/src/a.rs"], notes(), notes(), body), [])

    def test_changelog_flow_the_failure_names_the_files_the_fix_and_the_escape_hatch(self):
        problems = changelog.pr_problems(["crates/openchuck-path/src/lib.rs"], notes(), notes(), "")
        text = "\n".join(problems)
        self.assertIn("crates/openchuck-path/src/lib.rs", text)
        self.assertIn("## Development version", text)
        self.assertIn("user-docs/src/release-notes.md", text)
        self.assertIn("crates/openchuck-path/CHANGELOG.md", text)
        self.assertIn("No user-visible change: <reason>", text)
        self.assertIn("marked ready for review", text)

    def test_changelog_flow_an_entry_must_not_carry_internal_numbers(self):
        files = ["apps/openchuck-ui/src/a.rs", "user-docs/src/release-notes.md"]
        for entry in ("- **A.** Fixed (decision 0656).", "- **A.** Closes OI-933.", "- **A.** Meets GC-07.",
                      "- **A.** As NFR-12 says."):
            with self.subTest(entry=entry):
                problems = changelog.pr_problems(files, notes(), notes([entry]), "")
                self.assertTrue(any("internal" in p for p in problems), problems)
        self.assertEqual(changelog.pr_problems(files, notes(), notes(["- **A.** Fixed (#577)."]), ""), [])

    def test_changelog_flow_an_untouched_old_entry_with_numbers_is_not_this_pull_requests_problem(self):
        files = ["apps/openchuck-ui/src/a.rs", "user-docs/src/release-notes.md"]
        base = notes(["- **Old.** (decision 0001)."])
        head = notes(["- **Old.** (decision 0001).", "- **New.** Fine."])
        self.assertEqual(changelog.pr_problems(files, base, head, ""), [])


class ReleaseChecks(unittest.TestCase):
    """At a release the Development version is moved into the version's section and left empty."""

    def test_changelog_flow_a_release_with_its_section_and_an_empty_development_version_passes(self):
        self.assertEqual(changelog.release_problems(notes(releases=("0.60.0", "0.55.0")), "0.60.0"), [])

    def test_changelog_flow_a_release_with_development_entries_left_fails(self):
        problems = changelog.release_problems(notes(["- **A.** One."], releases=("0.60.0", "0.55.0")), "0.60.0")
        self.assertTrue(any("Development version" in p for p in problems), problems)

    def test_changelog_flow_a_release_without_its_section_fails(self):
        problems = changelog.release_problems(notes(), "0.60.0")
        self.assertTrue(any("0.60.0" in p for p in problems), problems)

    def test_changelog_flow_a_draft_section_not_yet_edited_fails(self):
        drafted = changelog.start_release(notes(["- **A.** One."]), "0.60.0")
        problems = changelog.release_problems(drafted, "0.60.0")
        self.assertTrue(any("draft" in p for p in problems), problems)


class StartRelease(unittest.TestCase):
    """start-release moves the Development version entries into a draft section for the version (on a fixture)."""

    FIXTURE = os.path.join(os.path.dirname(__file__), "fixtures", "release-notes-before.md")
    EXPECTED = os.path.join(os.path.dirname(__file__), "fixtures", "release-notes-after-0.60.0.md")

    def test_changelog_flow_start_release_matches_the_fixture(self):
        with open(self.FIXTURE, encoding="utf-8") as f:
            before = f.read()
        with open(self.EXPECTED, encoding="utf-8") as f:
            expected = f.read()
        self.assertEqual(changelog.start_release(before, "0.60.0"), expected)

    def test_changelog_flow_start_release_empties_the_development_version_and_keeps_its_text(self):
        after = changelog.start_release(notes(["- **A.** One.", "- **B.** Two."]), "0.60.0")
        self.assertEqual(changelog.development_entries(after), [])
        self.assertIn(INTRO, after)
        self.assertTrue(after.rstrip().endswith(TAIL))
        self.assertEqual(changelog.latest_release(after), "0.60.0")
        self.assertLess(after.index("## 0.60.0"), after.index("## 0.55.0"))
        section = after[after.index("## 0.60.0"):after.index("## 0.55.0")]
        self.assertIn("- **A.** One.", section)
        self.assertIn("- **B.** Two.", section)
        self.assertIn(TAIL, section)

    def test_changelog_flow_start_release_refuses_an_existing_or_older_version(self):
        with self.assertRaises(changelog.ChangelogError):
            changelog.start_release(notes(["- **A.** One."]), "0.55.0")
        with self.assertRaises(changelog.ChangelogError):
            changelog.start_release(notes(["- **A.** One."]), "0.54.0")
        with self.assertRaises(changelog.ChangelogError):
            changelog.start_release(notes(["- **A.** One."]), "next")

    def test_changelog_flow_start_release_refuses_a_page_with_no_development_section(self):
        with self.assertRaises(changelog.ChangelogError):
            changelog.start_release("# Release notes\n\n## 0.55.0\n\n- a\n", "0.60.0")


class CommandLine(unittest.TestCase):
    """The commands the workflows run, on a throwaway git repository."""

    def setUp(self):
        self.repo = tempfile.mkdtemp()
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.email", "t@example.invalid")
        self.git("config", "user.name", "t")
        self.write("user-docs/src/release-notes.md", notes(["- **Old.** Already there."]))
        self.write("crates/openchuck-types/src/notice.rs",
                   'pub const VERIFY_GCODE_NOTICE: &str = "Generated G-code must be verified by the user before it is run on a machine. \\\n'
                   '    OpenChuckCAM does not guarantee that a program is safe or correct for any machine, tool or setup.";\n')
        self.git("add", ".")
        self.git("commit", "-q", "-m", "base")
        self.base = self.git("rev-parse", "HEAD")

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.repo, check=True, capture_output=True, text=True).stdout.strip()

    def write(self, rel, text):
        path = os.path.join(self.repo, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)

    def cli(self, *args, env=None):
        full = dict(os.environ, **(env or {}))
        return subprocess.run([sys.executable, os.path.join(os.path.dirname(__file__), "..", "changelog.py"),
                               "--repo-root", self.repo, *args], capture_output=True, text=True, env=full)

    def test_changelog_flow_check_pr_fails_then_passes_with_an_entry(self):
        self.write("apps/openchuck-ui/src/a.rs", "fn a() {}\n")
        self.git("add", ".")
        self.git("commit", "-q", "-m", "code")
        r = self.cli("check-pr", "--base", self.base, env={"PR_BODY": "Work."})
        self.assertEqual(r.returncode, 1, r.stdout + r.stderr)
        self.assertIn("apps/openchuck-ui/src/a.rs", r.stdout)
        r = self.cli("check-pr", "--base", self.base, env={"PR_BODY": "No user-visible change: internal"})
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.write("user-docs/src/release-notes.md", notes(["- **Old.** Already there.", "- **New.** New."]))
        self.git("add", ".")
        self.git("commit", "-q", "-m", "entry")
        r = self.cli("check-pr", "--base", self.base, env={"PR_BODY": ""})
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_changelog_flow_check_pr_reads_only_the_pull_requests_own_changes(self):
        # main moves on with a code change after the branch was made; the branch changes docs only.
        self.git("checkout", "-q", "-b", "lane")
        self.write("docs/x.md", "x\n")
        self.git("add", ".")
        self.git("commit", "-q", "-m", "docs")
        self.git("checkout", "-q", "main")
        self.write("apps/openchuck-ui/src/a.rs", "fn a() {}\n")
        self.git("add", ".")
        self.git("commit", "-q", "-m", "other lane")
        main = self.git("rev-parse", "HEAD")
        self.git("checkout", "-q", "lane")
        r = self.cli("check-pr", "--base", main, env={"PR_BODY": ""})
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_changelog_flow_nightly_notes_command_writes_the_file(self):
        out = os.path.join(self.repo, "notes.md")
        r = self.cli("nightly-notes", "--sha", SHA, "--date", "20261123", "--repo", REPO, "--out", out)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        with open(out, encoding="utf-8") as f:
            text = f.read()
        self.assertIn("- **Old.** Already there.", text)
        self.assertIn(NOTICE, text)

    def test_changelog_flow_start_release_command_writes_and_check_release_reads(self):
        r = self.cli("start-release", "0.60.0")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        r = self.cli("check-release", "0.60.0")
        self.assertEqual(r.returncode, 1)
        self.assertIn("draft", r.stdout)
        r = self.cli("start-release", "0.60.0")
        self.assertEqual(r.returncode, 1)
        self.assertIn("0.60.0", r.stdout + r.stderr)


if __name__ == "__main__":
    unittest.main()
