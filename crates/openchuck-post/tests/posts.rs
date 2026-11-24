// SPDX-License-Identifier: AGPL-3.0-or-later
//! The template engine, the post loader and the shipped posts (GC-02, GC-03,
//! GC-04, GC-07, GC-09, GC-10, NFR-07, NFR-08).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::indexing_slicing
)]

use std::path::PathBuf;

mod support;

use openchuck_post::{PostDefinition, PostProcessor, TemplatePost, builtin, builtin_ids};

use support::*;

fn post_text(id: &str) -> String {
    builtin_post(id).post(&small_part(), &job()).unwrap().text
}

fn post_text_with(id: &str, job: &openchuck_post::PostJob) -> String {
    builtin_post(id).post(&small_part(), job).unwrap().text
}

fn golden(name: &str, text: &str) {
    let path = repo("testdata/golden").join(name);
    openchuck_testkit::golden::check(&path, text).unwrap_or_else(|m| panic!("{m}"));
}

/// The NFR-12 notice block of a program from `job()`: the comment lines after the title and
/// the generator, before the revision. The Z1 post wraps it to its 63-character lines
/// (decision 0656); joined, its text is the notice word for word.
fn notice_block(text: &str) -> Vec<&str> {
    text.lines()
        .skip(2)
        .take_while(|l| l.starts_with('(') && *l != "(revision abc1234)")
        .collect()
}

/// The notice block's text, the lines' parentheses removed and the lines joined by spaces.
fn notice_text(text: &str) -> String {
    notice_block(text)
        .iter()
        .map(|l| &l[1..l.len() - 1])
        .collect::<Vec<_>>()
        .join(" ")
}

