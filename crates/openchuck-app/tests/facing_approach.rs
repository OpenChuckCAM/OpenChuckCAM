// SPDX-License-Identifier: AGPL-3.0-or-later
//! End facing on the Baluster example, as the owner set it up (issue #499):
//! box stock 72.4 x 35 x 35 mm, the model scaled by 1.15, the start end held
//! for 8 mm, and End facing with a 3.175 mm flat tool and 1 mm stock to
//! leave. The tests run the real application and both built-in posts.
//!
//! The G-code these tests produce is not checked on a machine: generated
//! G-code is the user's to verify before it is run (NFR-12).
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::float_cmp,
    clippy::panic
)]

use std::path::{Path, PathBuf};

use openchuck_app::{Application, OutputGrouping, PostSelection, Regeneration, Threads};
use openchuck_types::ir::{MoveKind, ToolpathGroup};

const SETUP: &str = "57ad6798-2544-4dfb-90a0-caf67ec69484";
const FACING: &str = "99e9173f-5bc8-4b43-99a9-9030e097e6b5";
/// The approach distance the operation uses (the default).
const APPROACH: f64 = 1.;
/// The tool's radius (Z1 O-flute 3.175).
const RADIUS: f64 = 3.175 / 2.;
/// The far end of the stock and the top of the box, model frame. The
/// example is laid out from the Z1's headstock face, 17.990234375 mm behind
/// the owner's model X 0 (decision record 0649): the stock runs from X -1 to
/// 71.4 in the owner's layout, 16.990234375 to 89.390234375 here.
const STOCK_END: f64 = 89.390_234_375;
const STOCK_TOP: f64 = 17.5;

/// A copy of the Baluster example with the owner's End facing set-up, in a
/// folder of its own. `extra` adds lines to the operation's parameters.
fn project(name: &str, extra: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "openchuck-face-approach-{name}-{unique}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let example = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/examples/baluster/baluster-project");
    copy(&example, &dir);
    let project = dir.join("project.toml");
    let text = std::fs::read_to_string(&project).unwrap();
    std::fs::write(
        &project,
        format!("{text}\n[[models.transform]]\nkind = \"scale\"\nfactor = 1.15\n"),
    )
    .unwrap();
    let setup = dir.join(format!("setups/{SETUP}.toml"));
    let text = std::fs::read_to_string(&setup).unwrap();
    let stock = "[stock]\ntype = \"cylinder\"\ndiameter = 24.000000238343276\nlength = 63.0\nx_start = 16.990234375\n";
    assert!(text.contains(stock), "the example's stock has changed");
    std::fs::write(
        &setup,
        text.replace(
            stock,
            "[stock]\ntype = \"box\"\n\n[stock.bounds.min]\nx = 16.990234375\ny = -17.5\nz = -17.5\n\n\
             [stock.bounds.max]\nx = 89.390234375\ny = 17.5\nz = 17.5\n",
        ),
    )
    .unwrap();
    // Only the facing operation runs: the example's own operations are off.
    for entry in std::fs::read_dir(dir.join("operations")).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(
            &path,
            text.replacen("\n[parameters]", "enabled = false\n\n[parameters]", 1),
        )
        .unwrap();
    }
    std::fs::write(
        dir.join(format!("operations/{FACING}.toml")),
        format!(
            "schema = 25\nid = \"{FACING}\"\norder = \"U\"\nname = \"End facing\"\n\
             setup = \"{SETUP}\"\nstrategy = \"facing\"\n\
             tool = \"c0de0001-0000-4000-8000-000000000104\"\nselections = []\n\
             spindle_speed = 10000.0\n\n[parameters]\nextend = \"no\"\nfeed = 1000.0\n\
             plunge_feed = 300.0\nshape = \"end\"\nstepdown = 2.0\nstepover = 2.2225\n\
             stock_to_leave = 1.0\n{extra}"
        ),
    )
    .unwrap();
    dir
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn post(dir: &Path, post: &str) -> Regeneration {
    let mut app = Application::open(dir, Threads::Exactly(2)).unwrap();
    app.start_regenerate_with_post(PostSelection::Builtin(post.into()), OutputGrouping::Setup)
        .wait()
        .unwrap_or_else(|e| panic!("{post}: {e}"))
}

/// The facing group of a run.
fn facing(run: &Regeneration) -> &ToolpathGroup {
    assert_eq!(run.toolpath.groups.len(), 1);
    &run.toolpath.groups[0]
}

