// SPDX-License-Identifier: AGPL-3.0-or-later
//! Rendering the neutral program to text, and the `PostProcessor` trait
//! (GC-02, GC-03, GC-07; architecture section 5.3).

use std::collections::BTreeMap;

use minijinja::value::Serde;
use serde::Serialize;

use openchuck_types::ir::Toolpath;
use openchuck_types::kinematics::RotaryFrame;

use crate::convert::{ConvertOptions, ToolSetting, to_machine_program};
use crate::definition::PostDefinition;
use crate::engine;
use crate::error::{PostError, Result};
use crate::options::{ADirection, PostOptions, UnwindMode};
use crate::program::{MachineProgram, Step};

/// What a post needs to know about the job beyond the toolpath.
#[derive(Debug, Clone)]
pub struct PostJob {
    /// Machine profile settings for GC-17, before post overrides.
    pub block_time: crate::BlockTimeSettings,
    /// Allowed rotary travel; limited axes never receive counter resets.
    pub a_range: openchuck_types::machine::ARange,
    /// Required safe Z in the output work frame, for operation-boundary
    /// counter resets (decision 0241).
    pub safe_z: f64,
    /// The work frame the program is written in.
    pub frame: RotaryFrame,
    /// Tool numbers and spindle speeds by the IR's tool tag (GC-14).
    pub tools: BTreeMap<String, ToolSetting>,
    /// The program title, written as a comment.
    pub title: String,
    /// Display names of operations by the IR's operation tag, written in the
    /// comments that name each operation (GC-06). A tag with no name is
    /// written as it is.
    pub operation_names: BTreeMap<String, String>,
    /// Spindle speeds by the IR's operation tag; an operation with an entry
    /// runs at it, any other at its tool's speed (decision 0196).
    pub operation_spindle: BTreeMap<String, f64>,
    /// A revision identifier for the header, when supplied (VC-06).
    pub revision: Option<String>,
    /// The tool and its version, written as a comment after the title, such as
    /// `OpenChuckCAM 0.10.0` (NFR-16). The host passes the software version only, never a
    /// commit or a date, so one version always gives the same program (GC-07, NFR-06).
    pub generator: Option<String>,
    /// The setup's work offset register (`G54` to `G59`), which replaces the
    /// post's own `work_offset` when given (SET-05).
    pub work_offset: Option<String>,
}

/// The result of posting.
#[derive(Debug, Clone, PartialEq)]
pub struct PostOutput {
    /// Command-rate demand and mitigation per operation group.
    pub short_blocks: Vec<crate::ShortBlockReport>,
    /// IR with posted durations when Slow down is active, for replay comparison.
    pub verification_path: Toolpath,
    /// Original-IR complete-window diagnostics in Report mode (compatibility API).
    pub block_time_reports: Vec<crate::BlockTimeReport>,
    /// The program text.
    pub text: String,
    /// Fed moves whose feed was lowered to the post's maximum.
    pub clamped_feeds: usize,
    /// Sum of posted durations in Slow down; the original IR estimate otherwise, seconds.
    pub estimated_seconds: f64,
    /// Each text the post wrote with [`ACTIVE_COMMENT_GUARD`] in front, once per source and
    /// place, in source order, so the host can warn the user (issue #851).
    pub guarded_comments: Vec<GuardedComment>,
}

/// Where a text that the post guarded came from (issue #851).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommentSource {
    /// [`PostJob::title`].
    Title,
    /// [`PostJob::revision`].
    Revision,
    /// [`PostJob::generator`].
    Generator,
    /// An operation's name in [`PostJob::operation_names`], by the IR's operation tag.
    Operation(String),
    /// A tool's name in [`PostJob::tools`], by the IR's tool tag.
    Tool(String),
    /// A comment of the post's own (its header, footer or template) or of the program's own
    /// words, such as a wrapped line of a header comment.
    Post,
}

/// A text the post wrote with [`ACTIVE_COMMENT_GUARD`] in front, because it would otherwise
/// have started a comment the controller acts on (issue #851). The program text carries only
/// the mark; this report is what the host shows the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardedComment {
    /// Where the text came from.
    pub source: CommentSource,
    /// The entry of the post's `active_comment_prefixes` it matched, as the post lists it.
    pub opening: String,
    /// The text as the job gave it (before sanitising); for [`CommentSource::Post`], the
    /// comment as the template wrote it.
    pub text: String,
    /// Where in `text` the mark was written, in characters.
    pub at: usize,
}

