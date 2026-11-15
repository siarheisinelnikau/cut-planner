use cut_planner::{csv, model::Project, storage::*, units::Unit};

fn directory() -> tempfile::TempDir {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-data");
    std::fs::create_dir_all(&path).unwrap();
    tempfile::tempdir_in(path).unwrap()
}

#[test]
fn project_and_preferences_round_trip_without_losing_previous_file() {
    let dir = directory();
    let path = dir.path().join("example.cutplan");
    let mut project = Project::example();
    project.name = "Étagères / Полки / Regale".into();
    save_project(&path, &project).unwrap();
    assert_eq!(load_project(&path).unwrap(), project);
    let old = std::fs::read(&path).unwrap();
    project.stock[0].quantity = 0;
    assert!(save_project(&path, &project).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), old);
    let prefs_path = dir.path().join("settings/settings.json");
    let prefs = Preferences {
        language: "ru".into(),
        theme: Theme::Dark,
        reduced_motion: true,
        startup_update_check: false,
    };
    save_preferences(&prefs_path, &prefs).unwrap();
    assert_eq!(load_preferences(&prefs_path).unwrap(), prefs);
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        2,
        "no temporary debris"
    );
}

#[test]
fn corrupt_future_and_oversized_projects_fail_explicitly() {
    let dir = directory();
    let path = dir.path().join("bad.cutplan");
    std::fs::write(&path, "{broken}").unwrap();
    assert!(load_project(&path).is_err());
    save_project(&path, &Project::example()).unwrap();
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("\"schema_version\": 1", "\"schema_version\": 99");
    std::fs::write(&path, text).unwrap();
    assert_eq!(
        load_project(&path).unwrap_err(),
        "unsupported_project_version"
    );
    let f = std::fs::File::create(&path).unwrap();
    f.set_len(MAX_FILE_SIZE + 1).unwrap();
    assert_eq!(load_project(&path).unwrap_err(), "file_size_limit");
}

#[test]
fn quoted_csv_round_trip_and_invalid_rows_are_atomic() {
    let mut p = Project::example();
    p.parts[0].name = "Shelf, \"large\"\nПолка".into();
    let text = csv::export_parts(&p.parts, p.unit);
    assert_eq!(csv::import_parts(&text, p.unit).unwrap(), p.parts);
    for bad in [
        "name,width,height,quantity,rotate,grain\nA,100,10,0,true,none",
        "name,width,height,quantity,rotate,grain\nA,100,10,1,sometimes,none",
        "name,width,height,quantity,rotate,grain\n\"A,100,10,1,true,none",
        "name,width,height,quantity,rotate,grain\n\"A\"x,100,10,1,true,none",
        "name,width,height,quantity,rotate,grain\nA,100,10,3000,true,none",
        "name,width,height,quantity,rotate,grain\nA,NaN,10,1,true,none",
    ] {
        assert!(csv::import_parts(bad, Unit::Mm).is_err(), "{bad}");
    }
}

#[test]
fn csv_preserves_exact_geometry_when_display_units_change() {
    let mut p = Project::example();
    p.parts[0].width = Unit::Inch.parse("0.0001").unwrap();
    let text = csv::export_parts(&p.parts, Unit::Inch);
    assert!(text.contains(",unit\r\n"));
    assert!(text.contains("0.00254"));
    assert_eq!(csv::import_parts(&text, Unit::Cm).unwrap(), p.parts);
    let parts = csv::import_parts(
        "name,width,height,quantity,rotate,grain,unit\r\nA,1.125,2,1,true,none,in",
        Unit::Mm,
    )
    .unwrap();
    assert_eq!(parts[0].width, 28_575_000);
}

#[test]
fn failed_atomic_replacement_removes_only_its_temporary_file() {
    let dir = directory();
    let target = dir.path().join("blocked.cutplan");
    std::fs::create_dir(&target).unwrap();
    assert!(save_project(&target, &Project::example()).is_err());
    assert!(target.is_dir());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    let project = dir.path().join("unknown.cutplan");
    save_project(&project, &Project::example()).unwrap();
    let text = std::fs::read_to_string(&project).unwrap().replace(
        "\"material\":",
        "\"unknown_constraint\": true, \"material\":",
    );
    std::fs::write(&project, &text).unwrap();
    assert!(load_project(&project).is_err());
    assert_eq!(std::fs::read_to_string(&project).unwrap(), text);
}
