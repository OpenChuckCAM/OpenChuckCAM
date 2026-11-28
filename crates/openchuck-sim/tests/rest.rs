// SPDX-License-Identifier: AGPL-3.0-or-later
//! Rest machining and pencil clean-up against the simulated stock (OP-09,
//! SIM-06; work breakdown slice 15 review checklist; decision record 0044).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp
)]

use std::collections::BTreeMap;

use openchuck_path::params::names;
use openchuck_path::pencil::pencil_parameter;
use openchuck_path::rest::parameter;
use openchuck_path::{
    AcceptAll, GeneratedPath, HelicalFinishing, MeshSurface, ParameterValue, PencilFinishing,
    RestMachining, RotaryMapping, Strategy, StrategyInput, generate,
};
use openchuck_sim::{DeviationOptions, DexelStock, Simulator, compare};
use openchuck_testkit::parts::revolve;
use openchuck_types::ir::{MoveKind, PathTags};
use openchuck_types::job::{CancelToken, JobContext, NoProgress};
use openchuck_types::machine::{Clearance, MachineProfile};
use openchuck_types::mesh::TriangleMesh;
use openchuck_types::stock::Stock;
use openchuck_types::tool::{RevolvedProfile, Tool, ToolShape};
use openchuck_types::units::Millimetres;

const CELL: f64 = 0.1;

/// Where the shaft steps from its small radius up to its large one.
const STEP_X: f64 = 12.0;
const SMALL: f64 = 6.0;
const LARGE: f64 = 12.0;
/// The part's length along X.
const LENGTH: f64 = 24.0;
/// Where the sloped flank of the second shaft reaches the large radius: 75
/// degrees from the axis.
const SLOPE_X: f64 = 13.6;

fn tool(shape: ToolShape, d: f64) -> Tool {
    Tool {
        shape,
        diameter: Millimetres(d),
        flute_length: Millimetres(20.0),
        overall_length: Millimetres(50.0),
        shank_diameter: Millimetres(d),
        flute_count: 2,
        material: None,
        centre_cutting: None,
        stick_out: None,
    }
}

fn num(x: f64) -> ParameterValue {
    ParameterValue::Number(x)
}

fn values(extra: &[(&str, f64)]) -> BTreeMap<String, ParameterValue> {
    let mut v: BTreeMap<String, ParameterValue> = [
        (names::STEPOVER, 1.0),
        (names::FEED, 400.0),
        (names::PLUNGE_FEED, 200.0),
        (names::STOCK_TO_LEAVE, 0.0),
    ]
    .into_iter()
    .map(|(k, x)| (k.to_owned(), num(x)))
    .collect();
    for (k, x) in extra {
        v.insert((*k).to_owned(), num(*x));
    }
    v
}

/// A stepped shaft: a small cylinder, a flank, a large cylinder. The inside
/// corner where the small cylinder meets the flank is the place a large ball
/// cannot reach.
struct Rig {
    machine: MachineProfile,
    target: TriangleMesh,
    surface: MeshSurface,
    tags: PathTags,
}

impl Rig {
    /// A flank of 75 degrees, which a vertical tool can follow.
    fn new() -> Self {
        Self::with_profile(&[
            (0.0, SMALL),
            (STEP_X, SMALL),
            (SLOPE_X, LARGE),
            (LENGTH, LARGE),
        ])
    }

    /// A true wall, which a vertical tool cannot cut.
    fn walled() -> Self {
        Self::with_profile(&[
            (0.0, SMALL),
            (STEP_X, SMALL),
            (STEP_X, LARGE),
            (LENGTH, LARGE),
        ])
    }

    fn with_profile(profile: &[(f64, f64)]) -> Self {
        let target = revolve(profile, 180);
        Self {
            machine: MachineProfile::example_x_rotary(),
            surface: MeshSurface::new(&target).unwrap(),
            target,
            tags: PathTags {
                operation: "op".to_owned(),
                setup: "setup".to_owned(),
                tool: "tool".to_owned(),
            },
        }
    }

    fn stock(&self) -> DexelStock {
        let stock = Stock::Cylinder {
            diameter: Millimetres(26.0),
            length: Millimetres(LENGTH),
            x_start: Millimetres(0.0),
        };
        DexelStock::from_stock(&stock, None, CELL).unwrap()
    }

