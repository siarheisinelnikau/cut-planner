use cut_planner::{model::*, solver::*, units::*, validate::*};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

fn run(p: &Project) -> Plan {
    solve(
        p,
        SolveOptions {
            candidates: 8,
            budget: Duration::from_secs(30),
        },
        &AtomicBool::new(false),
        |_, _| {},
    )
    .unwrap()
}

fn fixture(w: i64, h: i64, pw: i64, ph: i64, quantity: u32) -> Project {
    let mut p = Project::example();
    p.stock.truncate(1);
    p.parts.truncate(1);
    p.stock[0].width = w * MM;
    p.stock[0].height = h * MM;
    p.stock[0].quantity = 1;
    p.stock[0].grain = Grain::None;
    p.parts[0].width = pw * MM;
    p.parts[0].height = ph * MM;
    p.parts[0].quantity = quantity;
    p.parts[0].grain = Grain::None;
    p.constraints = Constraints {
        kerf: 3 * MM,
        trim: 0,
        min_offcut: 10 * MM,
    };
    p
}

#[test]
fn exact_units_and_invalid_input() {
    assert_eq!(Unit::Inch.parse("1.125").unwrap(), 28_575_000);
    assert_eq!(Unit::Mm.parse("28,575").unwrap(), 28_575_000);
    assert_eq!(Unit::Inch.parse(".0001").unwrap(), 2540);
    for invalid in [
        "-1",
        "NaN",
        "1e4",
        "1.2.3",
        "1.00001",
        "999999999999999999999",
        "",
    ] {
        assert!(Unit::Mm.parse(invalid).is_err(), "{invalid}");
    }
    assert_eq!(Unit::Cm.format(28_575_000), "2.8575");
}

#[test]
fn kerf_is_real_material_and_cannot_be_shrunk() {
    let p = fixture(100, 100, 50, 100, 2);
    let plan = run(&p);
    assert_eq!(plan.stats.placed, 1);
    assert_eq!(plan.unplaced.len(), 1);
    assert_eq!(plan.stats.kerf_area, 3 * 100 * i128::from(MM).pow(2));
    let p = fixture(103, 100, 50, 100, 2);
    assert_eq!(run(&p).stats.placed, 2);
    let p = fixture(100, 100, 99, 100, 1);
    assert_eq!(
        run(&p).stats.placed,
        0,
        "a sub-kerf sliver is not a full saw cut"
    );
}

#[test]
fn grain_rotation_and_oversized_reasons() {
    let mut p = fixture(100, 200, 200, 100, 1);
    p.stock[0].grain = Grain::Height;
    p.parts[0].grain = Grain::Width;
    assert_eq!(run(&p).stats.placed, 1);
    p.parts[0].rotate = false;
    assert_eq!(run(&p).unplaced[0].reason, UnplacedReason::Size);
    p.parts[0].rotate = true;
    p.stock[0].grain = Grain::Width;
    assert_eq!(run(&p).unplaced[0].reason, UnplacedReason::Grain);
    p.stock[0].grain = Grain::None;
    assert_eq!(run(&p).unplaced[0].reason, UnplacedReason::Grain);
}

#[test]
fn trim_is_in_the_tree_and_stock_is_counted() {
    let mut p = fixture(110, 110, 100, 100, 3);
    p.stock[0].quantity = 2;
    p.constraints.trim = 5 * MM;
    let plan = run(&p);
    assert_eq!(plan.sheets.len(), 2);
    assert_eq!(plan.stats.placed, 2);
    assert_eq!(plan.unplaced.len(), 1);
    assert_eq!(
        plan.sheets[0]
            .steps()
            .iter()
            .filter(|s| s.purpose == Purpose::Trim)
            .count(),
        4
    );
    validate_plan(&p, &plan).unwrap();
    p.constraints.trim = p.constraints.kerf;
    validate_plan(&p, &run(&p)).unwrap();
    p.constraints.trim = MM;
    assert_eq!(p.validate().unwrap_err().key, "trim_kerf");
}

#[test]
fn mixed_stock_determinism_and_exact_multiplicities() {
    let mut p = fixture(100, 100, 80, 80, 5);
    let mut larger = p.stock[0].clone();
    larger.id = 2;
    larger.width = 200 * MM;
    larger.height = 200 * MM;
    larger.quantity = 1;
    p.stock.push(larger);
    let plan = run(&p);
    assert_eq!(plan.stats.placed, 5);
    assert_eq!(plan.sheets.len(), 2);
    assert_eq!(plan, run(&p));
    validate_plan(&p, &plan).unwrap();
}