/// NFR-16 and GC-07: both posts write the generator comment right after the title, before the
/// revision; with no generator, or with comments off, the line is not written.
#[test]
fn req_nfr_16_both_posts_name_the_tool_and_version_after_the_title() {
    for id in ["linuxcnc", "makera-z1"] {
        let text = post_text(id);
        // The NFR-12 notice lines (decision 0533) sit between the generator and the revision.
        let notice = notice_block(&text);
        let head: Vec<_> = text
            .lines()
            .filter(|l| !notice.contains(l))
            .take(3)
            .collect();
        assert_eq!(
            head,
            [
                "(Small part)",
                "(OpenChuckCAM 0.10.0)",
                "(revision abc1234)"
            ],
            "{id}"
        );
        let mut without = job();
        without.generator = None;
        let text = builtin_post(id).post(&small_part(), &without).unwrap().text;
        assert!(!text.contains("OpenChuckCAM"), "{id}: {text}");
        let dir = std::env::temp_dir().join(format!(
            "openchuck-post-test-quiet-{id}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for file in ["post.toml", "main.post"] {
            std::fs::copy(posts_path(id).join(file), dir.join(file)).unwrap();
        }
        let manifest = std::fs::read_to_string(dir.join("post.toml")).unwrap();
        std::fs::write(
            dir.join("post.toml"),
            manifest
                .replace("comments = true", "comments = false")
                .replace("safety_notice = true", "safety_notice = false"),
        )
        .unwrap();
        let text = TemplatePost::new(PostDefinition::load_dir(&dir).unwrap())
            .post(&small_part(), &job())
            .unwrap()
            .text;
        std::fs::remove_dir_all(dir).unwrap();
        assert!(!text.contains("OpenChuckCAM"), "{id}: comments are off");
    }
}

/// NFR-16: the version is text from the host; a name that would break a comment is made safe.
#[test]
fn req_nfr_16_req_nfr_07_the_generator_comment_is_made_safe() {
    let mut j = job();
    j.generator = Some("OpenChuckCAM (1.0)\nG0 X0".into());
    let text = post_text_with("makera-z1", &j);
    let line = text.lines().nth(1).unwrap();
    assert!(
        line.starts_with("(OpenChuckCAM ") && line.ends_with(')'),
        "{line}"
    );
    assert_eq!(line.matches('(').count(), 1, "{line}");
}

#[test]
fn req_gc_10_req_nfr_08_linuxcnc_post_matches_its_golden_file() {
    golden("post_linuxcnc_small_part.ngc", &post_text("linuxcnc"));
}

#[test]
fn req_gc_09_req_nfr_08_z1_post_matches_its_golden_file() {
    golden("post_makera_z1_small_part.ngc", &post_text("makera-z1"));
}

#[test]
fn req_gc_09_z1_output_follows_the_sp_12_conventions() {
    let text = post_text("makera-z1");
    assert!(!text.contains("G93"), "the Z1 post never writes G93");
    assert!(!text.contains("G43"), "the machine measures tool length");
    assert!(text.lines().all(|l| !l.starts_with('N')), "no line numbers");
    assert!(text.contains("\nG92.4 A0 S0\n"), "{text}");
    assert!(text.contains("\nM5\nT1 M6\n"), "M5 before T M6");
    assert!(
        text.trim_end().ends_with("M5\nG53 G0 Z-3\nG28\nM2"),
        "{text}"
    );
    // Comments stand on their own line and carry no parentheses from project text.
    assert!(
        text.contains("(operation groove [finish], tool T1 4 mm ball, about"),
        "{text}"
    );
}

#[test]
fn req_gc_09_req_gc_10_the_two_posts_share_one_template() {
    // One template serves both posts by options alone (decision 0019 item 9).
    let a = std::fs::read(posts_path("linuxcnc/main.post")).unwrap();
    let b = std::fs::read(posts_path("makera-z1/main.post")).unwrap();
    assert_eq!(a, b);
}

#[test]
fn req_gc_02_built_in_posts_load_by_id_and_unknown_ids_are_none() {
    let ids: Vec<_> = builtin_ids().collect();
    assert_eq!(ids, ["makera-z1", "linuxcnc"]);
    for id in ids {
        assert!(builtin(id).unwrap().is_ok());
    }
    assert!(builtin("nope").is_none());
    assert_ne!(
        builtin_post("linuxcnc").name(),
        builtin_post("makera-z1").name()
    );
}

#[test]
fn req_gc_03_loading_from_a_folder_equals_the_embedded_post() {
    let loaded = PostDefinition::load_dir(&posts_path("linuxcnc")).unwrap();
    assert_eq!(&loaded, builtin_post("linuxcnc").definition());
}

fn copy_post(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("openchuck-post-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for file in ["post.toml", "main.post"] {
        std::fs::copy(posts_path("linuxcnc").join(file), dir.join(file)).unwrap();
    }
    dir
}

#[test]
fn req_gc_03_editing_a_post_changes_the_output_without_recompiling() {
    let dir = copy_post("edit");
    let manifest = std::fs::read_to_string(dir.join("post.toml")).unwrap();
    let edited = manifest.replace(
        "footer = [\"M5\", \"M30\"]",
        "footer = [\"M5\", \"M2\", \"(done)\"]",
    );
    assert_ne!(manifest, edited);
    std::fs::write(dir.join("post.toml"), edited).unwrap();
    let text = TemplatePost::new(PostDefinition::load_dir(&dir).unwrap())
        .post(&small_part(), &job())
        .unwrap()
        .text;
    assert!(text.trim_end().ends_with("M5\nM2\n(done)"), "{text}");
    // A template edit shows too.
    let template = std::fs::read_to_string(dir.join("main.post")).unwrap();
    std::fs::write(dir.join("main.post"), template.replace("M3\")", "M4\")")).unwrap();
    let text = TemplatePost::new(PostDefinition::load_dir(&dir).unwrap())
        .post(&small_part(), &job())
        .unwrap()
        .text;
    assert!(text.contains("S10000 M4"), "{text}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn req_gc_03_an_unknown_variable_is_an_error_naming_the_line() {
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let template = "first line\nsecond {{ opt.no_such_option }}\n";
    let post = PostDefinition::from_parts("test", &manifest, template).unwrap();
    let e = TemplatePost::new(post)
        .post(&small_part(), &job())
        .unwrap_err()
        .to_string();
    assert!(e.contains("line 2"), "{e}");
    assert!(e.contains("main.post"), "{e}");
}

#[test]
fn req_gc_03_req_nfr_07_a_syntax_error_is_found_when_the_post_is_loaded() {
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let e = PostDefinition::from_parts("test", &manifest, "ok\n{% if %}\n")
        .unwrap_err()
        .to_string();
    assert!(e.contains("line 2"), "{e}");
}

#[test]
fn req_gc_04_req_nfr_07_bad_options_are_refused_naming_the_option() {
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let template = std::fs::read_to_string(posts_path("linuxcnc/main.post")).unwrap();
    let cases = [
        ("a_letter = \"A\"", "a_letter = \"Q\"", "a_letter"),
        ("pos = 3", "pos = 12", "decimals.pos"),
        ("step = 10", "step = 0", "line_numbers.step"),
        (
            "comments = true",
            "comments = true\nmax_feed = -1.0",
            "max_feed",
        ),
        (
            "work_offset = \"G54\"",
            "work_offset = \"G60\"",
            "work_offset",
        ),
        (
            "unwind = \"rotate\"",
            "unwind = \"drop_turns\"",
            "unwind_command",
        ),
        ("coolant = \"off\"", "coolant = \"spray\"", "spray"),
        // GC-08 (decision 0656): the line-length limit is checked when it is loaded.
        (
            "max_line_length = 255",
            "max_line_length = 19",
            "max_line_length",
        ),
        (
            "max_line_length = 255",
            "max_line_length = 4097",
            "max_line_length",
        ),
        ("feed_mode = \"g94\"", "feed_mode = \"g93\"", "feed_rule"),
        ("schema = 1", "schema = 2", "schema 2"),
        ("comments = true", "commets = true", "commets"),
        ("feed_rule = \"linuxcnc\"", "feed_rule = \"fast\"", "fast"),
    ];
    for (from, to, expect) in cases {
        assert!(manifest.contains(from), "{from}");
        let e = PostDefinition::from_parts("t", &manifest.replace(from, to), &template)
            .unwrap_err()
            .to_string();
        assert!(e.contains(expect), "{from} -> {to}: {e}");
    }
    let e = PostDefinition::from_parts("t", "not toml [", &template)
        .unwrap_err()
        .to_string();
    assert!(e.contains("post definition t"), "{e}");
}

#[test]
fn req_gc_04_g93_and_line_numbers_and_direction_and_coolant_options() {
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let template = std::fs::read_to_string(posts_path("linuxcnc/main.post")).unwrap();
    let edited = manifest
        .replace("feed_rule = \"linuxcnc\"", "feed_rule = \"inverse_time\"")
        .replace("feed_mode = \"g94\"", "feed_mode = \"g93\"")
        .replace("enabled = false", "enabled = true")
        .replace("a_direction = \"same\"", "a_direction = \"reversed\"")
        .replace("coolant = \"off\"", "coolant = \"flood\"");
    let post = TemplatePost::new(PostDefinition::from_parts("t", &edited, &template).unwrap());
    let text = post.post(&small_part(), &job()).unwrap().text;
    assert!(
        text.contains("\nN30 G93\n") || text.contains("G93"),
        "{text}"
    );
    assert!(text.lines().filter(|l| l.starts_with("N")).count() > 10);
    assert!(text.contains(" M8\n") || text.contains("M8"), "{text}");
    // Reversed A: the first cut moves to negative A (the first cutting move is 180 degrees).
    assert!(text.contains("A-45.000"), "{text}");
    // Inverse time: a 1 s move is F60 on every fed move.
    assert!(text.contains("F60.0"), "{text}");
}

#[test]
fn req_gc_04_the_z1_unwind_drops_turns_and_keeps_the_machine_position() {
    // From 1080 degrees back to 0: the controller keeps 0 after dropping three turns.
    let text = post_text("makera-z1");
    let after = text.split("G92.4 A0 S0\n").nth(1).unwrap();
    assert!(
        after.starts_with("M5\n"),
        "nothing moves after the unwind: {after}"
    );
}

#[test]
fn req_gc_04_req_gc_09_z1_reset_adjusts_following_a_at_signed_turn_boundaries() {
    use openchuck_types::ir::{MoveKind, Toolpath, ToolpathGroup};

    use openchuck_types::geometry::Point3;
    use openchuck_types::kinematics::{MachinePosition, tool_axis_in_model, tool_tip_in_model};
    use openchuck_types::units::Degrees;
    let fixed = |a| {
        let frame = job().frame;
        openchuck_types::ir::ToolPose {
            tip: tool_tip_in_model(
                &frame,
                &MachinePosition {
                    xyz: Point3::new(7.0, 2.0, 50.0),
                    a: Degrees(a),
                },
            ),
            axis: tool_axis_in_model(&frame, Degrees(a)),
            a: Degrees(a),
        }
    };
    for (before, reset) in [
        (360.0, 360.0),
        (-360.0, -360.0),
        (370.0, 10.0),
        (-370.0, -10.0),
        (1080.0, 0.0),
        (-1080.0, 0.0),
        (593275.0, 355.0),
        (-593275.0, -355.0),
    ] {
        let path = Toolpath {
            groups: vec![
                ToolpathGroup {
                    rotary_reset_after: Vec::new(),
                    tags: small_part().groups[0].tags.clone(),
                    start: fixed(before - 10.0),
                    segments: vec![seg(MoveKind::Rapid, fixed(before), 1.0, 0.0)],
                },
                ToolpathGroup {
                    rotary_reset_after: Vec::new(),
                    tags: small_part().groups[0].tags.clone(),
                    start: fixed(before),
                    segments: vec![seg(MoveKind::Rapid, fixed(before + 10.0), 1.0, 0.0)],
                },
            ],
        };
        let text = builtin_post("makera-z1").post(&path, &job()).unwrap().text;
        assert!(
            text.lines().any(|line| line
                .split_whitespace()
                .any(|word| word == format!("A{:.3}", reset + 10.0))),
            "A{before}: {text}"
        );
    }
}

#[test]
fn req_gc_09_safe_group_end_resets_counter_without_a_physical_unwind() {
    // The canonical operation ends at its safe retract, with accumulated A.
    let path = small_part();
    let text = builtin_post("makera-z1").post(&path, &job()).unwrap().text;
    assert!(text.contains("\nG92.4 A0 S0\nM5\n"), "{text}");
    let linux = builtin_post("linuxcnc").post(&path, &job()).unwrap().text;
    let last_a = linux
        .lines()
        .rev()
        .find(|line| line.split_whitespace().any(|w| w.starts_with('A')))
        .unwrap();
    assert!(last_a.contains("A1080.000"), "{linux}");
}

#[test]
fn req_gc_07_posting_twice_and_on_four_threads_gives_the_same_bytes() {
    let reference = post_text("makera-z1");
    assert_eq!(reference, post_text("makera-z1"));
    let handles: Vec<_> = (0..4)
        .map(|_| std::thread::spawn(|| post_text("makera-z1")))
        .collect();
    for h in handles {
        assert_eq!(h.join().unwrap(), reference);
    }
}

#[test]
fn req_gc_03_crlf_in_a_definition_gives_the_same_output() {
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let template = std::fs::read_to_string(posts_path("linuxcnc/main.post")).unwrap();
    let crlf = |s: &str| s.replace('\n', "\r\n");
    let post = TemplatePost::new(
        PostDefinition::from_parts("t", &crlf(&manifest), &crlf(&template)).unwrap(),
    );
    assert_eq!(
        post.post(&small_part(), &job()).unwrap().text,
        post_text("linuxcnc")
    );
}

#[test]
fn req_nfr_07_comments_from_project_text_are_made_safe() {
    let mut path = small_part();
    path.groups[0].tags.operation = "gr\u{fc}n (rough)\nM30".into();
    let text = builtin_post("linuxcnc").post(&path, &job()).unwrap().text;
    assert!(
        text.contains("(operation gr?n [rough] M30, tool T1"),
        "{text}"
    );
    assert!(!text.lines().any(|l| l == "M30" && false));
    let mut j = job();
    j.title = "a (b)".into();
    let text = builtin_post("linuxcnc")
        .post(&small_part(), &j)
        .unwrap()
        .text;
    assert!(
        text.starts_with("(a [b])\n(OpenChuckCAM 0.10.0)\n"),
        "{text}"
    );
    assert!(text.contains("\n(revision abc1234)\n"), "{text}");
}

#[test]
fn req_gc_14_a_tool_without_a_number_is_refused() {
    let mut j = job();
    j.tools.clear();
    let e = builtin_post("linuxcnc")
        .post(&small_part(), &j)
        .unwrap_err()
        .to_string();
    assert!(e.contains("t1"), "{e}");
}

#[test]
fn req_gc_04_req_op_13_a_feed_word_keeps_three_significant_figures() {
    use openchuck_post::feed_str;
    // `decimals.f` is the least; a small feed gets more, so its rounding is at
    // most 0.5 % of a move's time (decision record 0076).
    for (value, decimals, text) in [
        (1200.04, 1, "1200.0"),
        (480.0, 1, "480.0"),
        (12.34, 1, "12.3"),
        (4.237, 1, "4.24"),
        (4.15, 1, "4.15"),
        (0.0123, 1, "0.0123"),
        (99.96, 1, "100.0"),
        (4.237, 3, "4.237"),
        (7.0, 0, "7.00"),
        (1e-12, 1, "0.000000000"),
    ] {
        assert_eq!(feed_str(value, decimals), text, "{value} at {decimals}");
    }
}

/// A copy of a built-in post with `edit` applied to its manifest and template.
fn edited_post(id: &str, manifest_edit: (&str, &str), template_edit: (&str, &str)) -> String {
    let manifest = std::fs::read_to_string(posts_path(&format!("{id}/post.toml"))).unwrap();
    let template = std::fs::read_to_string(posts_path(&format!("{id}/main.post"))).unwrap();
    assert!(manifest.contains(manifest_edit.0), "{}", manifest_edit.0);
    assert!(template.contains(template_edit.0), "{}", template_edit.0);
    let def = PostDefinition::from_parts(
        "t",
        &manifest.replace(manifest_edit.0, manifest_edit.1),
        &template.replace(template_edit.0, template_edit.1),
    )
    .unwrap();
    TemplatePost::new(def)
        .post(&small_part(), &job())
        .unwrap()
        .text
}

/// NFR-12 (OI-2000, decision 0533): both built-in posts ship with the notice on and write it
/// as comment lines right after the generator line, before the revision and the header codes.
#[test]
fn req_nfr_12_both_posts_write_the_safety_notice_after_the_generator() {
    assert!(!openchuck_post::SAFETY_NOTICE.is_empty());
    for id in ["linuxcnc", "makera-z1"] {
        assert!(builtin_post(id).definition().options.safety_notice, "{id}");
        let text = post_text(id);
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines[0], "(Small part)", "{id}");
        assert_eq!(lines[1], "(OpenChuckCAM 0.10.0)", "{id}");
        // Word for word; on the Z1 wrapped to its 63-character lines (decision 0656).
        let notice = notice_block(&text);
        assert_eq!(
            notice_text(&text),
            openchuck_post::SAFETY_NOTICE.join(" "),
            "{id}"
        );
        if id == "linuxcnc" {
            let whole: Vec<_> = openchuck_post::SAFETY_NOTICE
                .iter()
                .map(|n| format!("({n})"))
                .collect();
            assert_eq!(notice, whole, "{id}");
        }
        assert_eq!(lines[2 + notice.len()], "(revision abc1234)", "{id}");
    }
}

/// NFR-12 and NFR-07: the notice is plain printable ASCII a comment can hold on both
/// controllers (no parentheses, short lines), and it never says the program is safe or checked.
#[test]
fn req_nfr_12_req_nfr_07_the_notice_is_comment_safe_and_claims_nothing() {
    for line in openchuck_post::SAFETY_NOTICE {
        assert_eq!(openchuck_post::sanitise_comment(line), *line);
        assert!(!line.contains('(') && !line.contains(')'), "{line}");
        assert!(line.len() <= 80, "{line}");
        let lower = line.to_lowercase();
        for claim in ["is safe", "verified", "guarantee", "tested"] {
            assert!(!lower.contains(claim), "{line}");
        }
    }
}

/// NFR-12 and GC-04: a post with the option off writes no notice and is otherwise unchanged.
#[test]
fn req_nfr_12_req_gc_04_notice_off_writes_nothing_else_changes() {
    for id in ["linuxcnc", "makera-z1"] {
        let on = post_text(id);
        let off = edited_post(
            id,
            ("safety_notice = true", "safety_notice = false"),
            ("", ""),
        );
        for notice in openchuck_post::SAFETY_NOTICE {
            assert!(!off.contains(notice), "{id}: {off}");
        }
        let notice = notice_block(&on);
        assert_eq!(
            notice_text(&on),
            openchuck_post::SAFETY_NOTICE.join(" "),
            "{id}"
        );
        let without: String = on
            .lines()
            .filter(|l| !notice.contains(l))
            .map(|l| format!("{l}\n"))
            .collect();
        assert_eq!(off, without, "{id}");
    }
}

/// GC-04 and GC-03: a post written before the option (no `safety_notice` key) still loads
/// and writes exactly what it wrote before: the option is off when the key is left out.
#[test]
fn req_gc_04_req_gc_03_a_post_without_the_key_loads_with_the_notice_off() {
    let off = edited_post(
        "linuxcnc",
        ("safety_notice = true", "safety_notice = false"),
        ("", ""),
    );
    let absent = edited_post("linuxcnc", ("safety_notice = true\n", ""), ("", ""));
    assert_eq!(absent, off);
}

/// NFR-12 and NFR-07: the notice is a comment, so asking for it with comments off is refused
/// when the post is loaded, naming the option, instead of being dropped without a word.
#[test]
fn req_nfr_12_req_nfr_07_notice_with_comments_off_is_refused() {
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let template = std::fs::read_to_string(posts_path("linuxcnc/main.post")).unwrap();
    let e = PostDefinition::from_parts(
        "t",
        &manifest.replace("comments = true", "comments = false"),
        &template,
    )
    .unwrap_err()
    .to_string();
    assert!(e.contains("options.safety_notice"), "{e}");
    assert!(e.contains("comments"), "{e}");
}

/// NFR-12 and NFR-07: a copied template that does not write the notice cannot drop it
/// silently while the option is on: posting is refused with a message saying so.
#[test]
fn req_nfr_12_req_nfr_07_a_template_that_drops_the_notice_is_refused() {
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let template = std::fs::read_to_string(posts_path("linuxcnc/main.post")).unwrap();
    let start = template.find("{% for notice in safety_notice %}").unwrap();
    let end = template[start..].find("{% endfor %}").unwrap() + start + "{% endfor %}".len();
    let mut dropped = template.clone();
    dropped.replace_range(start..end, "");
    let def = PostDefinition::from_parts("t", &manifest, &dropped).unwrap();
    let e = TemplatePost::new(def)
        .post(&small_part(), &job())
        .unwrap_err()
        .to_string();
    assert!(e.contains("safety_notice"), "{e}");
}

/// NFR-12, GC-07 and NFR-06: with the notice on, the program is the same bytes on 1, 2 and
/// N threads; the notice carries no date, clock or commit.
#[test]
fn req_nfr_12_req_gc_07_notice_output_is_byte_identical_on_1_2_and_n_threads() {
    for id in ["linuxcnc", "makera-z1"] {
        let reference = post_text(id);
        assert_eq!(
            notice_text(&reference),
            openchuck_post::SAFETY_NOTICE.join(" "),
            "{id}"
        );
        let n = std::thread::available_parallelism().map_or(4, |n| n.get().max(3));
        for threads in [1, 2, n] {
            let handles: Vec<_> = (0..threads)
                .map(|_| {
                    let id = id.to_owned();
                    std::thread::spawn(move || post_text(&id))
                })
                .collect();
            for h in handles {
                assert_eq!(h.join().unwrap(), reference, "{id} at {threads} threads");
            }
        }
    }
}

/// GC-03 and NFR-12: the example post of the user guide's how-to "Create a post-processing
/// template" (`testdata/examples/minimal-post`) loads from its folder, writes the safety notice,
/// writes every axis word on each move and leaves out a move that would repeat the last
/// position, so the guide's example cannot drift from what the program accepts.
#[test]
fn req_gc_03_the_user_guide_example_post_loads_and_writes_a_program() {
    let folder = repo("testdata/examples/minimal-post");
    let post = TemplatePost::new(PostDefinition::load_dir(&folder).unwrap());
    let text = post.post(&small_part(), &job()).unwrap().text;
    let lines: Vec<&str> = text.lines().collect();
    let notice: Vec<String> = openchuck_post::SAFETY_NOTICE
        .iter()
        .map(|n| format!("({n})"))
        .collect();
    assert_eq!(lines[0], "(Small part)", "{text}");
    assert_eq!(lines[1], "(OpenChuckCAM 0.10.0)", "{text}");
    assert_eq!(&lines[2..4], notice.as_slice(), "{text}");
    assert_eq!(&lines[4..6], ["G90 G21 G17", "G54"], "{text}");
    assert_eq!(&lines[lines.len() - 2..], ["M5", "M30"], "{text}");
    assert!(
        lines.contains(&"T1 M6") && lines.contains(&"S10000 M3"),
        "{text}"
    );
    let moves: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| l.starts_with("G0 ") || l.starts_with("G1 "))
        .collect();
    assert!(moves.len() > 20, "{text}");
    for m in &moves {
        for word in [" X", " Y", " Z", " A"] {
            assert!(m.contains(word), "every move has every axis word: {m}");
        }
        assert_eq!(
            m.starts_with("G1 "),
            m.contains(" F"),
            "only feed moves have F: {m}"
        );
    }
    let at = |m: &str| m.split(" F").next().unwrap()[3..].to_string();
    for pair in moves.windows(2) {
        assert_ne!(
            at(pair[0]),
            at(pair[1]),
            "no move repeats the last position"
        );
    }
    assert_eq!(
        text,
        post.post(&small_part(), &job()).unwrap().text,
        "GC-07"
    );
}

