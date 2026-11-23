// SPDX-License-Identifier: AGPL-3.0-or-later
//! The changelog since the last release (decision record 0678): the nightly notes, the
//! pull request check and the release step.
//!
//! - `cargo xtask changelog nightly-notes --sha SHA --date YYYYMMDD [--repo OWNER/NAME] [--out FILE]`
//! - `cargo xtask changelog check-pr --base SHA [--head REV]` (the description in `$PR_BODY`)
//! - `cargo xtask changelog start-release VERSION`
//! - `cargo xtask changelog check-release VERSION`
//!
//! The changelog is the *Development version* section of `user-docs/src/release-notes.md`: one
//! top-level bullet per change that users could notice since the last release, in plain words. The
//! nightly pre-release notes are made from it; each pull request that changes what users see adds or
//! changes a bullet, or says `No user-visible change: <reason>` in its description; a release moves
//! the bullets into the version's own section and leaves the Development version empty.
//!
//! Generated G-code is the user's to verify before it is run on a machine (NFR-12): nothing written
//! here says otherwise, and the notes carry the program's own notice. The output is deterministic.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{Context, Result, bail};

use crate::Findings;

/// The release-notes page, relative to the workspace root.
pub const NOTES_PATH: &str = "user-docs/src/release-notes.md";
/// The heading of the changelog section.
const DEV_HEADING: &str = "Development version";
/// The latest user guide, where the guide's relative links point in the notes.
pub const GUIDE: &str = "https://openchuck.cam/latest/";
/// GitHub accepts a release body of 125,000 characters; the notes stay well below it.
pub const NOTES_LIMIT: usize = 60_000;
/// The crates published to crates.io (`docs/releasing.md`); a change to one needs its own
/// `CHANGELOG.md` entry (INT-07).
const FAMILY: [&str; 4] = [
    "openchuck-types",
    "openchuck-path",
    "openchuck-post",
    "openchuck-gcode",
];
/// The escape hatch, a line of its own in the pull request description.
const HATCH: &str = "no user-visible change:";
/// The first words of the comment `start-release` puts in a draft section.
const DRAFT_MARKER: &str = "<!-- release-notes draft";
/// The closing reminder of a section when the page has none to copy (NFR-12).
const REMINDER: &str = "Generated G-code must be verified by you before it is run on a machine. OpenChuckCAM \
does not guarantee that a program is safe or correct for any machine, tool or setup.";

// --------------------------------------------------------------------------------------- parsing

/// One `## ` section: its heading text and its lines `[start, end)`, the heading included.
#[derive(Debug)]
struct Section {
    heading: String,
    start: usize,
    end: usize,
}

fn sections(lines: &[&str]) -> Vec<Section> {
    let starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("## "))
        .map(|(i, _)| i)
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(k, &start)| Section {
            heading: lines
                .get(start)
                .and_then(|l| l.get(3..))
                .unwrap_or("")
                .trim()
                .to_string(),
            start,
            end: starts.get(k + 1).copied().unwrap_or(lines.len()),
        })
        .collect()
}

fn dev_section(lines: &[&str]) -> Option<Section> {
    sections(lines)
        .into_iter()
        .find(|s| s.heading == DEV_HEADING)
}

