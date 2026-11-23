# SPDX-License-Identifier: AGPL-3.0-or-later
"""The changelog since the last release: nightly notes, the pull request check and the release step.

    python3 user-docs/tools/changelog.py nightly-notes --sha SHA --date YYYYMMDD [--repo OWNER/NAME] [--out FILE]
    python3 user-docs/tools/changelog.py check-pr --base SHA [--head REV]    (the description in $PR_BODY)
    python3 user-docs/tools/changelog.py start-release VERSION
    python3 user-docs/tools/changelog.py check-release VERSION

Decision record CHANGELOG-FLOW-A. The changelog is the *Development version* section of
user-docs/src/release-notes.md: one bullet per change that users could notice, in plain words, since
the last release. The nightly pre-release notes are made from it (nightly-notes); each pull request
that changes what users see adds or changes a bullet, or says "No user-visible change: <reason>" in
its description (check-pr); a release moves the bullets into the version's own section and leaves
the Development version empty (start-release, then check-release in the release workflow).

Generated G-code is the user's to verify before it is run on a machine (NFR-12): nothing written
here says otherwise, and the notes carry the program's own notice.

Python standard library only, so CI needs no package. The output is deterministic.
"""
import argparse
import os
import posixpath
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
NOTES_PATH = "user-docs/src/release-notes.md"
DEV_HEADING = "Development version"
GUIDE = "https://openchuck.cam/latest/"
DEFAULT_REPO = "OpenChuckCAM/OpenChuckCAM"
# GitHub accepts a release body of 125,000 characters; the notes stay well below it.
NOTES_LIMIT = 60_000
# The crates published to crates.io (docs/releasing.md); a change to one needs its own CHANGELOG.md entry.
FAMILY = ("openchuck-types", "openchuck-path", "openchuck-post", "openchuck-gcode")
HATCH = "No user-visible change"
DRAFT_MARKER = "<!-- release-notes draft"
# Numbers that mean nothing to users (the owner's rule: release notes carry no decision, open item or
# requirement numbers). Issue numbers (#577) are allowed: they link to the issue tracker.
INTERNAL = [
    re.compile(r"\b(?:decision|decisions|record|records)\s+\d{3,4}\b", re.I),
    re.compile(r"\bOI-\d+\b"),
    re.compile(r"\b[A-Z]{2,4}-\d{2,3}\b"),
]
VERSION = re.compile(r"^(\d+)\.(\d+)\.(\d+)$")


class ChangelogError(Exception):
    """The page or the request cannot be handled; the message says why."""


# ---------------------------------------------------------------------------------------------- parsing

def _sections(lines):
    """(heading text, first line, end line) of each `## ` section; the end is exclusive."""
    starts = [i for i, line in enumerate(lines) if line.startswith("## ")]
    return [(lines[s][3:].strip(), s, starts[k + 1] if k + 1 < len(starts) else len(lines))
            for k, s in enumerate(starts)]


def _dev_section(lines):
    for heading, start, end in _sections(lines):
        if heading == DEV_HEADING:
            return start, end
    return None


def _is_bullet(line):
    return line.startswith("- ") or line.startswith("* ")


def _entry_spans(lines, start, end):
    """[first, last) line spans of the top-level bullets between start and end."""
    spans = []
    i = start
    while i < end:
        if not _is_bullet(lines[i]):
            i += 1
            continue
        first = i
        i += 1
        last = i
        while i < end:
            line = lines[i]
            if _is_bullet(line):
                break
            if line.strip() == "":
                # A blank line: the item goes on only if the next text is indented under it.
                j = i
                while j < end and lines[j].strip() == "":
                    j += 1
                if j < end and lines[j].startswith((" ", "\t")):
                    i = j
                    continue
                break
            i += 1
            last = i
        spans.append((first, last))
    return spans


def development_entries(text):
    """The bullets of the Development version section, each with its continuation lines."""
    lines = text.splitlines()
    sec = _dev_section(lines)
    if sec is None:
        return []
    return ["\n".join(lines[a:b]).rstrip() for a, b in _entry_spans(lines, sec[0] + 1, sec[1])]


def _version_key(v):
    m = VERSION.match(v)
    return tuple(int(x) for x in m.groups()) if m else None


def latest_release(text):
    """The highest version that has a `## X.Y.Z` section, or None before the first release."""
    versions = [h for h, _, _ in _sections(text.splitlines()) if _version_key(h)]
    return max(versions, key=_version_key) if versions else None


