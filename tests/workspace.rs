//! State-only tests: no window creation, UI layout, capture, dialogs or live networking.
use cut_planner::{
    model::Project,
    solver::{SolveOptions, solve},
    storage::Preferences,
    ui::{Action, Message, Pending, Stage, Workspace},
};
use std::sync::atomic::AtomicBool;

#[test]
fn stale_solver_result_cannot_replace_a_different_project() {
    let mut workspace = Workspace::new(Preferences::default());
    workspace.replace_project(None, Project::example(), true);
    let generation = workspace.generation;
    let result = solve(
        &workspace.project,
        SolveOptions::default(),
        &AtomicBool::new(false),
        |_, _| {},
    );
    workspace.replace_project(None, Project::empty(), true);
    workspace.handle(Message::Solved { generation, result });
    assert!(workspace.planned.is_none());
    assert_eq!(workspace.stage, Stage::Inputs);
}

#[test]
fn unsaved_navigation_and_failed_save_preserve_input() {
    let mut workspace = Workspace::new(Preferences::default());
    workspace.replace_project(None, Project::example(), false);
    let original = workspace.project.clone();
    workspace.request(Pending::New);
    assert!(
        workspace.take_actions().is_empty(),
        "wait for the unsaved-work decision"
    );
    workspace.handle(Message::Saved {
        result: Err("synthetic write failure".into()),
        snapshot: original.clone(),
        after: Some(Pending::New),
    });
    assert_eq!(workspace.project, original);
    assert!(workspace.dirty());
    assert!(workspace.take_actions().is_empty());
    workspace.handle(Message::Saved {
        result: Ok(None),
        snapshot: original.clone(),
        after: Some(Pending::New),
    });
    assert!(workspace.dirty());
    assert!(workspace.take_actions().is_empty());
    workspace.handle(Message::Saved {
        result: Ok(Some("fixture.cutplan".into())),
        snapshot: original,
        after: Some(Pending::New),
    });
    assert!(!workspace.dirty());
    assert!(matches!(
        workspace.take_actions().as_slice(),
        [Action::Continue(Pending::New)]
    ));
}

#[test]
fn an_import_failure_keeps_the_original_parts_and_saved_state() {
    let mut workspace = Workspace::new(Preferences::default());
    workspace.replace_project(None, Project::example(), true);
    let original = workspace.project.clone();
    workspace.handle(Message::Imported(Err("csv_quotes".into())));
    assert_eq!(workspace.project, original);
    assert!(!workspace.dirty());
}
