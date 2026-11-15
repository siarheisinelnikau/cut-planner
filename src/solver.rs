//! Deterministic best-fit guillotine heuristics. This is not an optimality proof.
use crate::model::*;
use crate::units::Length;
use std::cmp::Reverse;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub struct SolveOptions {
    pub candidates: usize,
    /// Stop starting new candidates at this deadline. An in-flight candidate is finite.
    pub budget: Duration,
}
impl Default for SolveOptions {
    fn default() -> Self {
        Self {
            candidates: 8,
            budget: Duration::from_secs(3),
        }
    }
}

#[derive(Debug)]
pub enum SolveError {
    Input(InputError),
    Empty,
    Cancelled,
    InvalidPlan(String),
}
impl std::fmt::Display for SolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(e) => e.fmt(f),
            Self::Empty => f.write_str("stock_and_parts_required"),
            Self::Cancelled => f.write_str("cancelled"),
            Self::InvalidPlan(e) => write!(f, "invalid_plan: {e}"),
        }
    }
}
impl std::error::Error for SolveError {}

/// Eight finite candidates, a single worker, no random seed or background improvement loop.
pub fn solve(
    project: &Project,
    options: SolveOptions,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, usize),
) -> Result<Plan, SolveError> {
    project.validate().map_err(SolveError::Input)?;
    if project.parts.is_empty() || project.stock.is_empty() {
        return Err(SolveError::Empty);
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(SolveError::Cancelled);
    }
    let started = Instant::now();
    let count = options.candidates.clamp(1, 8);
    let mut best: Option<Plan> = None;
    let mut tried = 0;
    for candidate in 0..count {
        if tried > 0 && (started.elapsed() >= options.budget || cancel.load(Ordering::Relaxed)) {
            break;
        }
        let mut plan = candidate_plan(project, candidate, cancel);
        plan.stats = crate::validate::statistics(project, &plan);
        crate::validate::validate_plan(project, &plan).map_err(SolveError::InvalidPlan)?;
        tried += 1;
        if best.as_ref().is_none_or(|old| rank(&plan) < rank(old)) {
            best = Some(plan);
        }
        progress(tried, count);
    }
    let mut best = best.expect("at least one bounded candidate is evaluated");
    best.candidates = tried;
    best.cancelled = cancel.load(Ordering::Relaxed);
    Ok(best)
}

fn rank(plan: &Plan) -> (usize, i128, usize, i128) {
    (
        plan.unplaced.len(),
        plan.stats.stock_area,
        plan.sheets.len(),
        -plan.stats.part_area,
    )
}

fn candidate_plan(project: &Project, candidate: usize, cancel: &AtomicBool) -> Plan {
    let mut items: Vec<(usize, u32)> = project
        .parts
        .iter()
        .enumerate()
        .flat_map(|(i, p)| (0..p.quantity).map(move |copy| (i, copy)))
        .collect();
    items.sort_by_key(|(i, copy)| {
        let p = &project.parts[*i];
        let (w, h) = (i128::from(p.width), i128::from(p.height));
        let key = match candidate / 2 {
            0 => (w * h, w.max(h)),
            1 => (w.max(h), w * h),
            2 => (w, h),
            _ => (h, w),
        };
        (Reverse(key), p.id, *copy)
    });
    let mut plan = Plan::default();
    let mut used = vec![0_u32; project.stock.len()];
    for (i, copy) in items {
        let part = &project.parts[i];
        if cancel.load(Ordering::Relaxed) {
            plan.unplaced.push(Unplaced {
                id: part.id,
                copy,
                reason: UnplacedReason::Cancelled,
            });
            continue;
        }
        if let Some((sheet, node, rotated)) = find_space(project, &plan, part) {
            place(
                &mut plan.sheets[sheet],
                node,
                part,
                copy,
                rotated,
                project.constraints.kerf,
                candidate.is_multiple_of(2),
            );
            continue;
        }
        let mut stock_choices = Vec::new();
        for (s, stock) in project.stock.iter().enumerate() {
            if used[s] >= stock.quantity {
                continue;
            }
            let trim = project.constraints.trim;
            let rect = Rect {
                x: trim,
                y: trim,
                w: stock.width - 2 * trim,
                h: stock.height - 2 * trim,
            };
            for rotated in [false, true] {
                let (dx, dy) = if candidate >= 6 {
                    trim_alignment(stock, &project.constraints, Some(part_size(part, rotated)))
                } else {
                    (0, 0)
                };
                let rect = Rect {
                    w: rect.w - dx,
                    h: rect.h - dy,
                    ..rect
                };
                if orientation_allowed(part, stock, rotated)
                    && fits(rect, part, rotated, project.constraints.kerf)
                {
                    stock_choices.push((
                        i128::from(stock.width) * i128::from(stock.height),
                        s,
                        rotated,
                    ));
                }
            }
        }
        // Alternate sheet-size choice as well as cut direction to escape small-sheet traps.
        stock_choices.sort_by_key(|(area, s, rotated)| {
            (if candidate >= 6 { -*area } else { *area }, *s, *rotated)
        });
        if let Some((_, s, rotated)) = stock_choices.first().copied() {
            let align = if candidate >= 6 {
                Some(part_size(part, rotated))
            } else {
                None
            };
            let (mut sheet, usable) =
                prepare_sheet(&project.stock[s], used[s], &project.constraints, align);
            used[s] += 1;
            place(
                &mut sheet,
                usable,
                part,
                copy,
                rotated,
                project.constraints.kerf,
                candidate.is_multiple_of(2),
            );
            plan.sheets.push(sheet);
        } else {
            plan.unplaced.push(Unplaced {
                id: part.id,
                copy,
                reason: unplaced_reason(project, part),
            });
        }
    }
    plan
}