/// Something that turns a toolpath into program text (GC-02, GC-13). The
/// template implementation is first; a scripting engine can follow.
pub trait PostProcessor {
    /// A stable name.
    fn name(&self) -> &str;
    /// Posts a toolpath.
    fn post(&self, toolpath: &Toolpath, job: &PostJob) -> Result<PostOutput>;
}

/// The NFR-12 notice a post writes as comment lines after the generator line when its
/// `safety_notice` option is on (decision 0533, OI-2000). Fixed text, so no post can reword
/// it, with no date, clock or commit, so it keeps the output deterministic (GC-07, NFR-06).
/// It does not say the program is safe or checked: the user verifies it (NFR-12).
pub const SAFETY_NOTICE: &[&str] = &[
    "This program has not been run on your machine. Check it before you run it.",
    "Simulate it and air cut it first. You are responsible for what the machine does.",
];

/// Whether the program opens with the notice: every line of [`SAFETY_NOTICE`], in order,
/// as whole comment lines (after an optional `N` word) in the comment block at the top,
/// before the first line that is not a single comment and nothing else (so `(title) G90`
/// ends it, issue #839). Text that only appears inside a longer line, out of order or after
/// the program has started does not count (decision 0533).
fn writes_notice(text: &str) -> bool {
    let header: Vec<&str> = text
        .lines()
        .map(|line| match line.strip_prefix('N') {
            Some(rest) => match rest.split_once(' ') {
                Some((number, body)) if number.chars().all(|c| c.is_ascii_digit()) => body,
                _ => line,
            },
            None => line,
        })
        // A comment-only line: one comment and nothing else, so `(title) G90` ends the block
        // (issue #839).
        .take_while(|line| {
            line.strip_prefix('(')
                .and_then(|r| r.strip_suffix(')'))
                .is_some_and(|inner| !inner.contains(['(', ')']))
        })
        .collect();
    let wanted: Vec<String> = SAFETY_NOTICE.iter().map(|n| format!("({n})")).collect();
    header
        .windows(wanted.len())
        .any(|block| block.iter().zip(&wanted).all(|(line, want)| line == want))
}

/// The mark written in front of a comment that would otherwise start with one of the post's
/// `active_comment_prefixes`, so the controller reads it as an ordinary comment (issue #851).
/// LinuxCNC skips only white space before it compares a comment with its words, so one
/// printable character is enough.
pub const ACTIVE_COMMENT_GUARD: char = '_';

/// Whether `text`, after its leading white space, starts with `prefix`, ASCII letters in any
/// case (LinuxCNC compares its interpreter's words in lower case; matching every word in any
/// case also covers its task's upper-case ones).
pub(crate) fn starts_with_prefix(text: &str, prefix: &str) -> bool {
    text.trim_start()
        .as_bytes()
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix.as_bytes()))
}

/// The first of `prefixes` that a comment holding `text` would start with, so that the
/// controller would act on it (issue #851).
fn active_opening<'p>(text: &str, prefixes: &'p [String]) -> Option<&'p String> {
    prefixes.iter().find(|p| starts_with_prefix(text, p))
}

/// Where a wrapped comment line was guarded: the comment's text, the character in it where
/// the guarded line starts, and the opening matched.
struct WrapMark {
    inner: String,
    at: usize,
    opening: String,
}

