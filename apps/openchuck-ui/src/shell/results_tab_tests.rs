// SPDX-License-Identifier: AGPL-3.0-or-later
//! The Results tab of the dock (UI-12, UI-16; issue 475; decision 0519): the
//! Results window is a tab after Jobs on every step, opened by a click or by a
//! failed run, with no entry in the Project or View menu.

#![allow(clippy::unwrap_used)]

use super::DockTab;
use super::frame::Region;
use super::frame_tests::Rig;
use eframe::egui;
use openchuck_app::Regeneration;
use openchuck_app::overview::Step;
use std::sync::Arc;

fn rig(name: &str) -> Rig {
    let mut rig = Rig::cube(name, None);
    rig.size = egui::vec2(1440.0, 900.0);
    rig.settle();
    rig
}

fn centre(rig: &Rig, text: &str) -> egui::Pos2 {
    rig.text_rects()
        .into_iter()
        .find(|(t, _)| t == text)
        .unwrap_or_else(|| panic!("no {text:?}: {:?}", rig.texts()))
        .1
        .center()
}

fn exact(rig: &Rig, text: &str) -> usize {
    rig.texts().iter().filter(|(t, _)| t == text).count()
}

fn two_warnings() -> Arc<Regeneration> {
    Arc::new(Regeneration {
        // The run's warnings: its notes are information, which the label
        // does not count (decision 0536 section 1).
        warnings: vec![
            "alpha: a warning".to_owned(),
            "bravo: another warning".to_owned(),
        ],
        ..Regeneration::default()
    })
}

#[test]
fn req_ui_12_neither_project_nor_view_menu_has_a_results_item() {
    for menu in ["Project", "View"] {
        let mut rig = rig(&format!("results-menu-{menu}"));
        let tab_only = exact(&rig, "Results");
        let at = centre(&rig, menu);
        rig.click(at);
        rig.frame(Vec::new());
        assert!(!rig.shows("Results…"), "{menu}: no Results… item");
        assert_eq!(exact(&rig, "Results"), tab_only, "{menu}: no Results item");
    }
}

#[test]
fn req_ui_12_results_is_a_tab_after_jobs_on_every_step() {
    let mut rig = rig("results-tab-order");
    let results = DockTab::Results;
    rig.shell.step = Step::Operations;
    assert_eq!(
        rig.shell.tabs(),
        [DockTab::Messages, DockTab::Jobs, results]
    );
    rig.shell.step = Step::Setup;
    assert_eq!(
        rig.shell.tabs(),
        [
            DockTab::SetupChecks,
            DockTab::Messages,
            DockTab::Jobs,
            results
        ]
    );
    rig.shell.step = Step::Simulate;
    assert_eq!(
        rig.shell.tabs(),
        [DockTab::Messages, DockTab::Jobs, results, DockTab::Playback]
    );
}

#[test]
fn req_ui_12_there_is_no_results_tab_without_a_project() {
    let mut rig = rig("results-tab-no-project");
    rig.shell.app = None;
    assert!(!rig.shell.tabs().contains(&DockTab::Results));
}

#[test]
fn req_ui_12_the_label_shows_warnings_and_errors_of_the_latest_run() {
    let mut rig = rig("results-tab-label");
    assert_eq!(exact(&rig, "Results"), 1, "no run: no number");
    rig.shell.run.record_run(two_warnings());
    rig.frame(Vec::new());
    assert_eq!(exact(&rig, "Results (2)"), 1);
    // The chips filter the lines, not the count.
    rig.shell.result_filter.warnings = false;
    rig.frame(Vec::new());
    assert_eq!(exact(&rig, "Results (2)"), 1);
}

#[test]
fn req_ui_12_clicking_the_tab_on_the_rail_opens_the_dock_on_it() {
    let mut rig = rig("results-tab-rail");
    rig.shell.toggle_region(Region::Dock);
    rig.settle();
    assert!(rig.shell.collapsed(Region::Dock));
    let at = centre(&rig, "Results");
    rig.click(at);
    rig.frame(Vec::new());
    assert_eq!(rig.shell.dock_tab, DockTab::Results);
    assert!(!rig.shell.collapsed(Region::Dock));
}