fn find_space(project: &Project, plan: &Plan, part: &Part) -> Option<(usize, usize, bool)> {
    let mut best = None;
    for (s, sheet) in plan.sheets.iter().enumerate() {
        let stock = project
            .stock
            .iter()
            .find(|v| v.id == sheet.stock_id)
            .expect("generated stock id");
        for (n, node) in sheet.nodes.iter().enumerate() {
            if !matches!(node.kind, NodeKind::Offcut { interior: true }) {
                continue;
            }
            for rotated in [false, true] {
                if !orientation_allowed(part, stock, rotated)
                    || !fits(node.rect, part, rotated, project.constraints.kerf)
                {
                    continue;
                }
                let (w, h) = part_size(part, rotated);
                let score = (
                    node.rect.area() - i128::from(w) * i128::from(h),
                    (node.rect.w - w).min(node.rect.h - h),
                    s,
                    n,
                    rotated,
                );
                if best.as_ref().is_none_or(|old| score < *old) {
                    best = Some(score);
                }
            }
        }
    }
    best.map(|(_, _, s, n, r)| (s, n, r))
}

pub(crate) fn fits(rect: Rect, part: &Part, rotated: bool, kerf: Length) -> bool {
    let (w, h) = part_size(part, rotated);
    dimension_fits(rect.w, w, kerf) && dimension_fits(rect.h, h, kerf)
}
fn dimension_fits(available: Length, required: Length, kerf: Length) -> bool {
    available == required || required + kerf <= available
}

fn unplaced_reason(project: &Project, part: &Part) -> UnplacedReason {
    let mut geometry_fits = false;
    for stock in &project.stock {
        let t = project.constraints.trim;
        let rect = Rect {
            x: t,
            y: t,
            w: stock.width - 2 * t,
            h: stock.height - 2 * t,
        };
        for rotated in [false, true] {
            let (dx, dy) =
                trim_alignment(stock, &project.constraints, Some(part_size(part, rotated)));
            let adjusted = Rect {
                w: rect.w - dx,
                h: rect.h - dy,
                ..rect
            };
            if (rotated && !part.rotate)
                || (!fits(rect, part, rotated, project.constraints.kerf)
                    && !fits(adjusted, part, rotated, project.constraints.kerf))
            {
                continue;
            }
            geometry_fits = true;
            if orientation_allowed(part, stock, rotated) {
                return UnplacedReason::Stock;
            }
        }
    }
    if geometry_fits {
        UnplacedReason::Grain
    } else {
        UnplacedReason::Size
    }
}

fn trim_alignment(
    stock: &Stock,
    c: &Constraints,
    align: Option<(Length, Length)>,
) -> (Length, Length) {
    if c.trim == 0 {
        return (0, 0);
    }
    let extra = |available: Length, part: Length| {
        let count = (available + c.kerf) / (part + c.kerf);
        let remainder = if count > 0 {
            available - count * part - (count - 1) * c.kerf
        } else {
            0
        };
        if remainder > 0 && remainder < c.kerf {
            remainder
        } else {
            0
        }
    };
    align
        .map(|(w, h)| {
            (
                extra(stock.width - 2 * c.trim, w),
                extra(stock.height - 2 * c.trim, h),
            )
        })
        .unwrap_or((0, 0))
}