/// Splits `text` at spaces into pieces of at most `first` characters for the first piece and
/// `rest` for the others; a word longer than its piece's width is cut. Runs of spaces become
/// one. A piece that starts anywhere but at the start of `text` and would start an active
/// comment gets [`ACTIVE_COMMENT_GUARD`] in front, counted in its width (issue #851); the
/// start of `text` is the template's to write. Deterministic (GC-07).
/// Also returns, for each guarded piece, the character of `text` where it starts and the
/// opening matched.
fn wrap_words(
    text: &str,
    first: usize,
    rest: usize,
    prefixes: &[String],
) -> (Vec<String>, Vec<(usize, String)>) {
    let (first, rest) = (first.max(1), rest.max(1));
    let mut pieces: Vec<String> = Vec::new();
    let mut marks: Vec<(usize, String)> = Vec::new();
    let mut current: Vec<char> = Vec::new();
    let mut at_start = true;
    let chars: Vec<char> = text.chars().collect();
    let mut next = 0;
    while next < chars.len() {
        // The next word and the character of `text` it starts at, as `split_whitespace`.
        let Some(skip) = chars
            .get(next..)
            .and_then(|rest| rest.iter().position(|c| !c.is_whitespace()))
        else {
            break;
        };
        let mut start = next + skip;
        let end = chars
            .get(start..)
            .and_then(|rest| rest.iter().position(|c| c.is_whitespace()))
            .map_or(chars.len(), |n| start + n);
        next = end;
        let mut word: Vec<char> = chars.get(start..end).unwrap_or_default().to_vec();
        loop {
            // The piece being filled is `current`, the first while nothing is pushed.
            let width = if pieces.is_empty() { first } else { rest };
            if current.is_empty() {
                let opening = if at_start {
                    None
                } else {
                    active_opening(&word.iter().collect::<String>(), prefixes)
                };
                let guard = opening.is_some();
                if let Some(opening) = opening {
                    marks.push((start, opening.clone()));
                }
                let mark = usize::from(guard);
                if word.len() + mark <= width {
                    if guard {
                        current.push(ACTIVE_COMMENT_GUARD);
                    }
                    current.append(&mut word);
                    at_start = false;
                    break;
                }
                if pieces.is_empty() && word.len() <= rest {
                    // Too long for the narrow first line but not for a full one: keep it
                    // whole on the next line, and leave the first line an empty comment.
                    pieces.push(String::new());
                    continue;
                }
                let mut piece: String = if guard {
                    ACTIVE_COMMENT_GUARD.to_string()
                } else {
                    String::new()
                };
                let taken = width.saturating_sub(mark).max(1);
                piece.extend(word.drain(..taken));
                start += taken;
                pieces.push(piece);
                at_start = false;
            } else if current.len() + 1 + word.len() <= width {
                current.push(' ');
                current.append(&mut word);
                break;
            } else {
                pieces.push(std::mem::take(&mut current).into_iter().collect());
            }
        }
    }
    if !current.is_empty() {
        pieces.push(current.into_iter().collect());
    }
    (pieces, marks)
}

/// Wraps every comment line longer than `max` characters (a whole line `(...)`, after an
/// optional `N` word) into comment lines that fit, at spaces (decision 0656, OI-933). The
/// `N` word stays on the first of them, and a later line that would start an active comment
/// is guarded (issue #851). Other lines are left as they are: one that is too long is the
/// GC-08 check's to report.
///
/// Also returns where a wrapped line was guarded.
fn wrap_comments(text: &str, max: usize, prefixes: &[String]) -> (String, Vec<WrapMark>) {
    let mut out = String::with_capacity(text.len());
    let mut marks = Vec::new();
    for line in text.split_inclusive('\n') {
        let (body, end) = match line.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (line, ""),
        };
        if body.chars().count() <= max {
            out.push_str(line);
            continue;
        }
        let (prefix, rest) = match body.strip_prefix('N').and_then(|r| r.split_once(' ')) {
            Some((number, rest))
                if !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) =>
            {
                body.split_at(body.len() - rest.len())
            }
            _ => ("", body),
        };
        let inner = rest
            .strip_prefix('(')
            .and_then(|r| r.strip_suffix(')'))
            .filter(|inner| !inner.contains(['(', ')']));
        let Some(inner) = inner else {
            out.push_str(line);
            continue;
        };
        // Only the first line carries the `N` word; the others have the whole width.
        let first = max.saturating_sub(prefix.chars().count() + 2);
        let rest = max.saturating_sub(2);
        let (pieces, guarded) = wrap_words(inner, first, rest, prefixes);
        marks.extend(guarded.into_iter().map(|(at, opening)| WrapMark {
            inner: inner.to_owned(),
            at,
            opening,
        }));
        for (i, piece) in pieces.iter().enumerate() {
            if i == 0 {
                out.push_str(prefix);
            } else {
                out.push('\n');
            }
            out.push('(');
            out.push_str(piece);
            out.push(')');
        }
        out.push_str(end);
    }
    (out, marks)
}