    fn generate(
        &self,
        strategy: &dyn Strategy,
        tool: &RevolvedProfile,
        stock: &DexelStock,
        v: &BTreeMap<String, ParameterValue>,
    ) -> openchuck_path::Result<GeneratedPath> {
        let cancel = CancelToken::new();
        let ctx = JobContext::new(&cancel, &NoProgress);
        let input = StrategyInput {
            surface: &self.surface,
            stock,
            collision: &AcceptAll,
            tool,
            rotary: RotaryMapping::of_profile(&self.machine),
            clearance: Clearance {
                radius: Millimetres(19.0),
                safe_z: Millimetres(29.0),
            },
            max_feeds: self.machine.max_feeds,
            tags: &self.tags,
            facing: None,
            keep_out: &[],
            shank: None,
        };
        generate(strategy, &input, v, &ctx)
    }

    fn simulate(
        &self,
        stock: DexelStock,
        tool: &RevolvedProfile,
        path: &GeneratedPath,
    ) -> (DexelStock, usize) {
        let cancel = CancelToken::new();
        let ctx = JobContext::new(&cancel, &NoProgress);
        let mut sim = Simulator::without_checkpoints(stock, &self.machine);
        let result = sim.run_group(&path.group, tool, &ctx).unwrap();
        (sim.into_stock(), result.rapid_cuts.len())
    }

    /// The most material left on the part and the places overcut, as the
    /// simulation's deviation reports them.
    fn worst(&self, stock: &DexelStock) -> (f64, usize) {
        let d = compare(
            stock,
            &self.target,
            &DeviationOptions {
                spacing: 0.25,
                ..DeviationOptions::for_cell(CELL)
            },
        )
        .unwrap();
        (d.worst_remaining.map_or(0.0, |p| p.value), d.overcut_count)
    }

    /// The stock after a finishing pass with a large ball, which leaves the
    /// inside corner.
    fn after_big_ball(&self) -> DexelStock {
        let big = tool(ToolShape::BallEnd, 6.0).profile().unwrap();
        let path = self
            .generate(&HelicalFinishing, &big, &self.stock(), &values(&[]))
            .unwrap();
        let (stock, faults) = self.simulate(self.stock(), &big, &path);
        assert_eq!(faults, 0, "the first operation cuts on a rapid");
        stock
    }
}

fn small_ball() -> RevolvedProfile {
    tool(ToolShape::BallEnd, 1.0).profile().unwrap()
}

fn cutting_x_range(path: &GeneratedPath) -> (f64, f64) {
    path.group
        .segments
        .iter()
        .filter(|s| s.kind == MoveKind::Cutting)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), s| {
            (lo.min(s.end.tip.x), hi.max(s.end.tip.x))
        })
}

#[test]
fn req_op_09_rest_machining_cuts_only_where_material_remains_and_lowers_the_worst_remaining() {
    let rig = Rig::new();
    let stock = rig.after_big_ball();
    let (before, overcut_before) = rig.worst(&stock);
    eprintln!("before the rest operation: {before:.3} mm left, {overcut_before} overcut");
    // In the 105 degree corner a ball of radius 3 leaves 3 (1 / sin 52.5 - 1)
    // = 0.78 mm along the bisector.
    assert!(before > 0.6, "{before}");
    assert_eq!(overcut_before, 0);

    let small = small_ball();
    let v = values(&[(names::STEPOVER, 0.3)]);
    let path = rig
        .generate(&RestMachining, &small, &stock, &v)
        .expect("rest machining has the corner to clean");
    let (x_lo, x_hi) = cutting_x_range(&path);
    eprintln!(
        "rest cuts from X {x_lo:.2} to {x_hi:.2} in {} moves",
        path.group.segments.len()
    );
    // Only the corner is cut: within the reach of the large ball, not the
    // 40 mm of the part.
    assert!(x_lo > STEP_X - 4.5 && x_hi < SLOPE_X + 0.7, "{x_lo} {x_hi}");

    let (after_stock, faults) = rig.simulate(stock, &small, &path);
    assert_eq!(faults, 0, "a rapid move cuts");
    let (after, overcut) = rig.worst(&after_stock);
    eprintln!("after: {after:.3} mm left, {overcut} overcut");
    // The small ball (radius 0.5) leaves 0.13 mm; the simulation knows the
    // stock to a cell.
    assert!(after < 0.3, "{after}");
    assert!(after < before - 0.4, "{before} -> {after}");
    assert_eq!(overcut, 0, "the rest operation gouges");
}