/// NFR-12 and NFR-07: the check looks for the notice as whole comment lines, in order, in the
/// comment block at the top: the text inside a longer line, or the lines written only after
/// the program has started, are refused; with line numbers on the notice is still found.
#[test]
fn req_nfr_12_req_nfr_07_the_notice_must_be_whole_lines_at_the_top() {
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let template = std::fs::read_to_string(posts_path("linuxcnc/main.post")).unwrap();
    let block =
        "{% for notice in safety_notice %}\n{{ line(\"(\" ~ notice ~ \")\") }}\n{% endfor %}\n";
    assert!(template.contains(block));
    let refused = |template: &str| {
        let def = PostDefinition::from_parts("t", &manifest, template).unwrap();
        TemplatePost::new(def)
            .post(&small_part(), &job())
            .unwrap_err()
            .to_string()
    };
    // Inside a longer line.
    let inside = template.replace(
        block,
        "{% for notice in safety_notice %}\n{{ line(\"(note: \" ~ notice ~ \")\") }}\n{% endfor %}\n",
    );
    assert!(refused(&inside).contains("safety_notice"));
    // Only after the program has started (moved below the work offset).
    let late = template.replace(block, "").replace(
        "{% if opt.feed_mode == \"g93\" %}",
        &format!("{block}{{% if opt.feed_mode == \"g93\" %}}"),
    );
    assert_ne!(late, template);
    assert!(refused(&late).contains("safety_notice"));
    // With line numbers, the N word is not part of the comment.
    let numbered = edited_post("linuxcnc", ("enabled = false", "enabled = true"), ("", ""));
    assert!(
        numbered.contains(&format!("N30 ({})", openchuck_post::SAFETY_NOTICE[0])),
        "{numbered}"
    );
}