fn is_bullet(line: &str) -> bool {
    line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ")
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

/// The `[first, last)` line spans of the top-level bullets between `start` and `end`. An entry goes
/// on over its continuation lines, and over blank lines when the next text is indented under it.
fn entry_spans(lines: &[&str], start: usize, end: usize) -> Vec<(usize, usize)> {
    let line = |i: usize| lines.get(i).copied().unwrap_or("");
    let mut spans = Vec::new();
    let mut i = start;
    while i < end {
        if !is_bullet(line(i)) {
            i += 1;
            continue;
        }
        let first = i;
        i += 1;
        let mut last = i;
        while i < end {
            if is_bullet(line(i)) {
                break;
            }
            if is_blank(line(i)) {
                let mut j = i;
                while j < end && is_blank(line(j)) {
                    j += 1;
                }
                if j < end && line(j).starts_with([' ', '\t']) {
                    i = j;
                    continue;
                }
                break;
            }
            i += 1;
            last = i;
        }
        spans.push((first, last));
    }
    spans
}

fn join(lines: &[&str], a: usize, b: usize) -> String {
    lines
        .get(a..b)
        .unwrap_or(&[])
        .join("\n")
        .trim_end()
        .to_string()
}

/// The bullets of the Development version section, each with its continuation lines.
pub fn development_entries(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    match dev_section(&lines) {
        Some(s) => entry_spans(&lines, s.start + 1, s.end)
            .into_iter()
            .map(|(a, b)| join(&lines, a, b))
            .collect(),
        None => Vec::new(),
    }
}

/// `X.Y.Z` as numbers, or None.
fn version_key(v: &str) -> Option<(u64, u64, u64)> {
    let parts: Vec<&str> = v.split('.').collect();
    match parts.as_slice() {
        [a, b, c]
            if [a, b, c]
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|x| x.is_ascii_digit())) =>
        {
            Some((a.parse().ok()?, b.parse().ok()?, c.parse().ok()?))
        }
        _ => None,
    }
}

/// The version a `## ` heading names: its first word when that is `X.Y.Z`, so `## 0.60.0` and
/// `## 0.60.0 (2026-11-23)` name the same release. One rule for every command.
fn heading_version(heading: &str) -> Option<&str> {
    heading
        .split_whitespace()
        .next()
        .filter(|v| version_key(v).is_some())
}

/// The highest version that has a `## X.Y.Z` section, or None before the first release.
pub fn latest_release(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    sections(&lines)
        .iter()
        .filter_map(|s| heading_version(&s.heading))
        .filter_map(|v| version_key(v).map(|k| (k, v.to_string())))
        .max()
        .map(|(_, v)| v)
}

// ---------------------------------------------------------------------------------- nightly notes

/// `a/./b/../c` as `a/c`, for a relative path.
fn normalise(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|p| *p != "..") {
                    parts.pop();
                } else {
                    parts.push("..");
                }
            }
            p => parts.push(p),
        }
    }
    parts.join("/")
}

/// Whether `target` starts with a URL scheme (`https:`, `mailto:`).
fn has_scheme(target: &str) -> bool {
    let Some(colon) = target.find(':') else {
        return false;
    };
    let scheme = target.get(..colon).unwrap_or("");
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
}

fn rewrite_target(target: &str, guide: &str) -> String {
    if has_scheme(target) || target.starts_with('/') {
        return target.to_string();
    }
    let (path, frag) = match target.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (target, None),
    };
    let path = if path.is_empty() {
        "release-notes.md"
    } else {
        path
    };
    let path = match path.strip_suffix(".md") {
        Some(stem) => format!("{stem}.html"),
        None => path.to_string(),
    };
    let mut out = format!("{guide}{}", normalise(&path));
    if let Some(f) = frag.filter(|f| !f.is_empty()) {
        out.push('#');
        out.push_str(f);
    }
    out
}

/// Whether `rest`, what follows a link's destination, closes the link: `)` at once, or a title in
/// double or single quotes and then `)` (CommonMark), which is kept as it is.
fn link_end(rest: &str) -> bool {
    if rest.starts_with(')') {
        return true;
    }
    let title = rest.trim_start();
    if title.len() == rest.len() {
        return false;
    }
    let Some(quote) = title.chars().next().filter(|c| matches!(c, '"' | '\'')) else {
        return false;
    };
    let after_open = title.get(1..).unwrap_or("");
    after_open
        .find(quote)
        .and_then(|close| after_open.get(close + 1..))
        .is_some_and(|tail| tail.trim_start().starts_with(')'))
}

/// Relative links of the guide's source, `](target)`, become links to the latest guide.
fn rewrite_links(text: &str, guide: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("](") {
        let (before, after) = rest.split_at(at + 2);
        out.push_str(before);
        let end = after.find(|c: char| c == ')' || c.is_whitespace());
        match end {
            Some(e) if e > 0 && link_end(after.get(e..).unwrap_or("")) => {
                out.push_str(&rewrite_target(after.get(..e).unwrap_or(""), guide));
                rest = after.get(e..).unwrap_or("");
            }
            _ => rest = after,
        }
    }
    out.push_str(rest);
    out
}