#[test]
fn req_op_09_rest_machining_with_nothing_to_clean_says_so_instead_of_an_odd_path() {
    let rig = Rig::new();
    let stock = rig.after_big_ball();
    let small = small_ball();
    // Take away everything the small ball can reach, then ask again.
    let v = values(&[(names::STEPOVER, 0.3)]);
    let path = rig.generate(&RestMachining, &small, &stock, &v).unwrap();
    let (cleaned, _) = rig.simulate(stock, &small, &path);
    let again = values(&[(names::STEPOVER, 0.3), (parameter::MIN_REMAINING, 0.3)]);
    let err = rig
        .generate(&RestMachining, &small, &cleaned, &again)
        .unwrap_err()
        .to_string();
    assert!(err.contains("nothing to clean"), "{err}");
    assert!(err.contains(parameter::MIN_REMAINING), "{err}");
}

#[test]
fn req_op_09_rest_machining_cleans_the_corner_of_a_true_wall_too() {
    let rig = Rig::walled();
    let stock = rig.after_big_ball();
    let (before, _) = rig.worst(&stock);
    let small = small_ball();
    let v = values(&[(names::STEPOVER, 0.3)]);
    let path = rig.generate(&RestMachining, &small, &stock, &v).unwrap();
    let (cleaned, faults) = rig.simulate(stock, &small, &path);
    assert_eq!(faults, 0);
    let (after, overcut) = rig.worst(&cleaned);
    eprintln!("wall: {before:.3} -> {after:.3} mm left, {overcut} overcut");
    assert!(before > 1.0 && after < before - 0.7, "{before} -> {after}");
    assert_eq!(overcut, 0, "the rest operation gouges");
}

#[test]
fn req_op_09_the_whole_part_is_rest_when_no_earlier_operation_has_cut() {
    let rig = Rig::new();
    let small = small_ball();
    // On the setup's own stock everything stands proud of the part, so the
    // strategy cuts everywhere; the application warns about it (item 6).
    let v = values(&[(names::STEPOVER, 2.0)]);
    let path = rig
        .generate(&RestMachining, &small, &rig.stock(), &v)
        .unwrap();
    let (x_lo, x_hi) = cutting_x_range(&path);
    assert!(x_lo < 1.0 && x_hi > LENGTH - 1.0, "{x_lo} {x_hi}");
    assert!(RestMachining.needs_earlier_operations());
    assert!(PencilFinishing::default().needs_earlier_operations());
}

#[test]
fn req_op_09_pencil_follows_the_crease_within_tolerance_without_gouging() {
    let rig = Rig::new();
    let stock = rig.after_big_ball();
    let (before, _) = rig.worst(&stock);
    let small = small_ball();
    let v = values(&[(pencil_parameter::MAX_STEP, 0.4)]);
    let path = rig
        .generate(&PencilFinishing::default(), &small, &stock, &v)
        .expect("the corner is a crease with material in it");

    // A ball of radius 0.5 in the corner touches the small cylinder and the
    // flank: its centre is on the bisector of the corner, 0.5 / cos 37.5 from
    // it, so the tip is at the small radius and this far back along X.
    let slope = (SLOPE_X - STEP_X).atan2(LARGE - SMALL);
    let (nx, nr) = (-slope.cos(), slope.sin());
    let (bx, br) = (nx, 1.0 + nr);
    let norm = bx.hypot(br);
    let centre_distance = 0.5 / (0.5 * (0.5 * std::f64::consts::PI - slope)).cos().max(1e-9);
    let expected_x = STEP_X + bx / norm * centre_distance;
    let cuts: Vec<_> = path
        .group
        .segments
        .iter()
        .filter(|s| s.kind == MoveKind::Cutting)
        .collect();
    assert!(cuts.len() > 100, "{}", cuts.len());
    for s in &cuts {
        let radial = s.end.tip.y.hypot(s.end.tip.z);
        assert!(
            (s.end.tip.x - expected_x).abs() < 0.02,
            "tip X {} against {expected_x}",
            s.end.tip.x
        );
        assert!((radial - SMALL).abs() < 0.02, "tip radius {radial}");
    }

    let (after_stock, faults) = rig.simulate(stock, &small, &path);
    assert_eq!(faults, 0, "a rapid move cuts");
    let (after, overcut) = rig.worst(&after_stock);
    eprintln!("pencil: {before:.3} -> {after:.3} mm left, {overcut} overcut");
    assert_eq!(overcut, 0, "the pencil pass gouges");
    // One pass cleans a band the width of the tool, not the whole corner.
    assert!(after < 0.6 && after < before - 0.4, "{before} -> {after}");
}