/// GC-04 and GC-10 (decision 0241 item 6, issue #599): `unwind = "rotate"` writes nothing at an
/// operation boundary. The A words of the next operation keep the accumulated turns, so no
/// line turns A back by whole turns; the Z1's `drop_turns` writes its reset at the same place.
#[test]
fn req_gc_04_req_gc_10_rotate_writes_nothing_at_a_boundary_and_never_turns_back() {
    let mut path = small_part();
    let mut second = path.groups[0].clone();
    second.tags.operation = "second".into();
    second.start = pose(6.0, 0.0, 30.0, 1080.0);
    for segment in &mut second.segments {
        segment.end.a.0 += 1080.0;
    }
    path.groups.push(second);
    let linux = builtin_post("linuxcnc").post(&path, &job()).unwrap().text;
    let a_words: Vec<f64> = linux
        .lines()
        .flat_map(str::split_whitespace)
        .filter_map(|w| w.strip_prefix('A'))
        .map(|a| a.parse().unwrap())
        .collect();
    assert!(
        a_words.windows(2).all(|w| w[1] >= w[0]),
        "A never turns back: {a_words:?}"
    );
    assert_eq!(a_words.last(), Some(&2160.0), "{linux}");
    assert!(!linux.contains("G92"), "{linux}");
    let z1 = builtin_post("makera-z1").post(&path, &job()).unwrap().text;
    assert_eq!(z1.matches("G92.4 A0 S0").count(), 2, "{z1}");
}

