// SPDX-License-Identifier: AGPL-3.0-or-later
//! The Adaptive work limit and the job's memory limit (issue #847; the
//! owner's answers; OP-04, NFR-06, NFR-07, GC-07, NFR-12).
//!
//! Each operation with adaptive spacing has an Adaptive work limit, the most
//! geometry queries it may make (1,000,000 by default, as decision record
//! 0239 item 8 set it, up to 1,000,000,000), and its points and added
//! passes rise with it. A job whose points would need more memory than its
//! host lets it use is stopped with an error, and nothing is made.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

mod support;

use std::collections::BTreeMap;

use openchuck_path::adaptive::parameter::{ADAPTIVE_SPACING, WORK_LIMIT};
use openchuck_path::adaptive::{BYTES_PER_POINT, DEFAULT_WORK_LIMIT, MOST_WORK_LIMIT};
use openchuck_path::finishing::parameter::FACE_FREE_ENDS;
use openchuck_path::params::{ParameterKind, names};
use openchuck_path::{
    AxialFinishing, Facing, HelicalFinishing, IndexedFinishing, MeshSurface, ParameterValue,
    PathError, RotaryRoughing, SimultaneousFinishing, Strategy, generate,
};
use openchuck_testkit::parts::{Cylinder, RadialPart};
use openchuck_types::job::{CancelToken, JobContext, MemoryLimit, NoProgress};

use support::{Fixture, ball};

/// The approved tooltip (the owner's wording, 28 Nov 2026).
const TOOLTIP: &str = "The most geometry queries adaptive spacing may make for this operation. \
     A geometry query is one drop of the tool's shape onto the part along a line, or one ray: a \
     search of the whole part. Where the limit is met, the path made so far is kept and a \
     Partial path warning asks you to acknowledge it. Raising the limit lets adaptive spacing \
     check and add passes in more places, but it takes longer, roughly in proportion: the \
     largest values can take hours. The points and passes it may make rise in step, up to \
     100,000,000 points, which can require over 64GB of memory at high settings; if a job \
     would need more than this computer can give it, it is stopped with an error and nothing \
     is made. A larger Allowed excess, a larger Smallest spacing, a smaller Stepover or a \
     smaller selection need fewer queries. The same limit always gives the same program.";

const ADAPTIVE: [&dyn Strategy; 6] = [
    &HelicalFinishing,
    &AxialFinishing,
    &IndexedFinishing,
    &SimultaneousFinishing,
    &RotaryRoughing,
    &Facing,
];

#[test]
fn req_op_04_every_adaptive_strategy_has_an_adaptive_work_limit_under_its_own_name() {
    for s in ADAPTIVE {
        let schema = s.schema();
        let spec = schema
            .get(WORK_LIMIT)
            .unwrap_or_else(|| panic!("{}", s.name()));
        assert_eq!(spec.label, "Adaptive work limit", "{}", s.name());
        assert_eq!(spec.description, TOOLTIP, "{}", s.name());
        assert_eq!(
            spec.kind,
            ParameterKind::Integer {
                min: 1_000_000,
                max: 1_000_000_000,
                default: Some(1_000_000),
            },
            "{}",
            s.name()
        );
        // Off, it is not shown, as the other adaptive settings are not.
        let off = BTreeMap::from([(ADAPTIVE_SPACING.to_owned(), ParameterValue::Flag(false))]);
        assert!(s.schema_for(&off).get(WORK_LIMIT).is_none(), "{}", s.name());
    }
    assert_eq!(DEFAULT_WORK_LIMIT, 1_000_000);
    assert_eq!(MOST_WORK_LIMIT, 1_000_000_000);
}

fn fixture() -> Fixture {
    cylinder(12.0)
}

/// A cylinder of radius 10 and `length` mm, finished with a 4 mm ball.
fn cylinder(length: f64) -> Fixture {
    let part = Cylinder {
        radius: 10.0,
        length,
    };
    Fixture::new(
        MeshSurface::new(&part.mesh(49, 360)).unwrap(),
        12.0,
        &ball(4.0),
    )
}

fn values(extra: &[(&str, ParameterValue)]) -> BTreeMap<String, ParameterValue> {
    let mut v: BTreeMap<String, ParameterValue> = [
        (names::STEPOVER, 3.0),
        (names::TOLERANCE, 0.002),
        (names::FEED, 600.0),
        (names::PLUNGE_FEED, 200.0),
        (names::STOCK_TO_LEAVE, 0.0),
    ]
    .into_iter()
    .map(|(k, x)| (k.to_owned(), ParameterValue::Number(x)))
    .collect();
    v.insert(FACE_FREE_ENDS.to_owned(), ParameterValue::Flag(false));
    v.insert(ADAPTIVE_SPACING.to_owned(), ParameterValue::Flag(true));
    for (k, x) in extra {
        v.insert((*k).to_owned(), x.clone());
    }
    v
}