/// A toothed wheel along X: a hub of `root` radius with `teeth` teeth out to
/// `tip`, whose angular width grows from `narrow` degrees at X = 0 to `wide`
/// at X = `length`, so the gap between teeth narrows along the crease at
/// each tooth's root (a blisk's blades, issue 217).
fn toothed(teeth: usize, root: f64, tip: f64, narrow: f64, wide: f64, length: f64) -> TriangleMesh {
    use openchuck_types::geometry::Point3;
    let period = 360.0 / teeth as f64;
    let ring = |x: f64, width: f64| -> Vec<Point3> {
        let mut out = Vec::new();
        for k in 0..teeth {
            let t0 = k as f64 * period;
            for (r, t) in [
                (root, t0),
                (tip, t0 + 1.0),
                (tip, t0 + 1.0 + width),
                (root, t0 + 2.0 + width),
            ] {
                let a = t.to_radians();
                out.push(Point3::new(x, r * a.cos(), r * a.sin()));
            }
        }
        out
    };
    let (r0, r1) = (ring(0.0, narrow), ring(length, wide));
    let m = r0.len() as u32;
    let mut mesh = TriangleMesh::default();
    mesh.vertices.extend(r0);
    mesh.vertices.extend(r1);
    mesh.vertices.push(Point3::new(0.0, 0.0, 0.0));
    mesh.vertices.push(Point3::new(length, 0.0, 0.0));
    let (c0, c1) = (2 * m, 2 * m + 1);
    for i in 0..m {
        let j = (i + 1) % m;
        let (a0, b0, a1, b1) = (i, j, i + m, j + m);
        mesh.triangles.push([a0, b0, b1]);
        mesh.triangles.push([a0, b1, a1]);
        mesh.triangles.push([c0, b0, a0]);
        mesh.triangles.push([c1, a1, b1]);
    }
    mesh
}

/// Issue 217 (NFR-12): on the owner's blisk, pencil clean-up with a 2 mm
/// flat end cut about 1 mm into the blade tips. Where the gap between the
/// teeth is wider than the tool, it rests in the root; where it is narrower,
/// the drop rests it on the tips. Each placement is a drop, so neither
/// gouges, but the straight move from one to the next went through the
/// tooth. The simulated stock must show no overcut.
#[test]
fn req_op_09_req_nfr_12_pencil_never_cuts_through_a_tooth_between_two_rests() {
    let target = toothed(8, 10.0, 15.0, 14.0, 37.0, 8.0);
    target.validate().unwrap();
    let rig = Rig {
        surface: MeshSurface::new(&target).unwrap(),
        target,
        ..Rig::new()
    };
    let stock = DexelStock::from_stock(
        &Stock::Cylinder {
            diameter: Millimetres(30.0),
            length: Millimetres(8.0),
            x_start: Millimetres(0.0),
        },
        None,
        CELL,
    )
    .unwrap();
    let flat = tool(ToolShape::FlatEnd, 2.0).profile().unwrap();
    let v = values(&[
        (pencil_parameter::MAX_STEP, 0.5),
        (parameter::MIN_REMAINING, 0.0),
    ]);
    let path = rig
        .generate(&PencilFinishing::default(), &flat, &stock, &v)
        .expect("the roots of the teeth are creases");
    let (after, faults) = rig.simulate(stock, &flat, &path);
    assert_eq!(faults, 0, "a rapid move cuts");
    let d = compare(&after, &rig.target, &DeviationOptions::for_cell(CELL)).unwrap();
    let deepest = d.worst_overcut.map_or(0.0, |p| -p.value);
    assert_eq!(
        d.overcut_count,
        0,
        "the pencil pass cuts {deepest:.3} mm into the part at {:?}",
        d.overcut_places.first().map(|p| p.point)
    );
}

