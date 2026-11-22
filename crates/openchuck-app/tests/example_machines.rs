// SPDX-License-Identifier: AGPL-3.0-or-later
//! The public example projects follow the built-in Makera Z1 (SET-01,
//! decision 0649 item 11): each one's machine copy is the built-in profile
//! value for value, the minimum clearance radius and its provenance
//! included, so a setup in them gets the same checks as a new Z1 project
//! (issue #809, decision 0674). The blisk example's generator writes the
//! committed copy byte for byte, so running it again cannot put an old value
//! back, such as the A limit of 3600 deg/min (issue #797, decision 0647).
#![allow(clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};

use openchuck_project::machines::{MAKERA_Z1, builtin};

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

/// Every example project of `testdata/examples` holds a copy of the
/// built-in Z1 equal to it, at a project schema that holds the minimum
/// clearance radius (SET-01; issue #809: the baluster example was schema 25,
/// whose copy could not hold it).
#[test]
fn req_set_01_req_set_06_the_example_projects_hold_the_builtin_z1() {
    let z1 = builtin(MAKERA_Z1).unwrap().unwrap();
    assert_eq!(z1.min_clearance_radius.map(|r| r.0), Some(45.0));
    for folder in [
        "testdata/examples/baluster/baluster-project",
        "testdata/examples/blisk-start/blisk-project",
        "testdata/examples/crankshaft-simultaneous/crankshaft-project",
    ] {
        let project = openchuck_project::file::load(&repo(folder)).unwrap();
        let copy = project.machine().unwrap();
        assert_eq!(copy, &z1, "{folder}: the machine copy differs from the built-in Z1");
        // Schema 26 added the minimum clearance radius (decision 0495).
        let text = std::fs::read_to_string(repo(folder).join("project.toml")).unwrap();
        let schema: i64 = text
            .strip_prefix("schema = ")
            .and_then(|t| t.lines().next())
            .and_then(|n| n.parse().ok())
            .unwrap();
        assert!(
            (26..=openchuck_project::file::SCHEMA).contains(&schema),
            "{folder}: schema {schema} cannot hold the minimum clearance radius"
        );
    }
}

/// `generate.py` of the blisk example embeds the machine text it writes;
/// it must be the committed `machine.toml`, byte for byte (issue #797).
#[test]
fn req_set_01_the_blisk_generator_writes_the_committed_machine_copy() {
    let script = std::fs::read_to_string(repo("testdata/examples/blisk-start/generate.py")).unwrap();
    let start = script.find("MACHINE = \"\"\"").unwrap() + "MACHINE = \"\"\"".len();
    let len = script[start..].find("\"\"\"").unwrap();
    let embedded = &script[start..start + len];
    let committed =
        std::fs::read_to_string(repo("testdata/examples/blisk-start/blisk-project/machine.toml"))
            .unwrap();
    assert_eq!(embedded, committed);
    assert!(!embedded.contains("rotary = 3600"), "the old A limit");
}