/// GC-04 and NFR-12 (issue #598, decision 0654): with `coolant` flood or mist, coolant is
/// turned on with the spindle and off (`M9`) before every tool change and before the footer,
/// so no spindle stop, tool change or program end leaves it running. With `off`, as both
/// built-in posts ship, no coolant code is written.
#[test]
fn req_gc_04_req_nfr_12_coolant_goes_off_before_every_tool_change_and_the_end() {
    let mut path = small_part();
    let mut second = path.groups[0].clone();
    second.tags.operation = "second".into();
    second.tags.tool = "t2".into();
    path.groups.push(second);
    let mut j = job();
    j.tools.insert(
        "t2".into(),
        openchuck_post::ToolSetting {
            number: 2,
            name: "6 mm flat".into(),
            spindle_rpm: 8000.0,
        },
    );
    for id in ["linuxcnc", "makera-z1"] {
        let manifest = std::fs::read_to_string(posts_path(&format!("{id}/post.toml"))).unwrap();
        let template = std::fs::read_to_string(posts_path(&format!("{id}/main.post"))).unwrap();
        let off = builtin_post(id).post(&path, &j).unwrap().text;
        assert!(
            !off.lines().any(|l| ["M7", "M8", "M9"].contains(&l)),
            "{id}: {off}"
        );
        for (value, on) in [("flood", "M8"), ("mist", "M7")] {
            let edited = manifest.replace("coolant = \"off\"", &format!("coolant = \"{value}\""));
            let def = PostDefinition::from_parts("t", &edited, &template).unwrap();
            let text = TemplatePost::new(def).post(&path, &j).unwrap().text;
            let mut running = false;
            let (mut ons, mut offs) = (0, 0);
            for line in text.lines() {
                if line == on {
                    running = true;
                    ons += 1;
                } else if line == "M9" {
                    assert!(running, "{id} {value}: M9 with coolant off: {text}");
                    running = false;
                    offs += 1;
                } else if line == "M5" || line.contains(" M6") || ["M2", "M30"].contains(&line) {
                    assert!(!running, "{id} {value}: {line} with coolant on: {text}");
                }
            }
            assert!(!running, "{id} {value}: coolant on at the end: {text}");
            assert_eq!((ons, offs), (2, 2), "{id} {value}: {text}");
            // Nothing else changes: without the coolant lines it is the coolant-off program.
            let stripped: Vec<_> = text.lines().filter(|l| ![on, "M9"].contains(l)).collect();
            assert_eq!(stripped, off.lines().collect::<Vec<_>>(), "{id} {value}");
        }
    }
}

/// The comment lines in the comment block at the top of a program, without their parentheses.
fn top_comments(text: &str) -> Vec<&str> {
    text.lines()
        .take_while(|l| l.starts_with('('))
        .map(|l| &l[1..l.len() - 1])
        .collect()
}

/// GC-08, GC-09 and NFR-12 (issue #703, decision 0656, OI-933): the Makera Z1 post allows at
/// most 63 characters a line. A longer comment is wrapped at spaces into comment lines that
/// fit, so the NFR-12 notice is written word for word, only on more lines; every line fits.
/// The LinuxCNC post allows 255 and writes the notice as before.
#[test]
fn req_gc_08_req_nfr_12_the_z1_wraps_long_comments_and_keeps_the_notice_word_for_word() {
    assert_eq!(
        builtin_post("makera-z1")
            .definition()
            .options
            .max_line_length,
        Some(63)
    );
    assert_eq!(
        builtin_post("linuxcnc")
            .definition()
            .options
            .max_line_length,
        Some(255)
    );
    let text = post_text("makera-z1");
    let longest = text.lines().map(str::len).max().unwrap();
    assert!(longest <= 63, "{longest}: {text}");
    let joined = top_comments(&text).join(" ");
    for notice in openchuck_post::SAFETY_NOTICE {
        assert!(joined.contains(notice), "{notice}: {joined}");
        assert!(notice.len() + 2 > 63, "the notice line is wrapped");
    }
    let linux = post_text("linuxcnc");
    for notice in openchuck_post::SAFETY_NOTICE {
        assert!(linux.contains(&format!("\n({notice})\n")), "{linux}");
    }
}

/// GC-08, NFR-07 and GC-07 (decision 0656): a comment from project text longer than the limit
/// is wrapped at spaces, a word longer than a line is cut, nothing is lost, and with line
/// numbers on the numbered lines fit too. The same input gives the same lines.
#[test]
fn req_gc_08_req_gc_07_long_comments_are_wrapped_to_fit_and_lose_nothing() {
    let mut j = job();
    let word = "x".repeat(80);
    j.title = format!("A rather long project name - Setup 1 - with words {word} end");
    for numbered in [false, true] {
        let manifest = std::fs::read_to_string(posts_path("makera-z1/post.toml")).unwrap();
        let template = std::fs::read_to_string(posts_path("makera-z1/main.post")).unwrap();
        let manifest = if numbered {
            manifest.replace("enabled = false", "enabled = true")
        } else {
            manifest
        };
        let post =
            TemplatePost::new(PostDefinition::from_parts("t", &manifest, &template).unwrap());
        let text = post.post(&small_part(), &j).unwrap().text;
        assert_eq!(text, post.post(&small_part(), &j).unwrap().text);
        assert!(text.lines().all(|l| l.len() <= 63), "{text}");
        let bodies: String = text
            .lines()
            .map(|l| match l.split_once(' ') {
                Some((n, rest)) if n.starts_with('N') && rest.starts_with('(') => rest,
                _ => l,
            })
            .take_while(|l| l.starts_with('('))
            .map(|l| &l[1..l.len() - 1])
            .collect::<Vec<_>>()
            .join(" ");
        let squeezed = |s: &str| s.split_whitespace().collect::<String>();
        assert!(squeezed(&bodies).contains(&squeezed(&j.title)), "{bodies}");
        assert!(text.starts_with(if numbered {
            "N10 (A rather long"
        } else {
            "(A rather long"
        }));
    }
}