#[test]
fn req_op_09_pencil_with_no_inside_corner_says_so() {
    let target = revolve(&[(0.0, 8.0), (30.0, 8.0)], 90);
    let rig = Rig {
        surface: MeshSurface::new(&target).unwrap(),
        target,
        ..Rig::new()
    };
    let small = small_ball();
    let err = rig
        .generate(
            &PencilFinishing::default(),
            &small,
            &rig.stock(),
            &values(&[]),
        )
        .unwrap_err()
        .to_string();
    assert!(err.contains("no inside corner"), "{err}");
}

#[test]
fn req_nfr_06_req_op_09_rest_machining_is_identical_at_1_2_and_n_threads() {
    let rig = Rig::new();
    let stock = rig.after_big_ball();
    let small = small_ball();
    let v = values(&[(names::STEPOVER, 0.3)]);
    let runs = openchuck_testkit::determinism::check(|| {
        let rest = rig.generate(&RestMachining, &small, &stock, &v).unwrap();
        let pencil = rig
            .generate(&PencilFinishing::default(), &small, &stock, &values(&[]))
            .unwrap();
        format!("{:?}{:?}", rest.group, pencil.group).into_bytes()
    })
    .unwrap();
    assert_eq!(runs.len(), 4);
}

// ---- A flat end in the sharp corner of a STEP part (issue #632; decision 0684) ----

/// The closed-form shouldered shaft of decision 0226 (a bar of radius 8 with a
/// sharp shoulder out to radius 12 at X 12), finished axially with a 3.175 mm
/// tapered ball (tip radius 0.5, 5 degrees), as in the OWNERCHECKS project of
/// issue #632. The stock reaches X 8 to 16, about the corner, so the test
/// stays short. The finish is cut once and shared.
struct Corner {
    model: openchuck_types::brep::BrepModel,
    machine: MachineProfile,
    tags: PathTags,
    after: DexelStock,
}

/// The X of the sharp inside corner, and the bar's radius there.
const CORNER_X: f64 = 12.0;
const BAR: f64 = 8.0;

fn corner() -> &'static Corner {
    static CORNER: std::sync::OnceLock<Corner> = std::sync::OnceLock::new();
    CORNER.get_or_init(|| {
        let model = openchuck_testkit::brep::shouldered_shaft(4);
        let machine = MachineProfile::example_x_rotary();
        let tags = PathTags {
            operation: "op".to_owned(),
            setup: "setup".to_owned(),
            tool: "tool".to_owned(),
        };
        let fresh = DexelStock::from_stock(
            &Stock::Cylinder {
                diameter: Millimetres(26.0),
                length: Millimetres(8.0),
                x_start: Millimetres(8.0),
            },
            None,
            CELL,
        )
        .unwrap();
        let tapered = tapered_ball();
        let surface = openchuck_path::BrepSurface::new(&model).unwrap();
        let cancel = CancelToken::new();
        let ctx = JobContext::new(&cancel, &NoProgress);
        let input = corner_input(&surface, &fresh, &tapered, &machine, &tags);
        let axial = generate(
            &openchuck_path::AxialFinishing,
            &input,
            &values(&[(names::STEPOVER, 1.5)]),
            &ctx,
        )
        .unwrap();
        let mut sim = Simulator::without_checkpoints(fresh, &machine);
        let result = sim.run_group(&axial.group, &tapered, &ctx).unwrap();
        assert!(
            result.rapid_cuts.is_empty(),
            "the axial finish cuts on a rapid"
        );
        Corner {
            model,
            machine,
            tags,
            after: sim.into_stock(),
        }
    })
}

fn tapered_ball() -> RevolvedProfile {
    Tool {
        shape: ToolShape::TaperedBall {
            tip_radius: Millimetres(0.5),
            taper_half_angle: openchuck_types::units::Degrees(5.0),
        },
        diameter: Millimetres(3.175),
        flute_length: Millimetres(20.0),
        overall_length: Millimetres(50.0),
        shank_diameter: Millimetres(3.175),
        flute_count: 2,
        material: None,
        centre_cutting: None,
        stick_out: None,
    }
    .profile()
    .unwrap()
}

fn flat_2mm() -> RevolvedProfile {
    Tool {
        flute_length: Millimetres(10.0),
        ..tool(ToolShape::FlatEnd, 2.0)
    }
    .profile()
    .unwrap()
}