#[test]
fn req_ui_12_a_failed_run_selects_the_results_tab_and_opens_the_dock() {
    let mut rig = rig("results-tab-failed");
    rig.shell.toggle_region(Region::Dock);
    rig.settle();
    rig.shell
        .run
        .record_failure("the engine stopped".to_owned());
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    assert_eq!(rig.shell.dock_tab, DockTab::Results);
    assert!(!rig.shell.collapsed(Region::Dock));
    assert!(rig.shows("the engine stopped"));
}

#[test]
fn req_ui_12_the_results_window_is_gone_so_shortcuts_are_not_held_back() {
    let mut rig = rig("results-tab-no-window");
    rig.shell
        .run
        .record_failure("the engine stopped".to_owned());
    rig.frame(Vec::new());
    assert!(!rig.shell.window_open(), "Results is a tab, not a window");
}

#[test]
fn req_ui_12_req_ui_16_clicking_the_tab_opens_a_dock_the_small_window_rule_collapsed() {
    // Review of #480: the tab click checked only the saved setting, so on
    // a small window the dock stayed closed.
    let mut rig = Rig::cube("results-tab-small", None);
    rig.size = egui::vec2(1300.0, 700.0);
    rig.settle();
    assert!(
        !rig.shell.settings.dock.collapsed,
        "not collapsed by the user"
    );
    assert!(
        rig.shell.collapsed(Region::Dock),
        "collapsed by the window size"
    );
    let at = centre(&rig, "Results");
    rig.click(at);
    rig.frame(Vec::new());
    assert_eq!(rig.shell.dock_tab, DockTab::Results);
    assert!(!rig.shell.collapsed(Region::Dock));
}

/// Hiding a toolpath with its eye changes only the view (decision 0169 item
/// 6, GC-07): Results does not call the run out of date (issue #607).
#[test]
fn req_ui_12_req_sim_01_hiding_a_path_does_not_mark_the_run_changed() {
    let mut rig = super::pending_edit_tests::generated("results-hidden-path");
    rig.shell.dock_tab = DockTab::Results;
    rig.settle();
    let stale = "The project has changed since this run. Generate again.";
    assert!(!rig.shows(stale), "fresh run: {:?}", rig.texts());
    let op = rig.shell.app.as_ref().unwrap().project().operations()[0].id;
    rig.shell
        .app
        .as_mut()
        .unwrap()
        .execute(openchuck_app::Command::SetPathsShown {
            operations: vec![op],
            shown: false,
        })
        .unwrap();
    rig.shell.overview_dirty = true;
    rig.settle();
    assert!(!rig.shows(stale), "after hiding: {:?}", rig.texts());
    // An edit that does change the output still says so.
    rig.shell
        .app
        .as_mut()
        .unwrap()
        .set_parameter(
            op,
            "stepover",
            Some(openchuck_app::project::ParameterValue::Number(0.9)),
        )
        .unwrap();
    rig.shell.overview_dirty = true;
    rig.settle();
    assert!(rig.shows(stale), "after an edit: {:?}", rig.texts());
}

/// After a refused Generate, Results shows the refusal alone: nothing of the
/// previous run's summary, lines or count, so its figures cannot be read as
/// the refused run's (issue #766, the owner's answer A).
#[test]
fn req_ui_12_req_nfr_12_a_refused_generate_shows_only_the_refusal() {
    let mut rig = super::pending_edit_tests::generated("results-refused");
    rig.shell.dock_tab = DockTab::Results;
    rig.settle();
    assert!(rig.shows("s to generate"), "{:?}", rig.texts());
    rig.shell
        .run
        .record_failure("Generation (operation 'Finish'): refused for the test.".to_owned());
    rig.frame(Vec::new());
    rig.frame(Vec::new());
    assert!(rig.shows("Generation (operation 'Finish'): refused for the test."));
    assert!(!rig.shows("s to generate"), "{:?}", rig.texts());
    assert!(!rig.shows("The project has changed since this run. Generate again."));
    assert_eq!(
        rig.shell.run.results_count(),
        1,
        "the refusal alone is counted"
    );
    assert_eq!(exact(&rig, "Results (1)"), 1);
}