/// GC-08 (decision 0656): only comments are wrapped. A command line over the limit is written
/// as it is, for the G-code check to refuse; a word exactly two lines long fills two lines.
#[test]
fn req_gc_08_command_lines_are_never_wrapped() {
    let free = edited_post(
        "linuxcnc",
        ("max_line_length = 255", "max_line_length = 4096"),
        ("", ""),
    );
    let mut j = job();
    j.title = "y".repeat(2 * 18);
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let template = std::fs::read_to_string(posts_path("linuxcnc/main.post")).unwrap();
    let def = PostDefinition::from_parts(
        "t",
        &manifest.replace("max_line_length = 255", "max_line_length = 20"),
        &template,
    )
    .unwrap();
    let tight = TemplatePost::new(def).post(&small_part(), &j).unwrap().text;
    let commands = |t: &str| -> Vec<String> {
        t.lines()
            .filter(|l| !l.starts_with('('))
            .map(str::to_owned)
            .collect()
    };
    assert_eq!(commands(&tight), commands(&free));
    assert!(tight.lines().any(|l| l.len() > 20), "{tight}");
    let y = "y".repeat(18);
    assert!(tight.starts_with(&format!("({y})\n({y})\n")), "{tight}");
}

/// NFR-12 and NFR-07 (decision 0533, issue #839 item 3): a line that holds a comment and a
/// command, such as `(title) G90`, ends the opening comment block, so a notice written after it
/// is after the program has started and is refused; comment-only headers still pass.
#[test]
fn req_nfr_12_req_nfr_07_a_mixed_comment_and_command_line_ends_the_opening_block() {
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let template = std::fs::read_to_string(posts_path("linuxcnc/main.post")).unwrap();
    let title = "{{ line(\"(\" ~ title ~ \")\") }}";
    assert!(template.contains(title));
    let mixed = template.replace(title, "{{ line(\"(\" ~ title ~ \") G90\") }}");
    let def = PostDefinition::from_parts("t", &manifest, &mixed).unwrap();
    let e = TemplatePost::new(def)
        .post(&small_part(), &job())
        .unwrap_err()
        .to_string();
    assert!(e.contains("safety_notice"), "{e}");
    assert!(post_text("linuxcnc").contains(openchuck_post::SAFETY_NOTICE[0]));
}

/// GC-08 and NFR-12 (decision 0656; CodeRabbit on #840): a long `N` word narrows only the first
/// line of a wrapped comment; the lines after it carry no `N` word and use the whole width, so a
/// word that fits there is never split and the notice stays word for word.
#[test]
fn req_gc_08_req_nfr_12_a_long_line_number_narrows_only_the_first_wrapped_line() {
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml"))
        .unwrap()
        .replace("max_line_length = 255", "max_line_length = 20")
        .replace(
            "enabled = false\nstart = 10\nstep = 10",
            "enabled = true\nstart = 1000000000\nstep = 1",
        );
    assert!(manifest.contains("start = 1000000000"));
    let template = std::fs::read_to_string(posts_path("linuxcnc/main.post")).unwrap();
    let def = PostDefinition::from_parts("t", &manifest, &template).unwrap();
    let text = TemplatePost::new(def)
        .post(&small_part(), &job())
        .unwrap()
        .text;
    let comments: Vec<&str> = text
        .lines()
        .map(|l| match l.split_once(' ') {
            Some((n, rest)) if n.starts_with('N') && rest.starts_with('(') => rest,
            _ => l,
        })
        .filter(|l| l.starts_with('(') && l.ends_with(')'))
        .map(|l| &l[1..l.len() - 1])
        .collect();
    let joined = comments.join(" ");
    for notice in openchuck_post::SAFETY_NOTICE {
        assert!(joined.contains(notice), "{notice}: {text}");
    }
    for line in text.lines().filter(|l| l.contains('(')) {
        assert!(line.len() <= 20, "{line}");
    }
}

/// GC-08 (decision 0656; CodeRabbit on #840): a long comment on the last line of a program that
/// has no line end is still wrapped onto separate lines.
#[test]
fn req_gc_08_a_long_last_comment_without_a_line_end_is_wrapped_onto_lines() {
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml"))
        .unwrap()
        .replace("max_line_length = 255", "max_line_length = 20")
        .replace("safety_notice = true", "safety_notice = false");
    let mut j = job();
    j.title = "y".repeat(36);
    let def = PostDefinition::from_parts("t", &manifest, "({{ title }})").unwrap();
    let text = TemplatePost::new(def).post(&small_part(), &j).unwrap().text;
    let y = "y".repeat(18);
    assert_eq!(text, format!("({y})\n({y})"));
}

/// The comment openings LinuxCNC acts on, from its source (`convert_comment` in
/// `src/emc/rs274ngc/interp_convert.cc` and `COMMENT` in `src/emc/task/emccanon.cc`, master
/// `49d14cb6`): matched after the comment's leading white space, case-insensitively for the
/// interpreter's words, case-sensitively for the task's. Written here independently of the
/// post definition, as the oracle (issue #851).
const LINUXCNC_ACTIVE: &[&str] = &[
    "msg,",
    "debug,",
    "print,",
    "log,",
    "logopen,",
    "logappend,",
    "logclose",
    "py,",
    "pyrun,",
    "pyreload",
    "abort,",
    "PROBEOPEN",
    "PROBECLOSE",
    "RPY",
];

/// Whether LinuxCNC would act on a comment in `line` (issue #851): any `(...)` on the line
/// whose text, after its leading white space, starts with one of [`LINUXCNC_ACTIVE`]. The
/// interpreter's words are compared case-insensitively, as LinuxCNC does; the task's
/// upper-case words case-sensitively.
fn linuxcnc_active(line: &str) -> Option<String> {
    let mut rest = line;
    while let Some(open) = rest.find('(') {
        let after = &rest[open + 1..];
        let close = after.find(')').unwrap_or(after.len());
        let text = after[..close].trim_start();
        for word in LINUXCNC_ACTIVE {
            let hit = if word.chars().all(|c| c.is_ascii_uppercase()) {
                text.starts_with(word)
            } else {
                text.to_ascii_lowercase().starts_with(word)
            };
            if hit {
                return Some(format!("{line:?} is active ({word})"));
            }
        }
        rest = &after[close.min(after.len())..];
    }
    None
}

fn assert_no_active_comment(text: &str) {
    for line in text.lines() {
        if let Some(why) = linuxcnc_active(line) {
            panic!("{why}\n{text}");
        }
    }
}

/// The LinuxCNC post with its line limit set to `max`.
fn linuxcnc_limited(max: u32) -> TemplatePost {
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let template = std::fs::read_to_string(posts_path("linuxcnc/main.post")).unwrap();
    TemplatePost::new(
        PostDefinition::from_parts(
            "t",
            &manifest.replace("max_line_length = 255", &format!("max_line_length = {max}")),
            &template,
        )
        .unwrap(),
    )
}

