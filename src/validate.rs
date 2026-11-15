//! Independent cut-tree verifier: no calls to the placement or splitting algorithms.
use crate::model::*;
use std::collections::{HashMap, HashSet};

pub fn statistics(project: &Project, plan: &Plan) -> Statistics {
    let mut result = Statistics {
        required: project.parts.iter().map(|p| p.quantity as usize).sum(),
        ..Statistics::default()
    };
    for sheet in &plan.sheets {
        if let Some(root) = sheet.nodes.first() {
            result.stock_area += root.rect.area();
        }
        for node in &sheet.nodes {
            match node.kind {
                NodeKind::Part { .. } => {
                    result.placed += 1;
                    result.part_area += node.rect.area();
                }
                NodeKind::Cut { axis, kerf, .. } => {
                    result.kerf_area += i128::from(kerf)
                        * i128::from(match axis {
                            Axis::Vertical => node.rect.h,
                            Axis::Horizontal => node.rect.w,
                        });
                }
                _ => {}
            }
        }
    }
    result
}

pub fn validate_plan(project: &Project, plan: &Plan) -> Result<(), String> {
    project.validate().map_err(|e| e.to_string())?;
    let parts: HashMap<_, _> = project.parts.iter().map(|p| (p.id, p)).collect();
    let stock: HashMap<_, _> = project.stock.iter().map(|s| (s.id, s)).collect();
    let mut instances = HashSet::new();
    let mut used_stock = HashSet::new();
    for sheet in &plan.sheets {
        let s = stock.get(&sheet.stock_id).ok_or("unknown stock")?;
        if sheet.copy >= s.quantity || !used_stock.insert((s.id, sheet.copy)) {
            return Err("stock quantity exceeded or duplicated".into());
        }
        if sheet.nodes.is_empty() || sheet.nodes.len() > 4 * MAX_PARTS + 32 {
            return Err("invalid tree size".into());
        }
        let mut todo = vec![(
            0,
            Rect {
                x: 0,
                y: 0,
                w: s.width,
                h: s.height,
            },
        )];
        let mut visited = HashSet::new();
        let mut placed_rects = Vec::new();
        let mut leaf_area = 0_i128;
        let mut kerf_area = 0_i128;
        while let Some((index, expected)) = todo.pop() {
            if !visited.insert(index) {
                return Err("cycle or shared tree child".into());
            }
            let n = sheet.nodes.get(index).ok_or("missing tree child")?;
            if n.rect != expected || expected.w <= 0 || expected.h <= 0 {
                return Err("tree geometry does not partition its parent".into());
            }
            match n.kind {
                NodeKind::Part { id, copy, rotated } => {
                    let p = parts.get(&id).ok_or("unknown part")?;
                    if copy >= p.quantity || !instances.insert((id, copy)) {
                        return Err("part quantity exceeded or duplicated".into());
                    }
                    let (w, h) = part_size(p, rotated);
                    if (w, h) != (n.rect.w, n.rect.h) {
                        return Err("part dimensions changed".into());
                    }
                    if !orientation_allowed(p, s, rotated) {
                        return Err("grain or rotation violated".into());
                    }
                    let t = project.constraints.trim;
                    if n.rect.x < t
                        || n.rect.y < t
                        || n.rect.x + w > s.width - t
                        || n.rect.y + h > s.height - t
                    {
                        return Err("edge trim violated".into());
                    }
                    placed_rects.push(n.rect);
                    leaf_area += n.rect.area();
                }
                NodeKind::Offcut { .. } => {
                    leaf_area += n.rect.area();
                }
                NodeKind::Cut {
                    axis,
                    offset,
                    kerf,
                    low,
                    high,
                    ..
                } => {
                    let size = match axis {
                        Axis::Vertical => n.rect.w,
                        Axis::Horizontal => n.rect.h,
                    };
                    if kerf != project.constraints.kerf
                        || offset < 0
                        || offset > size - kerf
                        || (kerf == 0 && (offset == 0 || offset == size))
                    {
                        return Err("invalid cut offset or kerf".into());
                    }
                    let remaining = size - offset - kerf;
                    if low.is_some() != (offset > 0) || high.is_some() != (remaining > 0) {
                        return Err("missing or zero-area cut result".into());
                    }
                    let (a, b, span) = match axis {
                        Axis::Vertical => (
                            Rect {
                                w: offset,
                                ..n.rect
                            },
                            Rect {
                                x: n.rect.x + offset + kerf,
                                w: remaining,
                                ..n.rect
                            },
                            n.rect.h,
                        ),
                        Axis::Horizontal => (
                            Rect {
                                h: offset,
                                ..n.rect
                            },
                            Rect {
                                y: n.rect.y + offset + kerf,
                                h: remaining,
                                ..n.rect
                            },
                            n.rect.w,
                        ),
                    };
                    kerf_area += i128::from(kerf) * i128::from(span);
                    if let Some(low) = low {
                        todo.push((low, a));
                    }
                    if let Some(high) = high {
                        todo.push((high, b));
                    }
                }
            }
        }
        if visited.len() != sheet.nodes.len() {
            return Err("orphan tree nodes".into());
        }
        if placed_rects.is_empty() {
            return Err("used sheet contains no parts".into());
        }
        if leaf_area + kerf_area != i128::from(s.width) * i128::from(s.height) {
            return Err("sheet area is not conserved".into());
        }
        for (i, r) in placed_rects.iter().enumerate() {
            if placed_rects[i + 1..].iter().any(|other| r.overlaps(*other)) {
                return Err("parts overlap".into());
            }
        }
    }
    for missing in &plan.unplaced {
        let p = parts.get(&missing.id).ok_or("unknown unplaced part")?;
        if missing.copy >= p.quantity || !instances.insert((missing.id, missing.copy)) {
            return Err("unplaced quantity exceeded or duplicated".into());
        }
    }
    for p in &project.parts {
        for copy in 0..p.quantity {
            if !instances.contains(&(p.id, copy)) {
                return Err("required part silently omitted".into());
            }
        }
    }
    let actual = statistics(project, plan);
    if actual != plan.stats {
        return Err("incorrect reported statistics".into());
    }
    Ok(())
}