fn chars(s: &str) -> usize {
    s.chars().count()
}

/// The body of the rolling `nightly` pre-release, at most `limit` characters.
pub fn nightly_notes(
    text: &str,
    notice: &str,
    sha: &str,
    date: &str,
    repo: &str,
    guide: &str,
    limit: usize,
) -> Result<String> {
    let latest = latest_release(text);
    let entries: Vec<String> = development_entries(text)
        .iter()
        .map(|e| rewrite_links(e, guide))
        .collect();
    let notes_link = format!("{guide}release-notes.html#development-version");
    let mut head = vec![
        format!("Automatic development build of `main` at {sha}, built {date}."),
        String::new(),
        "**This is not a release.** It passed no release gate and may be incomplete or broken. It is \
         replaced by the next nightly build."
            .to_string(),
        String::new(),
        format!("**{notice}**"),
        String::new(),
        "## What has changed".to_string(),
        String::new(),
    ];
    match (&latest, entries.is_empty()) {
        (Some(v), false) => head.push(format!(
            "Changes since the last release, {v}, as listed in the [release notes of the latest user guide]({notes_link}):"
        )),
        (Some(v), true) => head.push(format!(
            "Nothing user-visible has changed since the last release, {v}. The [release notes]({notes_link}) of \
             the latest user guide list each release."
        )),
        (None, _) => head.push(format!(
            "There has been no release yet. The changes listed so far in the [release notes of the latest user guide]({notes_link}):"
        )),
    }
    head.push(String::new());
    let crates: Vec<String> = FAMILY
        .iter()
        .map(|c| format!("[{c}](https://github.com/{repo}/blob/{sha}/crates/{c}/CHANGELOG.md)"))
        .collect();
    let tail = vec![
        String::new(),
        format!("Changes to the library crates, for developers, are in their changelogs: {}.", crates.join(", ")),
        String::new(),
        "## Downloads".to_string(),
        String::new(),
        "Each archive holds `openchuck-cli`, `openchuck-ui`, the licence, the third-party notices and a README. \
         `openchuckcam-nightly-*-examples.zip` holds the example projects. The binaries are not signed: macOS \
         quarantines them and Windows SmartScreen may warn (see the README in each archive)."
            .to_string(),
        String::new(),
    ];
    // The notes with the first `kept` entries, and a line for the rest when any are left out.
    let render = |kept: usize| -> String {
        let mut body: Vec<String> = entries.iter().take(kept).cloned().collect();
        if kept < entries.len() {
            body.push(String::new());
            body.push(format!(
                "{} more changes are in the [release notes of the latest user guide]({notes_link}).",
                entries.len() - kept
            ));
        }
        let mut out = [head.clone(), body, tail.clone()].concat().join("\n");
        while out.contains("\n\n\n") {
            out = out.replace("\n\n\n", "\n\n");
        }
        out
    };
    // Every entry if they fit; otherwise as many as fit with the line for the rest, which is
    // counted with them: the first that fits, from the most entries down.
    let out = (0..=entries.len())
        .rev()
        .map(render)
        .find(|out| chars(out) <= limit);
    match out {
        Some(out) => Ok(out),
        None => bail!("the nightly notes cannot fit in {limit} characters"),
    }
}

/// The program's own G-code notice (`crates/openchuck-types/src/notice.rs`), as the guide reads it.
pub fn read_notice(root: &Path) -> Result<String> {
    let path = root.join("crates/openchuck-types/src/notice.rs");
    let rust =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let start = "VERIFY_GCODE_NOTICE: &str = \"";
    let Some(from) = rust.find(start).map(|i| i + start.len()) else {
        bail!("could not read VERIFY_GCODE_NOTICE from notice.rs");
    };
    let raw = rust.get(from..).unwrap_or("");
    let Some(to) = raw.find("\";") else {
        bail!("could not read VERIFY_GCODE_NOTICE from notice.rs");
    };
    // A backslash at the end of a line continues the string without the newline or the indent.
    let mut notice = String::new();
    let mut parts = raw.get(..to).unwrap_or("").split("\\\n");
    if let Some(first) = parts.next() {
        notice.push_str(first);
    }
    for part in parts {
        notice.push_str(part.trim_start());
    }
    Ok(notice)
}

