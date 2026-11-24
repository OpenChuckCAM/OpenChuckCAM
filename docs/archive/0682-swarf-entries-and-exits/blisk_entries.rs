// SPDX-License-Identifier: AGPL-3.0-or-later
// Helper behind decision record 0682's blisk measurement: pasted at the end of
// crates/openchuck-app/tests/step_strategies.rs (it uses that file's
// `blisk_on_axis` and imports) and run as described in README.md.

#[test]
#[ignore = "scratch measurement for #572"]
fn zz_blisk_572() {
    use openchuck_path::params::names;
    use openchuck_path::region::RegionConstraint;
    use openchuck_path::swarf::Swarf;
    use openchuck_path::{AcceptAll, AnalyticStock, RotaryMapping, StrategyInput};
    use openchuck_testkit::contact::{ConvexTool, Geometry, Settings};
    use openchuck_types::brep::FaceId;
    use openchuck_types::ir::PathTags;
    use openchuck_types::machine::Clearance;

    let model = blisk_on_axis();
    let exact = Geometry::of_faces(&model).unwrap();
    let tool = Tool {
        shape: ToolShape::FlatEnd,
        diameter: Millimetres(3.0),
        flute_length: Millimetres(16.0),
        overall_length: Millimetres(50.0),
        shank_diameter: Millimetres(3.0),
        flute_count: 2,
        material: None,
        centre_cutting: Some(true),
        stick_out: None,
    }
    .profile()
    .unwrap();
    let convex = ConvexTool::new(&tool).unwrap();
    let settings = Settings {
        margin: 0.5,
        tolerance: 0.001,
        budget: 4_000_000,
    };
    let machine = builtin("makera-z1").unwrap().unwrap();
    let rotary = RotaryMapping::of_profile(&machine);
    let stock = AnalyticStock::Cylinder {
        radius: 21.0,
        x_start: 9.0,
        x_end: 61.0,
    };
    let tags = PathTags {
        operation: "op".into(),
        setup: "setup".into(),
        tool: "tool".into(),
    };
    let with = std::env::var("WITH_NEIGHBOURS").is_ok();
    for blade in [7u32, 9, 13, 19] {
        let blade = FaceId(blade);
        let next: Vec<FaceId> = model
            .edges
            .iter()
            .filter(|e| e.faces.contains(&blade))
            .flat_map(|e| e.faces.iter().copied())
            .filter(|f| *f != blade)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let neighbours = BrepSurface::with_faces(&model, &next).unwrap();
        let surface = BrepSurface::with_faces(&model, &[blade]).unwrap();
        let input = StrategyInput {
            surface: &surface,
            stock: &stock,
            collision: &AcceptAll,
            tool: &tool,
            rotary,
            clearance: Clearance {
                radius: Millimetres(26.0),
                safe_z: Millimetres(36.0),
            },
            max_feeds: machine.max_feeds,
            tags: &tags,
            facing: None,
            keep_out: &[],
            shank: None,
        };
        let variants: [(&str, f64, f64); 8] = [
            ("lead", 0.0, 3.0),
            ("lead", 0.5, 3.0),
            ("lead", 1.0, 3.0),
            ("ramp", 0.0, 3.0),
            ("ramp", 0.0, 30.0),
            ("helix", 0.0, 3.0),
            ("helix", 0.0, 30.0),
            ("helix", 0.0, 89.0),
        ];
        for (style, radius, angle) in variants {
            let mut values: BTreeMap<String, openchuck_path::ParameterValue> = [
                (names::TOLERANCE, 0.01),
                (names::FEED, 300.0),
                (names::PLUNGE_FEED, 100.0),
                (names::LEAD_RADIUS, radius),
                (names::RAMP_ANGLE, angle),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), openchuck_path::ParameterValue::Number(v)))
            .collect();
            values.insert(
                names::ENTRY.into(),
                openchuck_path::ParameterValue::Text(style.into()),
            );
            let region = RegionConstraint {
                neighbours: Some(&neighbours),
                ..RegionConstraint::default()
            };
            let cancel = CancelToken::new();
            let ctx = JobContext::new(&cancel, &NoProgress);
            let made = if with {
                openchuck_path::generate_in_region(&Swarf, &input, &region, &values, &ctx)
            } else {
                openchuck_path::generate(&Swarf, &input, &values, &ctx)
            };
            match made {
                Err(e) => {
                    let t = e.to_string();
                    println!(
                        "blade {} {style} r{radius} a{angle}: refused: {}",
                        blade.0,
                        &t[..t.len().min(260)]
                    );
                }
                Ok(path) => {
                    let (mut entry, mut cutting) = (0.0f64, 0.0f64);
                    let mut prev = path.group.start;
                    for s in &path.group.segments {
                        let is_entry = matches!(s.kind, MoveKind::LeadIn | MoveKind::LeadOut);
                        if is_entry || s.kind == MoveKind::Cutting {
                            let (pa, pb) = (
                                rotary.to_axis(prev.tip, prev.a),
                                rotary.to_axis(s.end.tip, s.end.a),
                            );
                            for f in [0.25, 0.5, 0.75, 1.0] {
                                let a = openchuck_types::units::Degrees(
                                    prev.a.0 + (s.end.a.0 - prev.a.0) * f,
                                );
                                let tip = rotary.to_model(
                                    openchuck_path::rotary::AxisPosition {
                                        along: pa.along + (pb.along - pa.along) * f,
                                        across: pa.across + (pb.across - pa.across) * f,
                                        height: pa.height + (pb.height - pa.height) * f,
                                    },
                                    a,
                                );
                                let c = exact
                                    .clearance(&convex, tip, rotary.tool_axis(a).get(), &settings)
                                    .unwrap()
                                    .lower;
                                if is_entry {
                                    entry = entry.min(c)
                                } else {
                                    cutting = cutting.min(c)
                                }
                            }
                        }
                        prev = s.end;
                    }
                    println!(
                        "blade {} {style} r{radius} a{angle}: made: entry/exit deepest {:.4}, pass {:.4}",
                        blade.0, -entry, -cutting
                    );
                }
            }
        }
    }
}
