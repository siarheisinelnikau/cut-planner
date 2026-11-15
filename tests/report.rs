use cut_planner::{export::*, i18n::LANGUAGES, model::Project, solver::*};
use std::sync::atomic::AtomicBool;

#[test]
fn reports_include_every_cut_dimensions_and_escape_user_text() {
    let mut p = Project::example();
    p.name = "<script>alert(1)</script> & test".into();
    p.parts[0].name = "<unsafe \"part\">".into();
    let plan = solve(
        &p,
        SolveOptions::default(),
        &AtomicBool::new(false),
        |_, _| {},
    )
    .unwrap();
    for (language, _) in LANGUAGES {
        let report = html_report(&p, &plan, language).unwrap();
        assert!(!report.contains("<script>"));
        assert!(report.contains("&lt;script&gt;"));
        assert!(report.contains("2440 mm"));
        assert_eq!(report.matches("<svg ").count(), plan.sheets.len());
        assert!(report.contains("break-before:page"));
        for i in 0..plan.sheets.len() {
            let svg = sheet_svg(&p, &plan, i, language).unwrap();
            assert_eq!(
                svg.matches("stroke-dasharray").count(),
                plan.sheets[i].steps().len()
            );
        }
    }
    let mut invalid = plan;
    invalid.stats.placed += 1;
    assert!(html_report(&p, &invalid, "en").is_err());
}

#[test]
fn included_project_produces_a_reviewable_dimensioned_report() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = cut_planner::storage::load_project(&root.join("examples/workshop-shelves.cutplan"))
        .unwrap();
    assert_eq!(p, Project::example());
    let plan = solve(
        &p,
        SolveOptions::default(),
        &AtomicBool::new(false),
        |_, _| {},
    )
    .unwrap();
    assert_eq!(plan.stats.placed, 14);
    assert!(plan.unplaced.is_empty());
    let directory = root.join(".test-output");
    std::fs::create_dir_all(&directory).unwrap();
    cut_planner::storage::atomic_write(
        &directory.join("workshop-shelves.html"),
        html_report(&p, &plan, "en").unwrap().as_bytes(),
    )
    .unwrap();
    cut_planner::storage::atomic_write(
        &directory.join("workshop-shelves-sheet-1.svg"),
        sheet_svg(&p, &plan, 0, "en").unwrap().as_bytes(),
    )
    .unwrap();
    let table = html_report(&p, &plan, "en").unwrap();
    assert!(table.contains("From left / top of this piece"));
    println!(
        "Fixture report: {} sheets, {} parts, {:.1}% utilization, {} cuts",
        plan.sheets.len(),
        plan.stats.placed,
        plan.stats.utilization(),
        plan.sheets.iter().map(|s| s.steps().len()).sum::<usize>()
    );
}