#[test]
fn cancellation_preserves_an_accounted_candidate() {
    let p = Project::example();
    let cancel = AtomicBool::new(true);
    assert!(matches!(
        solve(&p, SolveOptions::default(), &cancel, |_, _| {}),
        Err(SolveError::Cancelled)
    ));
    cancel.store(false, Ordering::Relaxed);
    let plan = solve(&p, SolveOptions::default(), &cancel, |_, _| {
        cancel.store(true, Ordering::Relaxed);
    })
    .unwrap();
    assert!(plan.cancelled);
    assert_eq!(plan.candidates, 1);
    validate_plan(&p, &plan).unwrap();
}

#[test]
fn validator_rejects_tampered_geometry_counts_cycles_and_kerf() {
    let p = Project::example();
    let plan = run(&p);
    let mut bad = plan.clone();
    bad.sheets[0].nodes[0].rect.w += 1;
    assert!(validate_plan(&p, &bad).is_err());
    let mut bad = plan.clone();
    if let NodeKind::Cut { ref mut kerf, .. } = bad.sheets[0].nodes[0].kind {
        *kerf += 1;
    }
    assert!(validate_plan(&p, &bad).is_err());
    let mut bad = plan.clone();
    if let NodeKind::Cut { ref mut low, .. } = bad.sheets[0].nodes[0].kind {
        *low = Some(0);
    }
    assert!(validate_plan(&p, &bad).is_err());
    let mut bad = plan.clone();
    bad.unplaced.push(Unplaced {
        id: p.parts[0].id,
        copy: 0,
        reason: UnplacedReason::Stock,
    });
    assert!(validate_plan(&p, &bad).is_err());
    let mut bad = plan.clone();
    let part = bad.sheets[0]
        .nodes
        .iter_mut()
        .find(|n| matches!(n.kind, NodeKind::Part { .. }))
        .unwrap();
    part.kind = NodeKind::Offcut { interior: true };
    bad.stats = statistics(&p, &bad);
    assert!(validate_plan(&p, &bad).unwrap_err().contains("omitted"));
}

#[test]
fn generated_small_fixtures_conserve_area_and_validate() {
    let mut seed = 0x59f7_u64;
    for case in 0..72 {
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            seed >> 32
        };
        let mut p = fixture(
            300 + (next() % 500) as i64,
            300 + (next() % 500) as i64,
            1,
            1,
            1,
        );
        p.stock[0].quantity = 3;
        p.constraints.kerf = (next() % 5) as i64 * MM;
        p.constraints.trim = if case % 2 == 0 { 10 * MM } else { 0 };
        p.parts.clear();
        for id in 0..8 {
            p.parts.push(Part {
                id,
                name: format!("Part {id}"),
                width: (30 + next() % 600) as i64 * MM,
                height: (30 + next() % 600) as i64 * MM,
                quantity: 1 + (next() % 3) as u32,
                rotate: next() % 2 == 0,
                grain: Grain::None,
            });
        }
        let plan = run(&p);
        validate_plan(&p, &plan).unwrap_or_else(|e| panic!("case {case}: {e}"));
        assert_eq!(plan.stats.placed + plan.unplaced.len(), plan.stats.required);
    }
}

#[test]
fn maximum_quantity_uses_a_flat_valid_tree_and_respects_candidate_budget() {
    let p = fixture(25_997, 1000, 10, 1000, 2000);
    let plan = solve(
        &p,
        SolveOptions {
            candidates: 8,
            budget: Duration::ZERO,
        },
        &AtomicBool::new(false),
        |_, _| {},
    )
    .unwrap();
    assert_eq!(plan.candidates, 1);
    assert_eq!(plan.stats.placed, 2000);
    assert!(plan.unplaced.is_empty());
    validate_plan(&p, &plan).unwrap();
    assert_eq!(plan.sheets[0].steps().len(), 1999);
    assert_eq!(next_row_id([1, u64::MAX, 2, 4].into_iter()), 3);
}

#[test]
fn minimum_trim_absorbs_a_thin_sliver_without_changing_finished_parts() {
    let mut p = fixture(110, 1220, 100, 300, 4);
    p.constraints.trim = 5 * MM;
    let plan = run(&p);
    assert_eq!(plan.stats.placed, 4);
    assert_eq!(plan.sheets.len(), 1);
    validate_plan(&p, &plan).unwrap();
    let bottom = plan.sheets[0]
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Part { .. }))
        .map(|n| n.rect.y + n.rect.h)
        .max()
        .unwrap();
    assert_eq!(p.stock[0].height - bottom, 6 * MM);
    assert!(
        plan.sheets[0]
            .nodes
            .iter()
            .all(|n| !matches!(n.kind, NodeKind::Part { .. })
                || n.rect.w == 100 * MM && n.rect.h == 300 * MM)
    );
}