# ---------------------------------------------------------------------------------------- nightly notes

def _rewrite_links(text, guide):
    """Relative links of the guide's source become links to the latest guide."""
    def fix(m):
        target = m.group(1)
        if re.match(r"^[a-zA-Z][a-zA-Z0-9+.-]*:", target) or target.startswith("/"):
            return m.group(0)
        path, _, frag = target.partition("#")
        if not path:
            path = "release-notes.md"
        if path.endswith(".md"):
            path = path[:-3] + ".html"
        path = posixpath.normpath(path)
        return "](" + guide + path + ("#" + frag if frag else "") + ")"
    return re.sub(r"\]\(([^)\s]+)\)", fix, text)


def nightly_notes(text, notice, sha, date, repo=DEFAULT_REPO, guide=GUIDE, limit=NOTES_LIMIT):
    """The body of the rolling `nightly` pre-release, at most `limit` characters."""
    latest = latest_release(text)
    entries = [_rewrite_links(e, guide) for e in development_entries(text)]
    notes_link = f"{guide}release-notes.html#development-version"
    head = [
        f"Automatic development build of `main` at {sha}, built {date}.",
        "",
        "**This is not a release.** It passed no release gate and may be incomplete or broken. "
        "It is replaced by the next nightly build.",
        "",
        f"**{notice}**",
        "",
        "## What has changed",
        "",
    ]
    if latest and entries:
        head += [f"Changes since the last release, {latest}, as listed in the "
                 f"[release notes of the latest user guide]({notes_link}):", ""]
    elif latest:
        head += [f"Nothing user-visible has changed since the last release, {latest}. "
                 f"The [release notes]({notes_link}) of the latest user guide list each release.", ""]
    else:
        head += ["There has been no release yet. The changes listed so far in the "
                 f"[release notes of the latest user guide]({notes_link}):", ""]
    crates = ", ".join(f"[{c}](https://github.com/{repo}/blob/{sha}/crates/{c}/CHANGELOG.md)" for c in FAMILY)
    tail = [
        "",
        f"Changes to the library crates, for developers, are in their changelogs: {crates}.",
        "",
        "## Downloads",
        "",
        "Each archive holds `openchuck-cli`, `openchuck-ui`, the licence, the third-party notices and a README. "
        "`openchuckcam-nightly-*-examples.zip` holds the example projects. The binaries are not signed: "
        "macOS quarantines them and Windows SmartScreen may warn (see the README in each archive).",
        "",
    ]
    fixed = len("\n".join(head)) + len("\n".join(tail)) + 2
    more_room = 120  # the "N more changes" line
    if fixed + more_room > limit:
        raise ChangelogError(f"the notes cannot fit in {limit} characters")
    body, used = [], fixed
    for k, e in enumerate(entries):
        if used + len(e) + 1 + more_room > limit:
            left = len(entries) - k
            body += ["", f"{left} more changes are in the [release notes of the latest user guide]({notes_link})."]
            break
        body.append(e)
        used += len(e) + 1
    out = "\n".join(head + body + tail)
    while "\n\n\n" in out:
        out = out.replace("\n\n\n", "\n\n")
    assert len(out) <= limit
    return out


def read_notice(repo_root):
    """The program's own G-code notice (crates/openchuck-types/src/notice.rs), as check.py reads it."""
    with open(os.path.join(repo_root, "crates", "openchuck-types", "src", "notice.rs"), encoding="utf-8") as f:
        rust = f.read()
    m = re.search(r'VERIFY_GCODE_NOTICE: &str = "(.*?)";', rust, re.S)
    if not m:
        raise ChangelogError("could not read VERIFY_GCODE_NOTICE from notice.rs")
    return re.sub(r"\\\n\s*", "", m.group(1))


# ------------------------------------------------------------------------------------ pull request check

def triggers(path):
    """True when a change to this file can change what users see or download."""
    if path == NOTES_PATH:
        return False
    if path.startswith("user-docs/src/") and path.endswith(".toml"):
        return False  # the screenshot records (CAPTURED.toml): read by the checks, not shown to users
    if path.startswith(("user-docs/src/", "testdata/examples/", "crates/openchuck-post/posts/")):
        return True
    if re.match(r"apps/[^/]+/(src|assets)/", path):
        return True
    m = re.match(r"crates/([^/]+)/src/", path)
    return bool(m) and m.group(1) != "openchuck-testkit"