fn prepare_sheet(
    stock: &Stock,
    copy: u32,
    c: &Constraints,
    align: Option<(Length, Length)>,
) -> (SheetPlan, usize) {
    let mut sheet = SheetPlan {
        stock_id: stock.id,
        copy,
        nodes: vec![Node {
            rect: Rect {
                x: 0,
                y: 0,
                w: stock.width,
                h: stock.height,
            },
            kind: NodeKind::Offcut { interior: true },
        }],
    };
    let mut current = 0;
    if c.trim > 0 {
        // Trim is a minimum margin. Absorb a sub-kerf trailing sliver in an existing
        // trim cut, rather than needing an impossible extra cut after a repeated row.
        // Other candidates preserve the minimum exactly and protect mixed-size yield.
        let (extra_right, extra_bottom) = trim_alignment(stock, c, align);
        let (scrap, next) = split(
            &mut sheet,
            current,
            Axis::Vertical,
            c.trim - c.kerf,
            c.kerf,
            Purpose::Trim,
        );
        mark_scrap(&mut sheet, scrap);
        current = next.expect("validated positive usable width");
        let right = sheet.nodes[current].rect.w - c.trim - extra_right;
        let (next, scrap) = split(
            &mut sheet,
            current,
            Axis::Vertical,
            right,
            c.kerf,
            Purpose::Trim,
        );
        mark_scrap(&mut sheet, scrap);
        current = next.expect("validated positive usable width");
        let (scrap, next) = split(
            &mut sheet,
            current,
            Axis::Horizontal,
            c.trim - c.kerf,
            c.kerf,
            Purpose::Trim,
        );
        mark_scrap(&mut sheet, scrap);
        current = next.expect("validated positive usable height");
        let bottom = sheet.nodes[current].rect.h - c.trim - extra_bottom;
        let (next, scrap) = split(
            &mut sheet,
            current,
            Axis::Horizontal,
            bottom,
            c.kerf,
            Purpose::Trim,
        );
        mark_scrap(&mut sheet, scrap);
        current = next.expect("validated positive usable height");
    }
    (sheet, current)
}

fn mark_scrap(sheet: &mut SheetPlan, index: Option<usize>) {
    if let Some(index) = index {
        sheet.nodes[index].kind = NodeKind::Offcut { interior: false };
    }
}

fn place(
    sheet: &mut SheetPlan,
    node: usize,
    part: &Part,
    copy: u32,
    rotated: bool,
    kerf: Length,
    vertical_first: bool,
) {
    let (w, h) = part_size(part, rotated);
    let mut current = node;
    let axes = if vertical_first {
        [Axis::Vertical, Axis::Horizontal]
    } else {
        [Axis::Horizontal, Axis::Vertical]
    };
    for axis in axes {
        let rect = sheet.nodes[current].rect;
        let (size, required) = match axis {
            Axis::Vertical => (rect.w, w),
            Axis::Horizontal => (rect.h, h),
        };
        if size != required {
            current = split(sheet, current, axis, required, kerf, Purpose::Separate)
                .0
                .expect("positive part size");
        }
    }
    sheet.nodes[current].kind = NodeKind::Part {
        id: part.id,
        copy,
        rotated,
    };
}

fn split(
    sheet: &mut SheetPlan,
    node: usize,
    axis: Axis,
    offset: Length,
    kerf: Length,
    purpose: Purpose,
) -> (Option<usize>, Option<usize>) {
    let r = sheet.nodes[node].rect;
    let (low_rect, high_rect) = match axis {
        Axis::Vertical => (
            Rect { w: offset, ..r },
            Rect {
                x: r.x + offset + kerf,
                w: r.w - offset - kerf,
                ..r
            },
        ),
        Axis::Horizontal => (
            Rect { h: offset, ..r },
            Rect {
                y: r.y + offset + kerf,
                h: r.h - offset - kerf,
                ..r
            },
        ),
    };
    let mut append = |rect: Rect| {
        if rect.w > 0 && rect.h > 0 {
            let index = sheet.nodes.len();
            sheet.nodes.push(Node {
                rect,
                kind: NodeKind::Offcut { interior: true },
            });
            Some(index)
        } else {
            None
        }
    };
    let (low, high) = (append(low_rect), append(high_rect));
    sheet.nodes[node].kind = NodeKind::Cut {
        axis,
        offset,
        kerf,
        low,
        high,
        purpose,
    };
    (low, high)
}