/// How many times the tool goes back up to the retract height between the
/// first and the last cut.
fn retracts(group: &ToolpathGroup) -> usize {
    let safe = group.start.tip.z;
    let first = group
        .segments
        .iter()
        .position(|s| s.kind == MoveKind::Cutting)
        .unwrap();
    let last = group
        .segments
        .iter()
        .rposition(|s| s.kind == MoveKind::Cutting)
        .unwrap();
    group.segments[first..last]
        .iter()
        .filter(|s| s.kind == MoveKind::Rapid && s.end.tip.z >= safe - 1e-9)
        .count()
}

/// Issue #499: the owner's Baluster End facing posts through both posts.
/// It was refused with "the actual end sweeps leave an unfinished required
/// region in an axial layer": the written motion read back put the lowest
/// band 7e-15 mm above the stock's bottom edge.
#[test]
fn req_op_24_req_gc_08_baluster_end_facing_posts_through_both_posts() {
    let dir = project("posts", "");
    for name in ["makera-z1", "linuxcnc"] {
        let run = post(&dir, name);
        assert_eq!(run.gcode.len(), 1, "{name}");
        assert!(run.gcode[0].1.contains("G1"), "{name}");
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// Issue #499: a rapid that goes down beside the end face keeps the
/// approach distance from it, as a rapid above the stock keeps it from the
/// top (OP-14). It was 2.5 um, so stock slightly longer than modelled would
/// have been hit at rapid. Every fed move into the end starts from there.
#[test]
fn req_op_14_req_nfr_12_baluster_end_rapids_keep_the_approach_distance_from_the_end_face() {
    let dir = project("air", "");
    let run = post(&dir, "makera-z1");
    let group = facing(&run);
    let mut previous = group.start;
    for segment in &group.segments {
        if segment.kind == MoveKind::Rapid
            && previous.tip.z.min(segment.end.tip.z) < STOCK_TOP + APPROACH - 1e-9
        {
            let near = previous.tip.x.min(segment.end.tip.x) - RADIUS;
            assert!(
                near >= STOCK_END + APPROACH - 1e-9,
                "a rapid below the stock top + approach passes {:.6} mm from the end face",
                near - STOCK_END
            );
        }
        previous = segment.end;
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// Decision D1 of issue #499 (the owner's choice): Facing End offers the
/// "Links between passes" choice of decision 0496, retract by default. A
/// saved operation without it, and one set to retract, make the same path,
/// going up to the retract height between bands as before (GC-07).
#[test]
fn req_op_14_req_gc_07_end_facing_links_retract_by_default() {
    let saved = project("saved", "");
    let retract = project("retract", "link_height = \"retract\"\n");
    let a = post(&saved, "linuxcnc");
    let b = post(&retract, "linuxcnc");
    assert_eq!(a.canonical_toolpath, b.canonical_toolpath);
    assert_eq!(a.gcode, b.gcode);
    assert!(retracts(facing(&a)) >= 15, "{}", retracts(facing(&a)));
    for dir in [saved, retract] {
        std::fs::remove_dir_all(dir).unwrap();
    }
}

/// With "Links between passes" low, End facing links each band to the next
/// beside the end, in the air the approach distance clear of the face, and
/// never goes back up to the retract height between bands (OP-14, decision
/// 0496's rule). The held start end is far away. Both posts accept it.
#[test]
fn req_op_14_req_set_09_end_facing_low_links_stay_beside_the_end() {
    let dir = project("low", "link_height = \"low\"\n");
    for name in ["makera-z1", "linuxcnc"] {
        let run = post(&dir, name);
        let group = facing(&run);
        assert_eq!(retracts(group), 0, "{name}");
        let mut previous = group.start;
        for segment in &group.segments {
            if !segment.kind.is_fed() && previous.tip.z.min(segment.end.tip.z) < STOCK_TOP {
                assert!(
                    previous.tip.x.min(segment.end.tip.x) - RADIUS >= STOCK_END + APPROACH - 1e-9,
                    "{name}: a low link passes over the stock"
                );
            }
            previous = segment.end;
        }
        assert!(
            !run.notes.iter().any(|n| n.contains("could not be lowered")),
            "{name}: {:?}",
            run.notes
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// The owner's decision F1 on issue #499: the Baluster's end is 35 mm deep
/// across the tool and the tool has 25 mm of flute, so End facing faces it
/// from A 0 and A 180, never cutting more than the flute length below the
/// stock's top, and both posts accept the program.
#[test]
fn req_op_24_req_nfr_12_baluster_end_is_faced_from_both_sides_within_the_flutes() {
    let dir = project("flutes", "");
    for name in ["makera-z1", "linuxcnc"] {
        let run = post(&dir, name);
        let group = facing(&run);
        let mut angles = Vec::new();
        for segment in &group.segments {
            if segment.kind == MoveKind::Cutting {
                let a = segment.end.a.0.rem_euclid(360.);
                if !angles.contains(&a) {
                    angles.push(a);
                }
                // The tip's height above the axis along the tool, at this A.
                let (sin, cos) = segment.end.a.0.to_radians().sin_cos();
                let height = segment.end.tip.z * cos + segment.end.tip.y * sin;
                assert!(
                    STOCK_TOP - height <= 25. + 1e-6,
                    "{name}: a cut {:.3} mm below the stock top at A {a}",
                    STOCK_TOP - height
                );
            }
        }
        angles.sort_by(f64::total_cmp);
        assert_eq!(angles.len(), 2, "{name}: {angles:?}");
        assert!(
            (angles[1] - angles[0] - 180.).abs() < 1e-6,
            "{name}: {angles:?}"
        );
        assert_eq!(run.gcode.len(), 1, "{name}");
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// Decision 0547 item 4 (finding (a) of issue #641): on the Baluster's own
/// round stock, lengthened to leave exactly 3 mm past the end at 1 mm of
/// stock to leave, a Stepover of 1 mm (the slice along X, as the tool's
/// Hardwood preset gives it) cuts three full slices. Rounding the written
/// tool centre to three decimals made the read-back slice 1.0005 mm, and the
/// post-time check refused it as deeper than the Stepover. It posts through
/// both posts.
#[test]
fn req_op_24_req_nfr_05_whole_number_of_slices_posts_with_both_posts() {
    let dir = round_bar("whole-slices", 66.0, 2.2225, 1.0);
    for name in ["makera-z1", "linuxcnc"] {
        let run = post(&dir, name);
        assert_eq!(run.gcode.len(), 1, "{name}");
    }
    std::fs::remove_dir_all(dir).unwrap();
}

/// The Baluster at its own size on the example's round bar, `length` long
/// from the example's start, End faced with `stepdown` along X and
/// `stepover` across.
fn round_bar(name: &str, length: f64, stepdown: f64, stepover: f64) -> PathBuf {
    let dir = project(name, "");
    let setup = dir.join(format!("setups/{SETUP}.toml"));
    let text = std::fs::read_to_string(&setup).unwrap();
    let start = text.find("[stock]").unwrap();
    let end = text[start..]
        .find("\n[stock.bounds.max]")
        .map(|i| start + i)
        .unwrap();
    let tail = &text[end..];
    let after = tail
        .find("z = 17.5\n")
        .map(|i| i + "z = 17.5\n".len())
        .unwrap();
    let rest = &tail[after..];
    std::fs::write(
        &setup,
        format!(
            "{}[stock]\ntype = \"cylinder\"\ndiameter = 24.000000238343276\nlength = {length:?}\n\
             x_start = 16.990234375\n{rest}",
            &text[..start]
        ),
    )
    .unwrap();
    let file = dir.join("project.toml");
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(
        &file,
        text.replace(
            "\n[[models.transform]]\nkind = \"scale\"\nfactor = 1.15\n",
            "",
        ),
    )
    .unwrap();
    let operation = dir.join(format!("operations/{FACING}.toml"));
    let text = std::fs::read_to_string(&operation).unwrap();
    std::fs::write(
        &operation,
        text.replace("schema = 25\n", "schema = 28\n").replace(
            "stepdown = 2.0\nstepover = 2.2225\n",
            &format!("stepdown = {stepdown:?}\nstepover = {stepover:?}\n"),
        ),
    )
    .unwrap();
    dir
}

/// Issue #650: on the example's round bar 66.3 mm long, ordinary End
/// settings the generator accepts were refused when posted, with "the
/// actual end sweeps leave an unfinished required region in an axial
/// layer". The issue gives the slice along X as Stepdown; today it is
/// Stepover (slice thickness along X) and Stepdown is the band along the
/// tool, so both readings are run. Each posts through both posts. Split by
/// case to keep each test short (docs/ci-performance.md).
#[test]
fn req_op_24_req_nfr_07_end_facing_on_round_bar_posts_the_issue_settings() {
    posts_on_round_bar(&[(2.2225, 1.0), (1.0, 2.2225)]);
}

/// Issue #650's thicker slices (3.17 and 3.1 mm along X) with long bands.
#[test]
fn req_op_24_req_nfr_07_end_facing_on_round_bar_posts_thick_slices() {
    posts_on_round_bar(&[(25.0, 3.17), (25.0, 3.1), (10.0, 3.1)]);
}

/// Each `(stepdown, stepover)` End facing on the 66.3 mm round bar
/// generates and posts through both posts.
fn posts_on_round_bar(cases: &[(f64, f64)]) {
    for &(stepdown, stepover) in cases {
        let dir = round_bar("round-bar-66-3", 66.3, stepdown, stepover);
        for name in ["makera-z1", "linuxcnc"] {
            let mut app = Application::open(&dir, Threads::Exactly(2)).unwrap();
            let run = app
                .start_regenerate_with_post(
                    PostSelection::Builtin(name.into()),
                    OutputGrouping::Setup,
                )
                .wait();
            assert!(
                run.is_ok(),
                "stepdown {stepdown}, stepover {stepover}, {name}: {}",
                run.err().map(|e| e.to_string()).unwrap_or_default()
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}

/// Where the Baluster's grip ends: the held start runs 8 mm from the stock's
/// start (X 16.990234375 to 24.990234375).
const HELD_END: f64 = 24.990_234_375;
/// How far the whole tool keeps from the grip (decision 0161).
const HELD_CLEAR: f64 = 0.5;

/// A copy of the Baluster set-up with Facing of the given shape and extent,
/// regenerated through both posts, the start held for 8 mm.
fn facing_beside_the_grip(shape: &str, extra: &str, threads: usize) -> Vec<Regeneration> {
    let dir = project(shape, extra);
    let operation = dir.join(format!("operations/{FACING}.toml"));
    let text = std::fs::read_to_string(&operation).unwrap();
    std::fs::write(
        &operation,
        text.replace("shape = \"end\"", &format!("shape = \"{shape}\"")),
    )
    .unwrap();
    let runs = ["makera-z1", "linuxcnc"]
        .into_iter()
        .map(|name| {
            let mut app = Application::open(&dir, Threads::Exactly(threads)).unwrap();
            app.start_regenerate_with_post(
                PostSelection::Builtin(name.into()),
                OutputGrouping::Setup,
            )
            .wait()
            .unwrap_or_else(|e| panic!("{shape} {extra:?} {name}: {e}"))
        })
        .collect();
    std::fs::remove_dir_all(dir).unwrap();
    runs
}

/// No fed move of the run's Facing reaches the grip widened by the tool's
/// radius and the clearance: every tip X is at least X 27.078.
fn assert_clear_of_the_grip(run: &Regeneration) {
    let limit = HELD_END + RADIUS + HELD_CLEAR;
    let group = facing(run);
    let mut from = group.start;
    for s in &group.segments {
        if s.kind.is_fed() {
            let x = from.tip.x.min(s.end.tip.x);
            assert!(
                x >= limit - 1e-6,
                "a fed move reaches X {x}, inside {limit}"
            );
        }
        from = s.end;
    }
}

/// The one Facing note about the grip: information, never a warning.
fn grip_note(run: &Regeneration) -> String {
    assert!(
        !run.warnings.iter().any(|w| w.contains("held start")),
        "the limit is information, not a warning: {:?}",
        run.warnings
    );
    let notes: Vec<_> = run
        .notes
        .iter()
        .filter(|n| n.contains("stops short of the held start"))
        .collect();
    assert_eq!(notes.len(), 1, "{:?}", run.notes);
    notes[0].clone()
}

/// Issue #500, the owner's decision of 23 Nov 2026: Box facing on +Z of
/// the Baluster set-up (box stock, start held for 8 mm, the part starting
/// inside the grip) was refused. It now faces up to the grip, the whole
/// tool 0.5 mm clear of it, and its notes say where it stops, why and how
/// much of the requested extent is left. The numbers are worked by hand:
/// the grip ends at X 24.990, so the tool's edge stops at X 25.490; Whole
/// stock asks from the stock's start (X 16.990), Part from the part's
/// (X 17.990). Without the grip the same faces are faced whole.
#[test]
fn req_op_24_req_set_09_baluster_box_facing_stops_short_of_the_held_start() {
    for (extent, left, length) in [
        ("whole_stock", "16.990", "8.500"),
        ("part", "17.990", "7.500"),
    ] {
        let runs = facing_beside_the_grip(
            "box",
            &format!("positive_z = true\nextent = \"{extent}\"\n"),
            2,
        );
        for run in &runs {
            assert_clear_of_the_grip(run);
            let note = grip_note(run);
            for part in [
                "held start (X 16.990 to 24.990)".to_string(),
                "within 0.500 mm".into(),
                "finished from X 25.490".into(),
                format!("X {left} to 25.490 ({length} mm along X)"),
                "left unfaced".into(),
            ] {
                assert!(note.contains(&part), "{extent}: {part} not in {note}");
            }
            // The final-depth rows across +Z: the first one's edge is at
            // the stated limit, and the rows leave no strip.
            let group = facing(run);
            let lowest = group
                .segments
                .iter()
                .filter(|s| s.kind == MoveKind::Cutting)
                .map(|s| s.end.tip.z)
                .fold(f64::INFINITY, f64::min);
            let mut rows = Vec::new();
            let mut from = group.start;
            for s in &group.segments {
                if s.kind == MoveKind::Cutting
                    && (from.tip.z - lowest).abs() < 1e-9
                    && (s.end.tip.z - lowest).abs() < 1e-9
                    && (from.tip.x - s.end.tip.x).abs() < 1e-9
                {
                    rows.push(from.tip.x);
                }
                from = s.end;
            }
            rows.sort_by(f64::total_cmp);
            rows.dedup();
            assert!(
                (rows[0] - RADIUS - (HELD_END + HELD_CLEAR)).abs() < 1e-3,
                "{extent}: the first row's edge is at X {}",
                rows[0] - RADIUS
            );
            assert!(rows.windows(2).all(|w| w[1] - w[0] <= 2. * RADIUS));
        }
        // The two posts read the same canonical path.
        assert_eq!(
            facing(&runs[0]).segments.len(),
            facing(&runs[1]).segments.len()
        );
    }
    // Without the grip the whole stock is faced and nothing is said of it.
    let dir = project(
        "box-no-grip",
        "positive_z = true\nextent = \"whole_stock\"\n",
    );
    let operation = dir.join(format!("operations/{FACING}.toml"));
    let text = std::fs::read_to_string(&operation).unwrap();
    std::fs::write(
        &operation,
        text.replace("shape = \"end\"", "shape = \"box\""),
    )
    .unwrap();
    let setup = dir.join(format!("setups/{SETUP}.toml"));
    let text = std::fs::read_to_string(&setup).unwrap();
    assert!(text.contains("[workholding.start]\nlength = 8.0\n"));
    std::fs::write(
        &setup,
        text.replace("[workholding.start]\nlength = 8.0\n", ""),
    )
    .unwrap();
    let run = Application::open(&dir, Threads::Exactly(2))
        .unwrap()
        .start_regenerate()
        .wait()
        .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    assert!(!run.notes.iter().any(|n| n.contains("stops short")));
    let first = facing(&run)
        .segments
        .iter()
        .filter(|s| s.kind == MoveKind::Cutting)
        .map(|s| s.end.tip.x)
        .fold(f64::INFINITY, f64::min);
    assert!(
        first < HELD_END,
        "without a grip the rows start at X {first}"
    );
}

/// The same Box facing at one and four threads writes the same bytes
/// (GC-07, NFR-06).
#[test]
fn req_op_24_req_gc_07_baluster_box_facing_beside_the_grip_is_deterministic() {
    let extra = "positive_z = true\nextent = \"part\"\n";
    let one = facing_beside_the_grip("box", extra, 1);
    let four = facing_beside_the_grip("box", extra, 4);
    for (a, b) in one.iter().zip(&four) {
        assert_eq!(a.gcode, b.gcode);
    }
}

/// The owner's decision covers the restricted legacy Flat shape too: Flat
/// at A 0 with the Part extent was refused beside the grip. Its passes run
/// along X from the tool's centre limit (the grip's end, the tool's radius
/// and the clearance, X 27.078), but its target is the composite of the part
/// and the held region with the 1 mm stock to leave round them, which lifts
/// the passes' ends off the face. Its note reads where the face is finished
/// from the cut: here the lowest X of the cutting moves at the face, read
/// independently from the path. Round with the Part extent is refused on
/// this set-up for another reason, with or without the grip (a rapid's
/// clearance), so its limit is tested on a block in `openchuck-path`.
#[test]
fn req_op_24_req_set_09_baluster_flat_part_extent_stops_short_of_the_held_start() {
    for run in &facing_beside_the_grip(
        "flat",
        "angles = \"0\"\nsize = 16.0\nextent = \"part\"\n",
        2,
    ) {
        assert_clear_of_the_grip(run);
        let group = facing(run);
        let mut from = group.start;
        let mut first = f64::INFINITY;
        for s in &group.segments {
            if s.kind == MoveKind::Cutting
                && (from.tip.z - s.end.tip.z).abs() < 1e-9
                && (from.tip.y - s.end.tip.y).abs() < 1e-9
                && from.tip.z <= 16.0 + 0.01
            {
                first = first.min(from.tip.x.min(s.end.tip.x));
            }
            from = s.end;
        }
        let limit = HELD_END + RADIUS + HELD_CLEAR;
        assert!(
            first >= limit - 1e-6 && first <= limit + 2. * RADIUS + 1.,
            "{first}"
        );
        let note = grip_note(run);
        for part in [
            "held start (X 16.990 to 24.990)".to_string(),
            format!("finished from X {first:.3}"),
            format!(
                "X 17.990 to {first:.3} ({:.3} mm along X)",
                first - 17.990_234_375
            ),
            "its passes along X end there".into(),
        ] {
            assert!(note.contains(&part), "{part} not in {note}");
        }
    }
}

/// A copy of the Baluster set-up with Facing of `shape` and `extra`, its
/// draft scene and its scene with the generated path, and the run.
fn scenes(
    name: &str,
    shape: &str,
    extra: &str,
) -> (
    openchuck_app::scene::SceneView,
    openchuck_app::scene::SceneView,
    Regeneration,
) {
    let dir = project(name, extra);
    let operation = dir.join(format!("operations/{FACING}.toml"));
    let text = std::fs::read_to_string(&operation).unwrap();
    std::fs::write(
        &operation,
        text.replace("shape = \"end\"", &format!("shape = \"{shape}\"")),
    )
    .unwrap();
    let app = Application::open(&dir, Threads::Exactly(2)).unwrap();
    let run = std::sync::Arc::new(app.start_regenerate().wait().unwrap());
    let draft = app.start_scene(None).wait().unwrap();
    let generated = app
        .start_scene_with_facing(None, Some(run.clone()))
        .wait()
        .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    (draft, generated, std::sync::Arc::try_unwrap(run).unwrap())
}

/// The draft scene alone of a copy of the Baluster set-up with Facing of
/// the given shape, for a set-up that need not generate.
fn draft_scene(name: &str, shape: &str, extra: &str) -> openchuck_app::scene::SceneView {
    let dir = project(name, extra);
    let operation = dir.join(format!("operations/{FACING}.toml"));
    let text = std::fs::read_to_string(&operation).unwrap();
    std::fs::write(
        &operation,
        text.replace("shape = \"end\"", &format!("shape = \"{shape}\"")),
    )
    .unwrap();
    let app = Application::open(&dir, Threads::Exactly(2)).unwrap();
    let draft = app.start_scene(None).wait().unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    draft
}

/// The Facing labels of a scene, with their kinds.
fn facing_labels(
    view: &openchuck_app::scene::SceneView,
) -> Vec<(openchuck_app::facing_labels::FacingLabel, String)> {
    view.labels
        .iter()
        .filter_map(|l| l.facing.map(|t| (t.kind, l.text.clone())))
        .collect()
}

/// Issue #881 (the owner's answers of 25 Nov 2026, decision record
/// 0702): each Facing label in the view is at most 32 characters and
/// says what its number is measured from. On the Baluster set-up, Box
/// facing of all four sides gives one label per face, its finished plane's
/// distance from the rotary axis; one per pair of opposite faces, the
/// distance between them; and one where the faces stop beside the held
/// start, with the X they are faced from. No label carries the operation's
/// name, "draft" or a margin of zero, in the draft and on the generated
/// path alike.
#[test]
fn req_ui_03_req_op_24_baluster_box_labels_are_short_and_say_what_they_measure() {
    let (draft, generated, _) = scenes(
        "box-labels",
        "box",
        "positive_z = true\npositive_y = true\nnegative_z = true\nnegative_y = true\n\
         extent = \"whole_stock\"\n",
    );
    use openchuck_app::facing_labels::FacingLabel;
    for (what, view) in [("draft", &draft), ("generated", &generated)] {
        let labels = facing_labels(view);
        for (_, text) in &labels {
            assert!(text.chars().count() <= 32, "{what}: {text}");
            for word in ["draft", "End facing", "margin", "tessellation"] {
                assert!(!text.contains(word), "{what}: {text}");
            }
        }
        let of = |kind: FacingLabel| -> Vec<&str> {
            labels
                .iter()
                .filter(|(k, _)| *k == kind)
                .map(|(_, t)| t.as_str())
                .collect()
        };
        let mut faces = of(FacingLabel::Face);
        faces.sort_unstable();
        assert_eq!(
            faces,
            [
                "Face A 0°: 13.650 mm from axis",
                "Face A 180°: 13.650 mm from axis",
                "Face A 270°: 13.650 mm from axis",
                "Face A 90°: 13.650 mm from axis",
            ],
            "{what}"
        );
        let mut apart = of(FacingLabel::Separation);
        apart.sort_unstable();
        assert_eq!(
            apart,
            [
                "A 0° to A 180°: 27.300 mm apart",
                "A 90° to A 270°: 27.300 mm apart"
            ],
            "{what}"
        );
        assert_eq!(
            of(FacingLabel::Stop),
            ["Faced from X 25.490 (held start)"],
            "{what}"
        );
        assert!(of(FacingLabel::Margin).is_empty(), "{what}");
    }
}

/// A margin that is not zero has one label of its own, on the margin
/// outline, saying how far past the extent's edges Facing goes.
#[test]
fn req_ui_03_req_op_24_a_facing_margin_has_one_label_of_its_own() {
    let (draft, _, _) = scenes(
        "box-margin",
        "box",
        "positive_z = true\nnegative_z = true\nextent = \"whole_stock\"\n\
         extent_margin = 0.5\n",
    );
    let margins: Vec<String> = facing_labels(&draft)
        .into_iter()
        .filter(|(k, _)| *k == openchuck_app::facing_labels::FacingLabel::Margin)
        .map(|(_, t)| t)
        .collect();
    assert_eq!(margins, ["Margin: 0.500 mm past the edges"]);
}

/// Issue #881: Round facing labels its diameter and X range as a face, and
/// where it stops beside the held start as a stop, each in 32 characters or
/// fewer, with its stop ring tagged as a stop.
#[test]
fn req_ui_03_req_op_24_round_facing_labels_its_face_and_its_stop() {
    let draft = draft_scene("round-labels", "round", "size = 30.0\nextent = \"part\"\n");
    let labels = facing_labels(&draft);
    let stops: Vec<&String> = labels
        .iter()
        .filter(|(k, _)| *k == openchuck_app::facing_labels::FacingLabel::Stop)
        .map(|(_, t)| t)
        .collect();
    assert_eq!(stops.len(), 1, "{labels:?}");
    assert!(stops[0].starts_with("Faced from X ") && stops[0].ends_with(" (held start)"));
    assert!(labels.iter().any(|(k, t)| {
        *k == openchuck_app::facing_labels::FacingLabel::Face && t.starts_with('Ø')
    }));
    for (_, text) in &labels {
        assert!(text.chars().count() <= 32, "{text}");
    }
    assert!(draft.lines.iter().any(|l| {
        l.facing
            .is_some_and(|t| t.kind == openchuck_app::facing_labels::FacingLabel::Stop)
    }));
}

/// Issue #895 item 1: legacy Flat's face is finished from where its
/// passes reach the face, read from the cut (decision 0685). Before Generate
/// the draft marks the keep-out limit; with the generated path shown, the
/// view has one stop line and one stop label, at the X the note gives, not
/// the draft's.
#[test]
fn req_ui_03_req_op_24_the_generated_flat_stop_marker_is_where_its_cut_stops() {
    let (draft, generated, run) = scenes(
        "flat-stop",
        "flat",
        "angles = \"0\"\nsize = 16.0\nextent = \"part\"\n",
    );
    let limit = HELD_END + RADIUS + HELD_CLEAR;
    let stops = |view: &openchuck_app::scene::SceneView| -> Vec<String> {
        facing_labels(view)
            .into_iter()
            .filter(|(k, _)| *k == openchuck_app::facing_labels::FacingLabel::Stop)
            .map(|(_, t)| t)
            .collect()
    };
    assert_eq!(
        stops(&draft),
        [format!("Faced from X {limit:.3} (held start)")]
    );
    let note = grip_note(&run);
    let at = note.find("finished from X ").unwrap() + "finished from X ".len();
    let x: f64 = note[at..]
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(x > limit + 0.5, "the cut stops past the limit here: {x}");
    assert_eq!(
        stops(&generated),
        [format!("Faced from X {x:.3} (held start)")]
    );
    let stop_lines: Vec<_> = generated
        .lines
        .iter()
        .filter(|l| {
            l.facing
                .is_some_and(|t| t.kind == openchuck_app::facing_labels::FacingLabel::Stop)
        })
        .collect();
    assert_eq!(stop_lines.len(), 1, "{stop_lines:?}");
    assert!(
        stop_lines[0].points.iter().all(|p| (p.x - x).abs() < 1e-3),
        "{stop_lines:?}"
    );
    // The draft is said once, in a note, until the operation is generated.
    let draft_note = |view: &openchuck_app::scene::SceneView| {
        view.notes
            .iter()
            .filter(|n| n.contains("are a draft until you generate"))
            .count()
    };
    assert_eq!(draft_note(&draft), 1, "{:?}", draft.notes);
    assert_eq!(draft_note(&generated), 0, "{:?}", generated.notes);
}

/// The Baluster's round bar (diameter 24, length 63) with Round facing to
/// Finished size `size` over the Part extent, the start held for 8 mm or
/// not (issue #880).
fn round_on_round_bar(name: &str, size: f64, held: bool) -> PathBuf {
    let dir = round_bar(name, 63.0, 2.0, 2.2225);
    let operation = dir.join(format!("operations/{FACING}.toml"));
    let text = std::fs::read_to_string(&operation).unwrap();
    std::fs::write(
        &operation,
        text.replace(
            "shape = \"end\"",
            &format!("shape = \"round\"\nsize = {size:?}\nextent = \"part\""),
        ),
    )
    .unwrap();
    if !held {
        let setup = dir.join(format!("setups/{SETUP}.toml"));
        let text = std::fs::read_to_string(&setup).unwrap();
        assert!(text.contains("\n[workholding.start]\nlength = 8.0\n"));
        std::fs::write(
            &setup,
            text.replace("\n[workholding.start]\nlength = 8.0\n", "\n"),
        )
        .unwrap();
    }
    dir
}

/// Issue #880, the owner's answer A: Round facing over the Part extent on
/// the Baluster was refused by Facing's rapid check ("rapid move N has no
/// continuous complete-tool separation from stock, ..."), with and without
/// the held start, at Finished size 22 and 23.5. Round's links that turn A
/// now rise above the setup's enclosing radius first, so it generates and
/// posts through both posts, and every rapid that turns A is above that
/// radius or off the stock along X.
#[test]
fn req_op_24_req_nfr_07_round_facing_over_the_part_on_the_baluster_bar_generates() {
    for (size, held) in [(22.0, true), (22.0, false), (23.5, true)] {
        let dir = round_on_round_bar("round-880", size, held);
        for name in ["makera-z1", "linuxcnc"] {
            let mut app = Application::open(&dir, Threads::Exactly(2)).unwrap();
            let run = app
                .start_regenerate_with_post(
                    PostSelection::Builtin(name.into()),
                    OutputGrouping::Setup,
                )
                .wait();
            let run = run.unwrap_or_else(|e| panic!("size {size}, held {held}, {name}: {e}"));
            let group = facing(&run);
            let mut from = group.start;
            for s in &group.segments {
                if s.kind == MoveKind::Rapid && (s.end.a.0 - from.a.0).abs() > 1e-9 {
                    let height = |p: &openchuck_types::ir::ToolPose| p.tip.y.hypot(p.tip.z);
                    let off = from.tip.x.max(s.end.tip.x) < 16.990_234_375 - RADIUS
                        || from.tip.x.min(s.end.tip.x) > 16.990_234_375 + 63.0 + RADIUS;
                    assert!(
                        off || height(&from).min(height(&s.end)) >= 12.0,
                        "size {size}, held {held}: a rapid turns A at {} mm from the axis",
                        height(&from).min(height(&s.end))
                    );
                }
                from = s.end;
            }
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