def family_crate(path):
    """The library-family crate whose published contents this file is part of, or None."""
    m = re.match(r"crates/([^/]+)/(src|posts)/", path)
    if m and m.group(1) in FAMILY and (m.group(2) == "src" or m.group(1) == "openchuck-post"):
        return m.group(1)
    return None


def _entries(n):
    return "1 entry" if n == 1 else f"{n} entries"


def _norm(entry):
    return " ".join(entry.split())


def _hatch_reason(body):
    for line in (body or "").splitlines():
        plain = re.sub(r"[*_`]", "", line).strip()
        plain = re.sub(r"^[-*+]\s+", "", plain)
        m = re.match(re.escape(HATCH) + r":\s*(\S.*)$", plain, re.I)
        if m and m.group(1).strip() != "<reason>":
            return m.group(1).strip()
    return None


def pr_problems(files, base_text, head_text, body):
    """What a pull request is missing, as plain sentences; an empty list when it passes."""
    old = {_norm(e) for e in development_entries(base_text)}
    new_entries = [e for e in development_entries(head_text) if _norm(e) not in old]
    problems = []
    for e in new_entries:
        for pattern in INTERNAL:
            m = pattern.search(e)
            if m:
                problems.append(f"the Development version entry '{e.splitlines()[0][:80]}' carries the internal "
                                f"number '{m.group(0)}': release notes carry no decision, open item or requirement "
                                "numbers (issue numbers such as #577 are fine)")
                break
    if _hatch_reason(body):
        return problems
    visible = sorted(f for f in files if triggers(f))
    if not visible:
        return problems
    crates = sorted({family_crate(f) for f in visible} - {None})
    missing_crates = [c for c in crates if f"crates/{c}/CHANGELOG.md" not in files]
    if new_entries and not missing_crates:
        return problems
    shown = ", ".join(visible[:10]) + (f" and {len(visible) - 10} more" if len(visible) > 10 else "")
    problems.append(f"this pull request changes files that can change what users see: {shown}")
    if not new_entries:
        problems.append(f"add or change a bullet under '## {DEV_HEADING}' in {NOTES_PATH}: what changed, in plain "
                        "words for users, with no decision, open item or requirement numbers")
    for c in missing_crates:
        problems.append(f"{c} is published to crates.io: add an entry for this change to crates/{c}/CHANGELOG.md")
    problems.append(f"or, if users cannot notice the change, put a line '{HATCH}: <reason>' in the pull request "
                    "description. The check reads the description when the pull request is opened, on each push "
                    "and when it is marked ready for review; editing the description alone does not run it again")
    return problems


# ------------------------------------------------------------------------------------------ the release

def release_problems(text, version):
    """At a release: the version has its own section, written (no draft marker), and the Development version is empty."""
    lines = text.splitlines()
    problems = []
    sec = [(s, e) for h, s, e in _sections(lines) if re.match(re.escape(version) + r"(\s|$)", h)]
    if not sec:
        problems.append(f"the release notes have no section headed {version}")
    elif any(DRAFT_MARKER in line for line in lines[sec[0][0]:sec[0][1]]):
        problems.append(f"the section for {version} is still the draft made by start-release: write it, then "
                        "delete the draft comment")
    left = development_entries(text)
    if left:
        problems.append(f"the {DEV_HEADING} section still has {_entries(len(left))}: move them into the section for "
                        f"{version} (changelog.py start-release does it)")
    return problems


def _notice_paragraph(lines, start, end):
    """The closing G-code reminder of the Development version section, so the new section ends the same way."""
    for i in range(end - 1, start, -1):
        if lines[i].startswith("Generated G-code"):
            return lines[i]
    return ("Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM does not "
            "guarantee that a program is safe or correct for any machine, tool or setup.")