fn corner_input<'a>(
    surface: &'a openchuck_path::BrepSurface,
    stock: &'a DexelStock,
    tool: &'a RevolvedProfile,
    machine: &'a MachineProfile,
    tags: &'a PathTags,
) -> StrategyInput<'a> {
    StrategyInput {
        surface,
        stock,
        collision: &AcceptAll,
        tool,
        rotary: RotaryMapping::of_profile(machine),
        clearance: Clearance {
            radius: Millimetres(19.0),
            safe_z: Millimetres(29.0),
        },
        max_feeds: machine.max_feeds,
        tags,
        facing: None,
        keep_out: &[],
        shank: None,
    }
}

/// Pencil with the 2 mm flat end in the corner, with Exact STEP surface
/// contact `exact`, its defaults and `extra`.
fn corner_pencil(
    exact: bool,
    extra: &[(&str, f64)],
) -> openchuck_path::Result<openchuck_path::StrategyOutput> {
    let c = corner();
    let surface = openchuck_path::BrepSurface::new(&c.model)
        .unwrap()
        .with_exact_drops(exact);
    let flat = flat_2mm();
    let input = corner_input(&surface, &c.after, &flat, &c.machine, &c.tags);
    let pencil = PencilFinishing::on_brep(&c.model).unwrap();
    let parameters = pencil.schema().resolve(&values(extra)).unwrap();
    let cancel = CancelToken::new();
    let ctx = JobContext::new(&cancel, &NoProgress);
    pencil.passes_with_notes(&input, None, &parameters, &ctx)
}

/// The degrees of the turn about X a pass covers, from its contact points.
fn degrees_covered(pass: &openchuck_path::Pass) -> f64 {
    pass.points
        .windows(2)
        .map(|w| {
            let a = w[0].tip.z.atan2(w[0].tip.y);
            let b = w[1].tip.z.atan2(w[1].tip.y);
            let d = (b - a).abs().to_degrees();
            if d > 180.0 { 360.0 - d } else { d }
        })
        .sum()
}

#[test]
fn req_op_09_req_op_27_req_nfr_12_a_flat_end_cleans_the_sharp_corner_off_and_on_without_lifting() {
    // Issue #632. The corner's material is a thin wedge on the wall. Judged
    // from the drop's contact, which a flat end's line contact puts anywhere
    // along its flat bottom, the corner was refused with exact contact off
    // and cut in 31 stretches with it on; and a rest flush with the wall was
    // a knife edge for the drop, which read the tool as resting on the
    // shoulder 4 mm up. Judged from the tool's axis (M1), with the rest a
    // tenth of the tolerance clear of the wall (P1), both settings give one
    // pass right round the corner, on the bar, clear of the wall. One test,
    // because the finish before it is the costly part.
    let tolerance = 0.01;
    for exact in [false, true] {
        let out = corner_pencil(exact, &[]).unwrap();
        assert_eq!(
            out.passes.len(),
            1,
            "exact contact {exact}: {} passes",
            out.passes.len()
        );
        let turn = degrees_covered(&out.passes[0]);
        assert!(turn > 359.0, "exact contact {exact}: {turn} degrees");
        for p in &out.passes[0].points {
            let radial = p.tip.y.hypot(p.tip.z);
            assert!(
                (radial - BAR).abs() < tolerance,
                "exact contact {exact}: a tip at radius {radial}"
            );
            let clear = CORNER_X - 1.0 - p.tip.x;
            assert!(
                clear > 0.05 * tolerance && clear < tolerance,
                "exact contact {exact}: the tool's side {clear} mm from the wall"
            );
        }
    }

    // The pass (exact contact off, the default) takes the wedge the tapered
    // ball left, and cuts nothing it should not.
    let c = corner();
    let surface = openchuck_path::BrepSurface::new(&c.model).unwrap();
    let flat = flat_2mm();
    let input = corner_input(&surface, &c.after, &flat, &c.machine, &c.tags);
    let cancel = CancelToken::new();
    let ctx = JobContext::new(&cancel, &NoProgress);
    let pencil = PencilFinishing::on_brep(&c.model).unwrap();
    let path = generate(&pencil, &input, &values(&[]), &ctx).unwrap();
    let mut sim = Simulator::without_checkpoints(c.after.clone(), &c.machine);
    let result = sim.run_group(&path.group, &flat, &ctx).unwrap();
    assert!(result.rapid_cuts.is_empty(), "a rapid move cuts");
    let after = sim.into_stock();
    let in_corner = |stock: &DexelStock| {
        openchuck_path::rest::RemainingMap::detect(&surface, stock, 0.15, 0.25, 0.5, &ctx)
            .unwrap()
            .samples()
            .into_iter()
            .filter(|p| (p.x - CORNER_X).abs() < 1.0 && p.y.hypot(p.z) < BAR + 2.0)
            .count()
    };
    let (before, now) = (in_corner(&c.after), in_corner(&after));
    assert!(before > 100, "the tapered ball leaves the corner: {before}");
    assert!(
        now * 10 < before,
        "the corner keeps {now} of {before} samples"
    );
    // The test's stock covers only part of the 40 mm part, so the part
    // beyond it reads as overcut before and after alike: the pass adds none.
    let (mesh, _) = c.model.joined_tessellation();
    let overcut = |stock: &DexelStock| {
        compare(
            stock,
            &mesh,
            &DeviationOptions {
                spacing: 0.25,
                ..DeviationOptions::for_cell(CELL)
            },
        )
        .unwrap()
        .overcut_count
    };
    assert_eq!(overcut(&after), overcut(&c.after), "the pencil pass gouges");
}