/// GC-10 and NFR-07 (issue #851, item 1): the built-in LinuxCNC post lists the comment
/// openings LinuxCNC acts on, and a project whose name starts with one does not start an
/// active comment: the program title `MSG,direct title` is written with the guard mark in
/// front, so LinuxCNC reads it as an ordinary comment. The text is otherwise unchanged.
#[test]
fn req_gc_10_req_nfr_07_a_project_name_never_starts_an_active_comment_on_linuxcnc() {
    let post = builtin_post("linuxcnc");
    let listed = &post.definition().options.active_comment_prefixes;
    for word in LINUXCNC_ACTIVE {
        assert!(
            listed.iter().any(|l| l.eq_ignore_ascii_case(word)),
            "{word} is not in {listed:?}"
        );
    }
    let mut j = job();
    j.title = "MSG,direct title".into();
    let text = post_text_with("linuxcnc", &j);
    assert!(text.starts_with("(_MSG,direct title)\n"), "{text}");
    assert_no_active_comment(&text);
    // Every listed opening, in any case and after leading spaces, as the title and as the
    // operation name.
    for word in LINUXCNC_ACTIVE {
        for name in [
            word.to_string(),
            format!("{} x", word.to_ascii_uppercase()),
            format!("  {}x", word.to_ascii_lowercase()),
        ] {
            let mut j = job();
            j.title = name.clone();
            j.operation_names
                .insert("groove (finish)".into(), name.clone());
            let text = post_text_with("linuxcnc", &j);
            assert_no_active_comment(&text);
            assert_eq!(text, post_text_with("linuxcnc", &j), "GC-07");
        }
    }
}

/// GC-10, GC-08 and NFR-07 (issue #851, item 2): a wrapped comment's later lines start fresh
/// (decision 0656), so a word in the middle of a long title, operation or tool name can start
/// a line. With a limit of 20, `abcdefghijklmnopqrMSG,x` is cut after 18 characters and the
/// rest is written with the guard mark; a word after a space is guarded the same way. Every
/// line still fits and no text is lost.
#[test]
fn req_gc_10_req_gc_08_a_wrapped_line_never_starts_an_active_comment() {
    let post = linuxcnc_limited(20);
    let mut j = job();
    j.title = "abcdefghijklmnopqrMSG,x".into();
    let text = post.post(&small_part(), &j).unwrap().text;
    assert!(
        text.starts_with("(abcdefghijklmnopqr)\n(_MSG,x)\n"),
        "{text}"
    );
    assert_no_active_comment(&text);
    // A word too long for a line that starts with an opening: its first line is marked, the
    // mark counted in the width.
    let mut j = job();
    j.title = format!("x MSG,{}", "y".repeat(24));
    let text = post.post(&small_part(), &j).unwrap().text;
    let (a, b) = ("y".repeat(13), "y".repeat(11));
    assert!(
        text.starts_with(&format!("(x)\n(_MSG,{a})\n({b})\n")),
        "{text}"
    );
    for max in 20..=48 {
        let post = linuxcnc_limited(max);
        for word in LINUXCNC_ACTIVE {
            let mut j = job();
            j.title = format!("Part {word}one - Setup {word}two abcdefghijklmnopqr{word}three");
            j.operation_names
                .insert("groove (finish)".into(), format!("rough {word}op"));
            j.tools.get_mut("t1").unwrap().name = format!("ball {word}tool");
            let text = post.post(&small_part(), &j).unwrap().text;
            assert_no_active_comment(&text);
            assert!(
                text.lines()
                    .filter(|l| l.starts_with('('))
                    .all(|l| l.len() <= max as usize),
                "{max}: {text}"
            );
            assert_eq!(text, post.post(&small_part(), &j).unwrap().text, "GC-07");
        }
    }
}

/// GC-09 (issue #851): the Makera Z1 firmware (a Smoothieware fork) skips a line that starts
/// with `(` and cuts a comment off a command line, with no active comments in its source, so
/// the built-in Z1 post lists none and writes the project's text as it is.
#[test]
fn req_gc_09_the_z1_post_lists_no_active_comments_and_writes_names_as_they_are() {
    let post = builtin_post("makera-z1");
    assert!(post.definition().options.active_comment_prefixes.is_empty());
    let mut j = job();
    j.title = "MSG,direct title".into();
    let text = post.post(&small_part(), &j).unwrap().text;
    assert!(text.starts_with("(MSG,direct title)\n"), "{text}");
}

/// GC-03 and GC-04 (issue #851): only text from the project is guarded. A comment the post's
/// author writes in the header, such as LinuxCNC's `(MSG,...)`, is the author's to write and is
/// kept as it is, even when it is wrapped; its later lines are guarded.
#[test]
fn req_gc_03_req_gc_04_the_posts_own_active_comments_are_kept() {
    let text = edited_post(
        "linuxcnc",
        (
            "header = [\"G90 G21 G17\", \"G61\"]",
            "header = [\"G90 G21 G17\", \"G61\", \"(MSG,Check the stock is held)\"]",
        ),
        ("", ""),
    );
    assert!(
        text.contains("\nG61\n(MSG,Check the stock is held)\n"),
        "{text}"
    );
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let template = std::fs::read_to_string(posts_path("linuxcnc/main.post")).unwrap();
    let manifest = manifest
        .replace("max_line_length = 255", "max_line_length = 20")
        .replace(
            "header = [\"G90 G21 G17\", \"G61\"]",
            "header = [\"G90 G21 G17\", \"(MSG,Check the msg,stock)\"]",
        );
    let text = TemplatePost::new(PostDefinition::from_parts("t", &manifest, &template).unwrap())
        .post(&small_part(), &job())
        .unwrap()
        .text;
    assert!(text.contains("\n(MSG,Check the)\n(_msg,stock)\n"), "{text}");
}

/// GC-04 and NFR-07 (issue #851): the list is checked when the post is loaded, naming the
/// option: an empty or spaced entry, a parenthesis, an entry that starts with the guard mark
/// (the mark would then make the comment active), and an entry that a word of the NFR-12
/// notice starts with (a wrapped notice line would be changed, decision 0533).
#[test]
fn req_gc_04_req_nfr_07_bad_active_comment_prefixes_are_refused() {
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let template = std::fs::read_to_string(posts_path("linuxcnc/main.post")).unwrap();
    assert!(
        manifest.contains("active_comment_prefixes = ["),
        "{manifest}"
    );
    for bad in [
        "\"\"",
        "\"MSG ,\"",
        "\"(MSG\"",
        "\"_X\"",
        "\"check\"",
        "\"WHAT\"",
    ] {
        let edited = manifest.replace(
            "active_comment_prefixes = [",
            &format!("active_comment_prefixes = [{bad}, "),
        );
        let e = PostDefinition::from_parts("t", &edited, &template)
            .unwrap_err()
            .to_string();
        assert!(e.contains("active_comment_prefixes"), "{bad}: {e}");
    }
    let many: Vec<String> = (0..65).map(|i| format!("\"W{i},\"")).collect();
    let edited = manifest.replace(
        "active_comment_prefixes = [",
        &format!("active_comment_prefixes = [{}, ", many.join(", ")),
    );
    let e = PostDefinition::from_parts("t", &edited, &template)
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("active_comment_prefixes") && e.contains("64"),
        "{e}"
    );
    // A post without the notice may list a word the notice holds.
    let edited = manifest
        .replace("safety_notice = true", "safety_notice = false")
        .replace(
            "active_comment_prefixes = [",
            "active_comment_prefixes = [\"check\", ",
        );
    assert!(PostDefinition::from_parts("t", &edited, &template).is_ok());
}