/// Writes the nightly notes from the release notes at `root` to `out`, or returns them.
pub fn write_nightly_notes(
    root: &Path,
    sha: &str,
    date: &str,
    repo: &str,
    out: Option<&Path>,
) -> Result<String> {
    let path = root.join(NOTES_PATH);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let notes = nightly_notes(
        &text,
        &read_notice(root)?,
        sha,
        date,
        repo,
        GUIDE,
        NOTES_LIMIT,
    )?;
    if let Some(out) = out {
        std::fs::write(out, &notes).with_context(|| format!("writing {}", out.display()))?;
    }
    Ok(notes)
}

// ---------------------------------------------------------------------------- pull request check

/// Whether a change to this file can change what users see or download.
pub fn triggers(path: &str) -> bool {
    if path == NOTES_PATH {
        return false;
    }
    if path.starts_with("user-docs/src/") {
        // The screenshot records (CAPTURED.toml) are read by the checks, not shown to users.
        return !path.ends_with(".toml");
    }
    if path.starts_with("testdata/examples/") || path.starts_with("crates/openchuck-post/posts/") {
        return true;
    }
    let parts: Vec<&str> = path.splitn(4, '/').collect();
    match parts.as_slice() {
        ["apps", _, "src" | "assets", _] => true,
        ["crates", name, "src", _] => *name != "openchuck-testkit",
        _ => false,
    }
}

/// The library-family crate whose published contents this file is part of, or None.
pub fn family_crate(path: &str) -> Option<&'static str> {
    let parts: Vec<&str> = path.splitn(4, '/').collect();
    match parts.as_slice() {
        ["crates", name, folder, _] => FAMILY
            .iter()
            .find(|c| *c == name)
            .copied()
            .filter(|c| *folder == "src" || (*folder == "posts" && *c == "openchuck-post")),
        _ => None,
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The first decision, open item or requirement number in `text` (numbers that mean nothing to
/// users; the owner's rule). Issue numbers (#577) are allowed.
fn internal_number(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let at = |i: usize| chars.get(i).copied();
    let boundary_before = |i: usize| i == 0 || at(i - 1).is_none_or(|c| !is_word(c));
    let boundary_after = |i: usize| at(i).is_none_or(|c| !is_word(c));
    let digits_from = |i: usize| {
        chars
            .get(i..)
            .unwrap_or(&[])
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count()
    };
    let lower: Vec<char> = chars.iter().map(|c| c.to_ascii_lowercase()).collect();
    for i in 0..chars.len() {
        if !boundary_before(i) {
            continue;
        }
        // "decision 0656", "records 0400": a word, white space, three or four digits.
        for word in ["decisions", "decision", "records", "record"] {
            let w: Vec<char> = word.chars().collect();
            if lower.get(i..i + w.len()) == Some(&w[..]) {
                let mut j = i + w.len();
                let space = chars
                    .get(j..)
                    .unwrap_or(&[])
                    .iter()
                    .take_while(|c| c.is_whitespace())
                    .count();
                if space == 0 {
                    continue;
                }
                j += space;
                let n = digits_from(j);
                if (3..=4).contains(&n) && boundary_after(j + n) {
                    return Some(chars.get(i..j + n).unwrap_or(&[]).iter().collect());
                }
            }
        }
        // "OI-933", and requirement IDs such as "GC-07" and "NFR-12": a prefix that
        // docs/requirements.md defines, a hyphen and two or three digits (an open item any number
        // of digits). Other capitals and a number, such as an ER-32 collet, are not caught.
        let caps = chars
            .get(i..)
            .unwrap_or(&[])
            .iter()
            .take_while(|c| c.is_ascii_uppercase())
            .count();
        if at(i + caps) == Some('-') {
            let n = digits_from(i + caps + 1);
            let end = i + caps + 1 + n;
            let open_item = chars.get(i..i + caps) == Some(&['O', 'I'][..]) && n >= 1;
            let prefix: String = chars.get(i..i + caps).unwrap_or(&[]).iter().collect();
            let requirement = (2..=3).contains(&n) && is_requirement_prefix(&prefix);
            if (open_item || requirement) && boundary_after(end) {
                return Some(chars.get(i..end).unwrap_or(&[]).iter().collect());
            }
        }
    }
    None
}

/// The requirement-ID prefixes that `docs/requirements.md` defines (`GC`, `NFR`, `UI` and so on),
/// read once. When the file cannot be read, any two to four capitals count, so nothing is missed.
fn requirement_prefixes() -> Option<&'static BTreeSet<String>> {
    static PREFIXES: OnceLock<Option<BTreeSet<String>>> = OnceLock::new();
    PREFIXES
        .get_or_init(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent()?;
            let text = std::fs::read_to_string(root.join("docs/requirements.md")).ok()?;
            let prefixes: BTreeSet<String> = text
                .lines()
                .filter_map(|l| l.strip_prefix("| "))
                .filter_map(|l| l.split_once('-'))
                .filter(|(p, rest)| {
                    (2..=4).contains(&p.len())
                        && p.bytes().all(|b| b.is_ascii_uppercase())
                        && rest.starts_with(|c: char| c.is_ascii_digit())
                })
                .map(|(p, _)| p.to_string())
                .collect();
            (!prefixes.is_empty()).then_some(prefixes)
        })
        .as_ref()
}