fn helical(
    f: &Fixture,
    v: &BTreeMap<String, ParameterValue>,
    memory: Option<u64>,
) -> openchuck_path::Result<openchuck_path::GeneratedPath> {
    let cancel = CancelToken::new();
    let context = JobContext::new(&cancel, &NoProgress)
        .with_memory(memory.map(|bytes| MemoryLimit { bytes }));
    generate(&HelicalFinishing, &f.input(), v, &context)
}

#[test]
fn req_op_04_req_nfr_07_an_adaptive_work_limit_out_of_its_range_is_refused_by_name() {
    let f = fixture();
    for bad in [999_999_i64, 1_000_000_001] {
        let e = helical(
            &f,
            &values(&[(WORK_LIMIT, ParameterValue::Integer(bad))]),
            None,
        )
        .unwrap_err();
        assert!(
            matches!(&e, PathError::Parameter { name, .. } if name == WORK_LIMIT),
            "{bad}: {e:?}"
        );
    }
    // Its own default, set, makes the same program as not set (saved
    // before the setting).
    let unset = helical(&f, &values(&[]), None).unwrap();
    let set = helical(
        &f,
        &values(&[(WORK_LIMIT, ParameterValue::Integer(1_000_000))]),
        None,
    )
    .unwrap();
    assert_eq!(unset.group, set.group);
}

#[test]
fn req_nfr_07_req_gc_07_a_job_over_its_memory_limit_is_stopped_and_makes_nothing() {
    let f = fixture();
    let v = values(&[]);
    let free = helical(&f, &v, None).unwrap();
    let points = free.group.segments.len() as u64;
    // A limit the job fits within changes nothing.
    let roomy = helical(&f, &v, Some(u64::MAX)).unwrap();
    assert_eq!(roomy.group, free.group);
    // Far less than its points need: stopped with an error, nothing made.
    let small = BYTES_PER_POINT * points / 10;
    let stop = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| helical(&f, &v, Some(small)))
    };
    let reached = match stop(1) {
        Err(PathError::OutOfMemory {
            reached,
            limit,
            work_limit,
        }) => {
            assert_eq!(limit, small);
            assert!(reached > limit);
            assert_eq!(work_limit, DEFAULT_WORK_LIMIT);
            reached
        }
        other => panic!("{other:?}"),
    };
    // Where it stops does not depend on the thread count (NFR-06).
    for threads in [2, 8] {
        match stop(threads) {
            Err(PathError::OutOfMemory { reached: r, .. }) => assert_eq!(r, reached, "{threads}"),
            other => panic!("{threads}: {other:?}"),
        }
    }
}

#[test]
fn req_op_04_req_nfr_12_raising_the_adaptive_work_limit_confirms_more_places() {
    // A 30 mm cylinder at an Allowed excess and Smallest spacing so small
    // that the default limit is met; at twice the limit adaptive spacing goes
    // on, adding passes, and the warning no longer names the default limit.
    let f = cylinder(30.0);
    let tight = [
        (
            openchuck_path::adaptive::parameter::ALLOWED_EXCESS,
            ParameterValue::Number(0.0001),
        ),
        (
            openchuck_path::adaptive::parameter::MIN_SPACING,
            ParameterValue::Number(0.0001),
        ),
    ];
    let warning = |p: &openchuck_path::GeneratedPath| {
        p.notices
            .iter()
            .find(|n| n.topic.as_deref() == Some(openchuck_path::topic::PARTIAL_PATH))
            .map(|n| n.text.clone())
    };
    let default = helical(&f, &values(&tight), None).unwrap();
    let text = warning(&default).expect("the default limit is met");
    assert!(
        text.contains("it met its limit of 1,000,000 geometry queries")
            && text.contains("raise Adaptive work limit (Advanced)"),
        "{text}"
    );
    let mut raised = tight.to_vec();
    raised.push((WORK_LIMIT, ParameterValue::Integer(2_000_000)));
    let more = helical(&f, &values(&raised), None).unwrap();
    assert!(
        more.group.segments.len() > default.group.segments.len(),
        "{} vs {}",
        more.group.segments.len(),
        default.group.segments.len()
    );
    if let Some(text) = warning(&more) {
        assert!(
            !text.contains("its limit of 1,000,000 geometry queries"),
            "{text}"
        );
    }
}
