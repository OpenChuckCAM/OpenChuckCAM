// SPDX-License-Identifier: AGPL-3.0-or-later
//! An end of the part that does not reach round the rotary axis, in a saved
//! project (OP-04, OP-21, SET-09, GC-16, NFR-12; issue #834; decision 0700).
//!
//! A public, synthetic part: a pin 15 mm across from X 0 to 20 whose axis is
//! 10 mm off the rotary axis, in round stock from X -10, gripped 4 mm at its
//! start, with a tab on the axis 4 mm across from the grip to X 0. The pin's
//! start end comes no nearer than 2.5 mm to the axis, so the tab does not
//! meet it. Helical finishing turns beside that end down to the tab, which
//! cuts the part free of the tab: that is said as a possible collision to
//! acknowledge before the files are written, whether or not Face free ends
//! is on. A project saved before this change gives the warning too when it is
//! opened and made again (a visible change for such projects).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use openchuck_app::project::id::{OperationId, SetupId, ToolId};
use openchuck_app::project::{
    Item, Operation, OrderKey, ParameterValue, Placement, Setup, ToolSnapshot,
};
use openchuck_app::setup::{
    HeldRegion, Millimetres, Stock, Tab, TabLayout, TabShape, Workholding, builtin,
};
use openchuck_app::simulation::SimulationSettings;
use openchuck_app::{Application, Command, OutputGrouping, PostSelection, Threads};
use openchuck_testkit::parts::revolve;
use openchuck_types::mesh::TriangleMesh;
use openchuck_types::tool::{ControllerMapping, CuttingData, Tool, ToolShape};
use openchuck_types::units::{FeedRate, SpindleSpeed};

/// What the warning of a tab that misses its end says.
const MISSES: &str = "has a tab past it that does not meet the face";

/// The pin, 7.5 mm in radius from X 0 to 20, its axis 10 mm along +Y.
fn pin() -> TriangleMesh {
    let mut mesh = revolve(&[(0.0, 7.5), (20.0, 7.5)], 720);
    for v in &mut mesh.vertices {
        v.y += 10.0;
    }
    mesh
}

fn write_stl(mesh: &TriangleMesh, path: &Path) {
    let mut text = String::from("solid part\n");
    for t in &mesh.triangles {
        text.push_str("facet normal 0 0 0\n outer loop\n");
        for i in t {
            let p = mesh.vertices[*i as usize];
            text.push_str(&format!("  vertex {:e} {:e} {:e}\n", p.x, p.y, p.z));
        }
        text.push_str(" endloop\nendfacet\n");
    }
    text.push_str("endsolid part\n");
    std::fs::write(path, text).unwrap();
}

struct Built {
    dir: PathBuf,
    app: Application,
}