#[test]
fn req_op_09_req_nfr_12_a_corner_shorter_than_the_shortest_cut_is_refused_and_said() {
    // D2 with the owner's Shortest cut (decision record 0684 item
    // 3): a stretch shorter than it is not cut. The stepped shaft's corner is
    // a circle of radius 6, about 38 mm round: a shortest cut of 40 mm leaves
    // nothing to cut, and the refusal names the setting; at its default of 0
    // the whole corner is one pass.
    let rig = Rig::new();
    let stock = rig.after_big_ball();
    let small = small_ball();
    let all = rig
        .generate(&PencilFinishing::default(), &small, &stock, &values(&[]))
        .unwrap();
    assert!(
        all.group
            .segments
            .iter()
            .any(|s| s.kind == MoveKind::Cutting)
    );
    let err = rig
        .generate(
            &PencilFinishing::default(),
            &small,
            &stock,
            &values(&[(pencil_parameter::MIN_LENGTH, 40.0)]),
        )
        .unwrap_err();
    let text = err.to_string();
    assert!(text.contains("shortest cut (40 mm)"), "{text}");
    assert!(
        matches!(
            &err,
            openchuck_path::PathError::Refused { setting: Some(s), .. }
                if s == pencil_parameter::MIN_LENGTH
        ),
        "{err:?}"
    );
}

#[test]
fn req_op_09_a_shortest_cut_of_tool_diameter_is_the_tools_diameter() {
    // Issue #892 (the owner's answer A): Tool diameter is read from the
    // operation's tool when the path is made, so it gives the path the
    // tool's diameter as a number gives.
    let rig = Rig::new();
    let stock = rig.after_big_ball();
    let small = small_ball();
    let with = |x: ParameterValue| {
        let mut v = values(&[]);
        v.insert(pencil_parameter::MIN_LENGTH.to_owned(), x);
        rig.generate(&PencilFinishing::default(), &small, &stock, &v)
            .unwrap()
    };
    let by_word = with(ParameterValue::Text(
        openchuck_path::params::TOOL_DIAMETER.to_owned(),
    ));
    let by_number = with(num(2.0 * small.cutting_radius()));
    assert_eq!(by_word.group, by_number.group);
    assert_eq!(by_word.notices, by_number.notices);
}

#[test]
fn req_op_09_req_nfr_12_pencil_counts_the_corners_material_with_stock_to_leave() {
    // Decision 0684 item 1 with stock to leave (CodeRabbit on pull request
    // #856): the tool's axis rests its radius plus the stock to leave from
    // the part, and the reach from the axis takes the leave too. Beside a
    // true wall (parallel to the tool's axis), with 0.3 mm to leave, the
    // corner's material is found and cut.
    let rig = Rig::walled();
    let stock = rig.after_big_ball();
    let small = small_ball();
    let path = rig
        .generate(
            &PencilFinishing::default(),
            &small,
            &stock,
            &values(&[(names::STOCK_TO_LEAVE, 0.3)]),
        )
        .expect("the corner holds material beyond the stock to leave");
    assert!(
        path.group
            .segments
            .iter()
            .any(|s| s.kind == MoveKind::Cutting),
        "no cut"
    );
}