def start_release(text, version):
    """Move the Development version entries into a new draft `## <version>` section above the newest release."""
    key = _version_key(version)
    if key is None:
        raise ChangelogError(f"'{version}' is not a version (X.Y.Z)")
    latest = latest_release(text)
    if latest and key <= _version_key(latest):
        raise ChangelogError(f"{version} is not newer than the last release, {latest}")
    lines = text.splitlines()
    sec = _dev_section(lines)
    if sec is None:
        raise ChangelogError(f"the release notes have no '## {DEV_HEADING}' section")
    start, end = sec
    spans = _entry_spans(lines, start + 1, end)
    entries = ["\n".join(lines[a:b]).rstrip() for a, b in spans]
    reminder = _notice_paragraph(lines, start, end)

    # The Development version keeps its heading and paragraphs, without the entries.
    drop = set()
    for a, b in spans:
        drop.update(range(a, b))
    kept = [line for i, line in enumerate(lines[start:end], start) if i not in drop]
    dev = []
    for line in kept:
        if line.strip() == "" and dev and dev[-1].strip() == "":
            continue
        dev.append(line)

    new = [
        f"## {version}",
        "",
        f"{DRAFT_MARKER}: made by changelog.py start-release. Write the release's summary, the known "
        "limitations, the unconfirmed Makera Z1 values and the known issues (docs/releasing.md), edit the "
        "changes below into groups for users, then delete this comment. -->",
        "",
        "### What has changed",
        "",
        *(entries or ["- Nothing user-visible has changed since the last release."]),
        "",
        reminder,
        "",
    ]
    first = min([s for h, s, _ in _sections(lines) if _version_key(h) or h == DEV_HEADING])
    out = lines[:first] + new + [line for line in lines[first:start]] + dev + lines[end:]
    if start < first:  # Development version above the releases: keep the page's order, new section first.
        out = lines[:start] + new + dev + lines[end:]
    result = "\n".join(out)
    while "\n\n\n" in result:
        result = result.replace("\n\n\n", "\n\n")
    return result.rstrip("\n") + "\n"


# --------------------------------------------------------------------------------------- command line

def _git(root, *args, check=True):
    r = subprocess.run(["git", *args], cwd=root, capture_output=True, text=True)
    if check and r.returncode != 0:
        raise ChangelogError(f"git {' '.join(args)}: {r.stderr.strip()}")
    return r


def _show(root, rev, path):
    r = _git(root, "show", f"{rev}:{path}", check=False)
    return r.stdout if r.returncode == 0 else ""


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--repo-root", default=REPO_ROOT, help=argparse.SUPPRESS)
    sub = ap.add_subparsers(dest="cmd", required=True)
    n = sub.add_parser("nightly-notes", help="write the notes of the nightly pre-release")
    n.add_argument("--sha", required=True)
    n.add_argument("--date", required=True)
    n.add_argument("--repo", default=DEFAULT_REPO)
    n.add_argument("--out", help="the file to write (default: standard output)")
    p = sub.add_parser("check-pr", help="check a pull request's changes (the description in $PR_BODY)")
    p.add_argument("--base", required=True, help="the pull request's base commit")
    p.add_argument("--head", default="HEAD")
    s = sub.add_parser("start-release", help="move the Development version into a draft section for VERSION")
    s.add_argument("version")
    c = sub.add_parser("check-release", help="the release-notes checks of a release")
    c.add_argument("version")
    args = ap.parse_args(argv)
    root = args.repo_root
    notes_file = os.path.join(root, NOTES_PATH)
    try:
        if args.cmd == "nightly-notes":
            with open(notes_file, encoding="utf-8") as f:
                out = nightly_notes(f.read(), read_notice(root), args.sha, args.date, args.repo)
            if args.out:
                with open(args.out, "w", encoding="utf-8") as f:
                    f.write(out)
            else:
                sys.stdout.write(out)
            return 0
        if args.cmd == "check-pr":
            mb = _git(root, "merge-base", args.base, args.head).stdout.strip()
            files = [f for f in _git(root, "diff", "--name-only", mb, args.head).stdout.splitlines() if f]
            problems = pr_problems(files, _show(root, mb, NOTES_PATH), _show(root, args.head, NOTES_PATH),
                                   os.environ.get("PR_BODY", ""))
            if problems:
                print("::error title=Changelog::This pull request needs a changelog entry or a reason why not "
                      "(see below).")
                for line in problems:
                    print(f"- {line}")
                return 1
            print(f"Changelog: {len(files)} changed files checked; nothing missing.")
            return 0
        with open(notes_file, encoding="utf-8") as f:
            text = f.read()
        if args.cmd == "start-release":
            new = start_release(text, args.version)
            with open(notes_file, "w", encoding="utf-8") as f:
                f.write(new)
            print(f"Moved {_entries(len(development_entries(text)))} into a draft section for {args.version} in "
                  f"{NOTES_PATH}. Write the section, then delete its draft comment.")
            return 0
        problems = release_problems(text, args.version)
        for line in problems:
            print(f"error: {line}")
        return 1 if problems else 0
    except ChangelogError as e:
        print(f"error: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