/// Where `needle` occurs in `hay`, by character.
fn char_spans(hay: &[char], needle: &[char]) -> Vec<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return Vec::new();
    }
    (0..=hay.len() - needle.len())
        .filter(|&i| hay.get(i..i + needle.len()) == Some(needle))
        .collect()
}

/// Whether `value` is written at the start of a comment in `text`: after a `(` and nothing but
/// spaces.
fn starts_a_comment(text: &str, value: &str) -> bool {
    text.match_indices(value).any(|(i, _)| {
        text[..i]
            .trim_end_matches(char::is_whitespace)
            .ends_with('(')
    })
}

/// A text from the job that the template may write, as the template is given it.
struct Given {
    source: CommentSource,
    /// The text as the job gave it.
    original: String,
    /// The text the template is given: sanitised, and guarded when it started an active comment.
    given: Vec<char>,
    /// When it was guarded: the number of leading spaces dropped.
    guarded_lead: Option<usize>,
}

impl Given {
    /// `at` in the given text, as a character of the original.
    fn original_at(&self, at: usize) -> usize {
        match self.guarded_lead {
            Some(lead) => (at + lead).saturating_sub(1),
            None => at,
        }
    }
}

/// Replaces what a G-code comment cannot hold: parentheses, control
/// characters and anything that is not ASCII (decision 0019 item 4, OI-84).
pub fn sanitise_comment(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '(' => '[',
            ')' => ']',
            c if c.is_ascii() && !c.is_ascii_control() => c,
            c if c.is_control() => ' ',
            _ => '?',
        })
        .collect()
}

#[derive(Serialize)]
struct Context<'a> {
    title: String,
    revision: Option<String>,
    generator: Option<String>,
    safety_notice: &'static [&'static str],
    opt: &'a PostOptions,
    steps: &'a [Step],
}

fn map_a(step: &mut Step, f: impl Fn(f64) -> f64) {
    match step {
        Step::Rapid { target } | Step::Feed { target, .. } => target.a = f(target.a),
        Step::RotaryBoundary { at } => at.a = f(at.a),
        _ => {}
    }
}

/// Applies the post's A conventions to the neutral program: direction, and
/// how whole turns are removed.
fn apply_a_conventions(program: &MachineProgram, options: &PostOptions) -> Vec<Step> {
    let sign = match options.a_direction {
        ADirection::Same => 1.0,
        ADirection::Reversed => -1.0,
    };
    let mut steps = program.steps.clone();
    for step in &mut steps {
        map_a(step, |a| a * sign);
    }
    match options.unwind {
        UnwindMode::Rotate => steps
            .into_iter()
            .filter(|step| !matches!(step, Step::RotaryBoundary { .. }))
            .collect(),
        UnwindMode::DropTurns => {
            // The controller keeps the remainder of A with its sign, which may
            // differ from the IR's chosen turn; shifting what follows by whole
            // turns keeps every machine position the same.
            let mut shift = 0.0;
            steps
                .into_iter()
                .filter_map(|mut step| match &mut step {
                    Step::RotaryBoundary { at } => {
                        let held = at.a + shift;
                        // Firmware 1.1.2 leaves exactly +/-360 alone. The
                        // following A words must use that retained counter,
                        // or an intended short move gains a full turn.
                        if held.abs() <= 360.0 {
                            return None;
                        }
                        let remainder = held % 360.0;
                        shift = remainder - at.a;
                        at.a = remainder;
                        Some(step)
                    }
                    _ => {
                        map_a(&mut step, |a| a + shift);
                        Some(step)
                    }
                })
                .collect()
        }
    }
}

/// A post defined by a template and options (GC-03).
#[derive(Debug, Clone, PartialEq)]
pub struct TemplatePost {
    definition: PostDefinition,
}

impl TemplatePost {
    /// Wraps a checked definition.
    pub fn new(definition: PostDefinition) -> Self {
        Self { definition }
    }

    /// The definition.
    pub fn definition(&self) -> &PostDefinition {
        &self.definition
    }
}

impl PostProcessor for TemplatePost {
    fn name(&self) -> &str {
        &self.definition.name
    }