fn is_requirement_prefix(prefix: &str) -> bool {
    match requirement_prefixes() {
        Some(known) => known.contains(prefix),
        None => (2..=4).contains(&prefix.len()),
    }
}

fn squash(entry: &str) -> String {
    entry.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The reason given on an escape-hatch line of the description, if there is one.
fn hatch_reason(body: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let plain: String = line
            .chars()
            .filter(|c| !matches!(c, '*' | '_' | '`'))
            .collect();
        let plain = plain.trim();
        let plain = match plain.strip_prefix(['-', '*', '+']) {
            Some(rest) if rest.starts_with(char::is_whitespace) => rest.trim_start(),
            _ => plain,
        };
        let lower = plain.to_lowercase();
        if !lower.starts_with(HATCH) {
            return None;
        }
        let reason = plain.get(HATCH.len()..).unwrap_or("").trim();
        (!reason.is_empty() && reason != "<reason>").then(|| reason.to_string())
    })
}

fn plural_entries(n: usize) -> String {
    if n == 1 {
        "1 entry".to_string()
    } else {
        format!("{n} entries")
    }
}

/// What a pull request is missing, as plain sentences; empty when it passes.
pub fn pr_problems(files: &[String], base_text: &str, head_text: &str, body: &str) -> Findings {
    let old: BTreeSet<String> = development_entries(base_text)
        .iter()
        .map(|e| squash(e))
        .collect();
    let new_entries: Vec<String> = development_entries(head_text)
        .into_iter()
        .filter(|e| !old.contains(&squash(e)))
        .collect();
    let mut problems = Findings::new();
    for e in &new_entries {
        if let Some(number) = internal_number(e) {
            let first: String = e.lines().next().unwrap_or("").chars().take(80).collect();
            problems.push(format!(
                "the Development version entry '{first}' carries the internal number '{number}': release notes \
                 carry no decision, open item or requirement numbers (issue numbers such as #577 are fine)"
            ));
        }
    }
    if hatch_reason(body).is_some() {
        return problems;
    }
    let mut visible: Vec<&String> = files.iter().filter(|f| triggers(f)).collect();
    visible.sort();
    if visible.is_empty() {
        return problems;
    }
    let crates: BTreeSet<&str> = visible.iter().filter_map(|f| family_crate(f)).collect();
    let missing: Vec<&str> = crates
        .into_iter()
        .filter(|c| {
            !files
                .iter()
                .any(|f| *f == format!("crates/{c}/CHANGELOG.md"))
        })
        .collect();
    if !new_entries.is_empty() && missing.is_empty() {
        return problems;
    }
    let mut shown: String = visible
        .iter()
        .take(10)
        .map(|f| f.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    if visible.len() > 10 {
        shown.push_str(&format!(" and {} more", visible.len() - 10));
    }
    problems.push(format!(
        "this pull request changes files that can change what users see: {shown}"
    ));
    if new_entries.is_empty() {
        problems.push(format!(
            "add or change a bullet under '## {DEV_HEADING}' in {NOTES_PATH}: what changed, in plain words for \
             users, with no decision, open item or requirement numbers"
        ));
    }
    for c in missing {
        problems.push(format!(
            "{c} is published to crates.io: add an entry for this change to crates/{c}/CHANGELOG.md"
        ));
    }
    problems.push(
        "or, if users cannot notice the change, put a line 'No user-visible change: <reason>' in the pull request \
         description. The check reads the description when the pull request is opened, on each push and when it \
         is marked ready for review; editing the description alone does not run it again"
            .to_string(),
    );
    problems
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))?;
    if !out.status.success() {
        bail!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The check of a pull request from `base` to `head` in the repository at `root`: the changes are
/// taken from their merge base, so what reached `main` after the branch was made is not counted.
pub fn check_pr(root: &Path, base: &str, head: &str, body: &str) -> Result<Findings> {
    let merge_base = git(root, &["merge-base", base, head])?.trim().to_string();
    // NUL-separated, so a name Git would quote (`café.md`) is read as it is; no rename detection,
    // so a file moved out of a covered folder counts as removed from it.
    let status = git(
        root,
        &[
            "diff",
            "--name-status",
            "--no-renames",
            "-z",
            &merge_base,
            head,
        ],
    )?;
    let mut files = Vec::new();
    let mut fields = status.split('\0').filter(|f| !f.is_empty());
    while let (Some(kind), Some(path)) = (fields.next(), fields.next()) {
        // A deleted crate changelog is not an entry for the crate.
        if kind == "D" && path.ends_with("/CHANGELOG.md") {
            continue;
        }
        files.push(path.to_string());
    }
    let show = |rev: &str| git(root, &["show", &format!("{rev}:{NOTES_PATH}")]).unwrap_or_default();
    Ok(pr_problems(&files, &show(&merge_base), &show(head), body))
}

// --------------------------------------------------------------------------------- the release

/// Whether a section's lines hold written text: a line that is not blank, not a heading and not
/// inside an HTML comment (which can run over several lines).
fn has_written_text(lines: &[&str]) -> bool {
    let mut in_comment = false;
    for line in lines {
        let mut rest = *line;
        let mut text = String::new();
        loop {
            if in_comment {
                match rest.find("-->") {
                    Some(end) => {
                        in_comment = false;
                        rest = rest.get(end + 3..).unwrap_or("");
                    }
                    None => break,
                }
            } else {
                match rest.find("<!--") {
                    Some(start) => {
                        text.push_str(rest.get(..start).unwrap_or(""));
                        in_comment = true;
                        rest = rest.get(start + 4..).unwrap_or("");
                    }
                    None => {
                        text.push_str(rest);
                        break;
                    }
                }
            }
        }
        let text = text.trim();
        if !text.is_empty() && !text.starts_with('#') {
            return true;
        }
    }
    false
}

/// At a release: the version has its own section, written (not `start-release`'s draft), and the
/// Development version has no entries left.
pub fn release_problems(text: &str, version: &str) -> Findings {
    let lines: Vec<&str> = text.lines().collect();
    let mut problems = Findings::new();
    let section = sections(&lines)
        .into_iter()
        .find(|s| heading_version(&s.heading) == Some(version));
    match section {
        None => problems.push(format!(
            "the release notes have no section headed {version}"
        )),
        Some(s)
            if lines
                .get(s.start..s.end)
                .unwrap_or(&[])
                .iter()
                .any(|l| l.contains(DRAFT_MARKER)) =>
        {
            problems.push(format!(
                "the section for {version} is still the draft made by start-release: write it, then delete the \
                 draft comment"
            ));
        }
        Some(s) if !has_written_text(lines.get(s.start + 1..s.end).unwrap_or(&[])) => {
            problems.push(format!(
                "the section for {version} has nothing written in it: write what changed, the known limitations \
                 and the known issues (docs/releasing.md)"
            ));
        }
        Some(_) => {}
    }
    let left = development_entries(text).len();
    if left > 0 {
        problems.push(format!(
            "the {DEV_HEADING} section still has {}: move them into the section for {version} (cargo xtask \
             changelog start-release does it)",
            plural_entries(left)
        ));
    }
    problems
}

/// Moves the Development version entries into a new draft `## <version>` section above the newest
/// release; the Development version keeps its heading and paragraphs.
pub fn start_release(text: &str, version: &str) -> Result<String> {
    let Some(key) = version_key(version) else {
        bail!("'{version}' is not a version (X.Y.Z)");
    };
    if let Some(latest) = latest_release(text)
        && version_key(&latest).is_some_and(|l| key <= l)
    {
        bail!("{version} is not newer than the last release, {latest}");
    }
    let lines: Vec<&str> = text.lines().collect();
    let Some(dev) = dev_section(&lines) else {
        bail!("the release notes have no '## {DEV_HEADING}' section");
    };
    let spans = entry_spans(&lines, dev.start + 1, dev.end);
    let entries: Vec<String> = spans.iter().map(|&(a, b)| join(&lines, a, b)).collect();
    let reminder = lines
        .get(dev.start..dev.end)
        .unwrap_or(&[])
        .iter()
        .rev()
        .find(|l| l.starts_with("Generated G-code"))
        .map_or(REMINDER.to_string(), |l| (*l).to_string());

    let dropped = |i: usize| spans.iter().any(|&(a, b)| (a..b).contains(&i));
    let kept: Vec<String> = (dev.start..dev.end)
        .filter(|&i| !dropped(i))
        .filter_map(|i| lines.get(i).map(|l| (*l).to_string()))
        .collect();

    let mut new = vec![
        format!("## {version}"),
        String::new(),
        format!(
            "{DRAFT_MARKER}: made by cargo xtask changelog start-release. Write the release's summary, the known \
             limitations, the unconfirmed Makera Z1 values and the known issues (docs/releasing.md), edit the \
             changes below into groups for users, then delete this comment. -->"
        ),
        String::new(),
        "### What has changed".to_string(),
        String::new(),
    ];
    if entries.is_empty() {
        new.push("- Nothing user-visible has changed since the last release.".to_string());
    } else {
        new.extend(entries);
    }
    new.extend([String::new(), reminder, String::new()]);

    let to_strings = |a: usize, b: usize| -> Vec<String> {
        lines
            .get(a..b)
            .unwrap_or(&[])
            .iter()
            .map(|l| (*l).to_string())
            .collect()
    };
    let first = sections(&lines)
        .iter()
        .filter(|s| heading_version(&s.heading).is_some() || s.heading == DEV_HEADING)
        .map(|s| s.start)
        .min()
        .unwrap_or(dev.start);
    let out: Vec<String> = if first < dev.start {
        [
            to_strings(0, first),
            new,
            to_strings(first, dev.start),
            kept,
            to_strings(dev.end, lines.len()),
        ]
        .concat()
    } else {
        [
            to_strings(0, dev.start),
            new,
            kept,
            to_strings(dev.end, lines.len()),
        ]
        .concat()
    };
    let mut result = out.join("\n");
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }
    Ok(format!("{}\n", result.trim_end_matches('\n')))
}

fn read_notes(root: &Path) -> Result<String> {
    let path = root.join(NOTES_PATH);
    std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
}

/// `start-release` on the page at `root`; returns how many entries moved.
pub fn start_release_file(root: &Path, version: &str) -> Result<usize> {
    let text = read_notes(root)?;
    let new = start_release(&text, version)?;
    let path = root.join(NOTES_PATH);
    std::fs::write(&path, new).with_context(|| format!("writing {}", path.display()))?;
    Ok(development_entries(&text).len())
}

/// `check-release` on the page at `root`.
pub fn check_release(root: &Path, version: &str) -> Result<Findings> {
    Ok(release_problems(&read_notes(root)?, version))
}

/// For the command line: the number of entries, in words.
pub fn entries_in_words(n: usize) -> String {
    plural_entries(n)
}

#[cfg(test)]
mod tests;