impl Drop for Built {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The pin in round stock 37 mm across from X -10 to 25, gripped 4 mm at
/// its start, with the tab from X -6 to 0, and helical finishing with a
/// 3.175 mm flat end mill; `parameters` are the operation's own.
fn build(name: &str, parameters: &[(&str, ParameterValue)]) -> Built {
    let dir = std::env::temp_dir().join(format!("openchuck-offaxis-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut app = Application::create(&dir, name, Threads::Exactly(2)).unwrap();
    std::fs::create_dir_all(dir.join("models")).unwrap();
    write_stl(&pin(), &dir.join("models/part.stl"));
    let (model, _) = app.add_model(&dir.join("models/part.stl"), None).unwrap();
    let machine = builtin("makera-z1").unwrap().unwrap();
    app.execute(Command::SetMachine(Some(Box::new(machine))))
        .unwrap();
    app.execute(Command::SetSimulation(SimulationSettings {
        cell: Millimetres(1.0),
    }))
    .unwrap();
    let setup_id = SetupId::new();
    let mut setup = Setup::new(setup_id, "Setup 1");
    setup.part = Some(model);
    setup.material = Some("oak".into());
    setup.stock = Some(Stock::Cylinder {
        diameter: Millimetres(37.0),
        length: Millimetres(35.0),
        x_start: Millimetres(-10.0),
    });
    setup.workholding = Workholding {
        start: Some(HeldRegion {
            length: Millimetres(4.0),
            diameter: None,
        }),
        end: None,
        tabs: vec![Tab {
            x_start: Millimetres(-6.0),
            length: Millimetres(6.0),
            shape: TabShape::Cylinder {
                diameter: Millimetres(4.0),
            },
            layout: TabLayout::OnAxis,
            auto: None,
        }],
    };
    let tool_id = ToolId::new();
    let cutting_data = BTreeMap::from([(
        "oak".to_owned(),
        CuttingData {
            spindle_speed: SpindleSpeed(12000.0),
            feed_per_tooth: Millimetres(0.02),
            plunge_feed: FeedRate(150.0),
            stepover: Millimetres(1.0),
            stepdown: Millimetres(2.0),
        },
    )]);
    let snapshot = ToolSnapshot {
        id: tool_id,
        order: OrderKey::default(),
        name: "3.175 mm flat".into(),
        tool: Tool {
            shape: ToolShape::FlatEnd,
            diameter: Millimetres(3.175),
            flute_length: Millimetres(25.0),
            overall_length: Millimetres(38.0),
            shank_diameter: Millimetres(3.175),
            flute_count: 2,
            material: None,
            centre_cutting: Some(true),
            stick_out: None,
        },
        controller: ControllerMapping {
            number: 1,
            length_offset: None,
            diameter_offset: None,
        },
        cutting_data,
    };
    let items = vec![
        Item::Setup(setup),
        Item::Tool(snapshot),
        Item::Operation(Operation {
            enabled: true,
            id: OperationId::new(),
            order: OrderKey::default(),
            name: "Helical Finishing".into(),
            setup: setup_id,
            strategy: "helical_finishing".into(),
            tool: Some(tool_id),
            selections: Vec::new(),
            boundary: None,
            avoid: Vec::new(),
            check: Vec::new(),
            restriction_options: Default::default(),
            spindle_speed: None,
            material: None,
            parameters: [
                ("stepover", ParameterValue::Number(2.0)),
                ("adaptive_spacing", ParameterValue::Flag(false)),
            ]
            .into_iter()
            .chain(parameters.iter().cloned())
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
        }),
    ];
    for item in items {
        app.execute(Command::Add {
            item,
            placement: Placement::End,
        })
        .unwrap();
    }
    Built { dir, app }
}

fn post(app: &mut Application) -> openchuck_app::Regeneration {
    app.set_post(Some(PostSelection::Builtin("makera-z1".into())));
    app.set_output_grouping(OutputGrouping::Setup);
    app.regenerate().unwrap_or_else(|e| panic!("{e}"))
}

/// The possible collisions the run lists for writing that are the tab's.
fn missed(app: &Application, run: &openchuck_app::Regeneration) -> Vec<String> {
    openchuck_app::collision_warnings(app.project(), run)
        .into_iter()
        .filter(|t| t.contains(MISSES))
        .collect()
}

#[test]
fn req_set_09_req_gc_16_a_saved_project_whose_tab_misses_an_end_asks_for_acknowledgement() {
    // A project saved with no Cutting direction (and Face free ends left at
    // its default), as one saved before the setting existed, opened again.
    let built = build("saved", &[]);
    built.app.save().unwrap();
    let mut app = Application::open(&built.dir, Threads::Exactly(3)).unwrap();
    let run = std::sync::Arc::new(post(&mut app));
    let found = missed(&app, &run);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("Helical Finishing")
            && found[0].contains("the turn beside the face cuts the part free of the tab"),
        "{found:?}"
    );
    // Writing waits for the acknowledgement of the possible collision, and
    // then writes (decision 0344, decision 0538).
    let out = built.dir.join("out");
    let mut plan = app.prepare_output(&out, &run).unwrap();
    assert!(
        plan.collisions().iter().any(|c| c.contains(MISSES)),
        "{:?}",
        plan.collisions()
    );
    assert!(!plan.collisions_acknowledged());
    let refused = app.write_output(&plan, false);
    assert!(refused.written.is_empty());
    assert!(
        refused
            .failed
            .as_ref()
            .unwrap()
            .reason
            .contains("possible collision"),
        "{refused:?}"
    );
    plan.acknowledge_collisions();
    let report = app.write_output(&plan, false);
    assert!(report.is_complete(), "{report:?}");
}

#[test]
fn req_set_09_the_tab_warning_is_given_with_face_free_ends_off() {
    // The turn beside the face is the tab's own (decision 0297), not Face
    // free ends', so turning the setting off keeps the warning.
    let mut built = build(
        "faceoff",
        &[("face_free_ends", ParameterValue::Flag(false))],
    );
    let run = post(&mut built.app);
    let found = missed(&built.app, &run);
    assert_eq!(found.len(), 1, "{found:?}");
}