    fn post(&self, toolpath: &Toolpath, job: &PostJob) -> Result<PostOutput> {
        let d = &self.definition;
        let (short_blocks, sources) = d.options.effective_short_blocks(job.block_time.into());
        let convert = ConvertOptions {
            short_blocks,
            feed_decimals: Some(d.options.decimals.f),
            a_range: job.a_range,
            safe_z: job.safe_z,
            frame: job.frame,
            feed_rule: d.feed_rule,
            max_feed: d.options.max_feed,
            tools: job.tools.clone(),
            decimals: Some((d.options.decimals.pos, d.options.decimals.a)),
            operation_names: job.operation_names.clone(),
            operation_spindle: job.operation_spindle.clone(),
            combine_rotary_runs: d.options.combine_rotary_runs,
        };
        let mut program = to_machine_program(toolpath, &convert)?;
        for r in &mut program.short_blocks {
            r.sources = sources;
        }
        // Text from the project never starts an active comment (issue #851): each text the
        // template is given is guarded where it would start one, and remembered, so a guard
        // that reaches the program is reported with its source.
        let active = &d.options.active_comment_prefixes;
        let mut candidates: Vec<(GuardedComment, String)> = Vec::new();
        let mut safe = |text: &str, source: CommentSource| -> String {
            let clean = sanitise_comment(text);
            match active_opening(&clean, active) {
                Some(opening) => {
                    let lead = clean.chars().take_while(|c| c.is_whitespace()).count();
                    let guarded = format!("{ACTIVE_COMMENT_GUARD}{}", clean.trim_start());
                    candidates.push((
                        GuardedComment {
                            source,
                            opening: opening.clone(),
                            text: text.to_owned(),
                            at: lead,
                        },
                        guarded.clone(),
                    ));
                    guarded
                }
                None => clean,
            }
        };
        let tool_tag = |number: u32| {
            job.tools
                .iter()
                .find(|(_, t)| t.number == number)
                .map(|(tag, _)| tag.clone())
        };
        for step in &mut program.steps {
            match step {
                // The operation comment is the program's own words around the names.
                Step::Comment { text } => *text = safe(text, CommentSource::Post),
                Step::ToolChange { name, number } => {
                    let source = tool_tag(*number).map_or(CommentSource::Post, CommentSource::Tool);
                    *name = safe(name, source);
                }
                _ => {}
            }
        }
        // The program's own comment lines (each operation's), as the template is given them: a
        // wrapped line of one is attributed to the names inside it only.
        let program_comments: std::collections::BTreeSet<String> = program
            .steps
            .iter()
            .filter_map(|step| match step {
                Step::Comment { text } => Some(text.clone()),
                _ => None,
            })
            .collect();
        let mut options = d.options.clone();
        if let Some(offset) = &job.work_offset {
            options.work_offset = Some(offset.clone());
            options.validate()?;
        }
        let steps = apply_a_conventions(&program, &options);
        let context = Context {
            title: safe(&job.title, CommentSource::Title),
            revision: job
                .revision
                .as_deref()
                .map(|t| safe(t, CommentSource::Revision)),
            generator: job
                .generator
                .as_deref()
                .map(|t| safe(t, CommentSource::Generator)),
            safety_notice: if options.safety_notice {
                SAFETY_NOTICE
            } else {
                &[]
            },
            opt: &options,
            steps: &steps,
        };
        // Held to the end of this function: dropping it frees any namespace
        // the template left holding itself (issue #794).
        let (mut env, _namespaces) = engine::environment(steps.len())?;
        env.add_template_owned(d.template_name.clone(), d.template.clone())
            .map_err(|e| PostError::Template(engine::describe(&e)))?;
        let template = env
            .get_template(&d.template_name)
            .map_err(|e| PostError::Template(engine::describe(&e)))?;
        let text = template
            .render(Serde(&context))
            .map_err(|e| PostError::Template(engine::describe(&e)))?;
        if options.safety_notice && !writes_notice(&text) {
            return Err(PostError::Setting {
                name: "options.safety_notice".into(),
                reason: format!(
                    "is on, but the template {} does not write the notice; write each line of \
                     `safety_notice` as a comment after the generator line, or set \
                     safety_notice = false",
                    d.template_name
                ),
            });
        }
        // Long comments are wrapped after the notice check, so the notice is checked as the
        // template wrote it and kept word for word (decision 0656).
        // A guarded text is reported when the template wrote it at the start of a comment.
        let mut guarded: BTreeMap<(CommentSource, usize, String), GuardedComment> = BTreeMap::new();
        for (report, value) in std::mem::take(&mut candidates) {
            if starts_a_comment(&text, &value) {
                guarded
                    .entry((report.source.clone(), report.at, report.text.clone()))
                    .or_insert(report);
            }
        }
        let (text, marks) = match options.max_line_length {
            Some(max) => wrap_comments(&text, max as usize, active),
            None => (text, Vec::new()),
        };
        // A wrapped line that was guarded is reported against the text from the job that holds
        // its start; one held by none is the post's own.
        let sanitised = |s: &str| sanitise_comment(s).chars().collect::<Vec<char>>();
        let mut sources: Vec<Given> = Vec::new();
        let mut give = |source: CommentSource, original: &str| {
            let clean = sanitise_comment(original);
            let guarded_lead = active_opening(&clean, active)
                .map(|_| clean.chars().take_while(|c| c.is_whitespace()).count());
            let given = match guarded_lead {
                Some(_) => format!("{ACTIVE_COMMENT_GUARD}{}", clean.trim_start()),
                None => clean,
            };
            sources.push(Given {
                source,
                original: original.to_owned(),
                given: given.chars().collect(),
                guarded_lead,
            });
        };
        give(CommentSource::Title, &job.title);
        if let Some(revision) = &job.revision {
            give(CommentSource::Revision, revision);
        }
        if let Some(generator) = &job.generator {
            give(CommentSource::Generator, generator);
        }
        // Names inside the program's operation comment are written sanitised, not guarded.
        for (tag, name) in &job.operation_names {
            sources.push(Given {
                source: CommentSource::Operation(tag.clone()),
                original: name.clone(),
                given: sanitised(name),
                guarded_lead: None,
            });
        }
        for (tag, tool) in &job.tools {
            sources.push(Given {
                source: CommentSource::Tool(tag.clone()),
                original: tool.name.clone(),
                given: sanitised(&tool.name),
                guarded_lead: None,
            });
        }
        for mark in marks {
            let inner: Vec<char> = mark.inner.chars().collect();
            // In an operation comment, only the names it holds; elsewhere, the title, revision and
            // generator first, so a name that equals another's text is reported for its own line.
            let in_program_comment = program_comments.contains(&mark.inner);
            let names = |given: &&Given| {
                matches!(
                    given.source,
                    CommentSource::Operation(_) | CommentSource::Tool(_)
                )
            };
            let held = sources
                .iter()
                .filter(|given| names(given) == in_program_comment)
                .chain(
                    sources
                        .iter()
                        .filter(|given| !in_program_comment && names(given)),
                )
                .find_map(|given| {
                    char_spans(&inner, &given.given)
                        .into_iter()
                        .find(|&start| mark.at >= start && mark.at < start + given.given.len())
                        .map(|start| (given, mark.at - start))
                });
            let report = match held {
                Some((given, at)) => GuardedComment {
                    source: given.source.clone(),
                    opening: mark.opening,
                    text: given.original.clone(),
                    at: given.original_at(at),
                },
                None => GuardedComment {
                    source: CommentSource::Post,
                    opening: mark.opening,
                    text: mark.inner,
                    at: mark.at,
                },
            };
            guarded
                .entry((report.source.clone(), report.at, report.text.clone()))
                .or_insert(report);
        }
        let estimated_seconds = if short_blocks.enabled()
            && short_blocks.mode == openchuck_types::machine::ShortBlockMode::SlowDown
        {
            program.verification_path.total_duration().0
        } else {
            toolpath.total_duration().0
        };
        Ok(PostOutput {
            short_blocks: program.short_blocks,
            verification_path: program.verification_path,
            block_time_reports: if short_blocks.mode
                == openchuck_types::machine::ShortBlockMode::Report
            {
                crate::block_time_reports(toolpath, d.options.block_time(job.block_time))?
            } else {
                Vec::new()
            },
            text,
            clamped_feeds: program.clamped_feeds,
            estimated_seconds,
            guarded_comments: guarded.into_values().collect(),
        })
    }
}