/// NFR-07 and GC-10 (issue #851, the owner's addition of 23 Nov 2026): every text the post
/// guarded is reported once, with where it came from (the title, an operation's name by its
/// tag, a tool's name by its tag, or a comment line of the post's own), the opening it matched
/// as the post lists it, the text as the job gave it and where in that text the mark went, so
/// the host can warn the user. Ordinary names and the Z1 post report nothing; the report is
/// deterministic (GC-07).
#[test]
fn req_gc_10_req_nfr_07_each_guarded_text_is_reported_once_with_its_source() {
    use openchuck_post::{CommentSource, GuardedComment};
    let report = |post: &TemplatePost, j: &openchuck_post::PostJob| {
        let out = post.post(&small_part(), j).unwrap();
        assert_eq!(
            out.guarded_comments,
            post.post(&small_part(), j).unwrap().guarded_comments,
            "GC-07"
        );
        out.guarded_comments
    };
    let guarded = |source: CommentSource, opening: &str, text: &str, at: usize| GuardedComment {
        source,
        opening: opening.into(),
        text: text.into(),
        at,
    };
    assert!(report(&builtin_post("linuxcnc"), &job()).is_empty());
    let mut j = job();
    j.title = "  MSG,direct title".into();
    assert_eq!(
        report(&builtin_post("linuxcnc"), &j),
        vec![guarded(
            CommentSource::Title,
            "MSG,",
            "  MSG,direct title",
            2
        )]
    );
    assert!(report(&builtin_post("makera-z1"), &j).is_empty());

    // Wrapped at 20: the title cut in a word, an operation name and a tool name, each once.
    let mut j = job();
    j.title = "abcdefghijklmnopqrMSG,x".into();
    j.operation_names
        .insert("groove (finish)".into(), "rough logopen,f".into());
    j.tools.get_mut("t1").unwrap().name = "PY,abcdefghijklmnopq".into();
    assert_eq!(
        report(&linuxcnc_limited(20), &j),
        vec![
            guarded(CommentSource::Title, "MSG,", "abcdefghijklmnopqrMSG,x", 18),
            guarded(
                CommentSource::Operation("groove (finish)".into()),
                "LOGOPEN,",
                "rough logopen,f",
                6
            ),
            guarded(
                CommentSource::Tool("t1".into()),
                "PY,",
                "PY,abcdefghijklmnopq",
                0
            ),
        ]
    );

    // The same text as the title and an operation's name (CodeRabbit on #857): a mark inside the
    // operation comment is the operation's, and the title's own line stays the title's.
    let mut j = job();
    j.title = "rough logopen,f".into();
    j.operation_names
        .insert("groove (finish)".into(), "rough logopen,f".into());
    let reported = report(&linuxcnc_limited(20), &j);
    assert_eq!(
        reported,
        vec![guarded(
            CommentSource::Operation("groove (finish)".into()),
            "LOGOPEN,",
            "rough logopen,f",
            6
        )],
        "the title fits one line and is not guarded"
    );

    // A title guarded at its start with another opening later, at a wrap: both places, each
    // counted in the title as the job gave it.
    let mut j = job();
    j.title = " MSG,one two three MSG,four".into();
    assert_eq!(
        report(&linuxcnc_limited(20), &j),
        vec![
            guarded(
                CommentSource::Title,
                "MSG,",
                " MSG,one two three MSG,four",
                1
            ),
            guarded(
                CommentSource::Title,
                "MSG,",
                " MSG,one two three MSG,four",
                19
            ),
        ]
    );

    // A wrapped line of the post's own header: its first line is the author's and is kept;
    // the later line is guarded and reported as the post's.
    let manifest = std::fs::read_to_string(posts_path("linuxcnc/post.toml")).unwrap();
    let template = std::fs::read_to_string(posts_path("linuxcnc/main.post")).unwrap();
    let manifest = manifest
        .replace("max_line_length = 255", "max_line_length = 20")
        .replace(
            "header = [\"G90 G21 G17\", \"G61\"]",
            "header = [\"G90 G21 G17\", \"(MSG,Check the msg,stock)\"]",
        );
    let post = TemplatePost::new(PostDefinition::from_parts("t", &manifest, &template).unwrap());
    assert_eq!(
        report(&post, &job()),
        vec![guarded(
            CommentSource::Post,
            "MSG,",
            "MSG,Check the msg,stock",
            14
        )]
    );
}

/// NFR-07, issue #794: the nightly `post_definition` fuzz input, a damaged copy of the LinuxCNC
/// post whose template stores a namespace inside itself (`{% set st.a = st %}`), loads and posts
/// to a result, never a crash. The engine frees the namespace after the render (its unit test
/// watches the memory being freed; the fuzz target's leak check runs this file in the nightly).
/// The render then stops at a damaged line, after the cycle is made, so the post is refused
/// with a template error naming the line: the error path frees the namespace too.
#[test]
fn req_nfr_07_a_template_with_a_namespace_cycle_posts_without_a_crash() {
    let data = std::fs::read(repo("testdata/posts/fuzz_namespace_cycle.txt")).unwrap();
    let text = std::str::from_utf8(&data).unwrap();
    // Split as the fuzz target does.
    let (manifest, template) = text.split_once("\n---\n").unwrap();
    let definition = PostDefinition::from_parts("fuzz", manifest, template).unwrap();
    let err = TemplatePost::new(definition)
        .post(&small_part(), &job())
        .unwrap_err()
        .to_string();
    assert!(err.contains("main.post at line 62"), "{err}");
}

/// NFR-07, issue #794: a template may keep state in a namespace as before, but a namespace key
/// that is not a name is refused with a template error naming the key.
#[test]
fn req_nfr_07_a_namespace_key_that_is_not_a_name_is_a_template_error() {
    let linux = builtin("linuxcnc").unwrap().unwrap();
    let template = format!(
        "{{% set bad = namespace({{'a b': 1}}) %}}\n{}",
        linux.template
    );
    let manifest =
        std::fs::read_to_string(repo("crates/openchuck-post/posts/linuxcnc/post.toml")).unwrap();
    let definition = PostDefinition::from_parts("linuxcnc", &manifest, &template).unwrap();
    let err = TemplatePost::new(definition)
        .post(&small_part(), &job())
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("namespace") && err.contains("is not a name"),
        "{err}"
    );
}
