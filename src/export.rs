use crate::{i18n::tr, model::*, units::Unit, validate::validate_plan};
use std::fmt::Write;

pub const PART_COLORS: [&str; 6] = [
    "#b9dfd4", "#bdd9eb", "#e7d8af", "#d7cbe5", "#cddda9", "#efcdb7",
];

pub fn escape(text: &str) -> String {
    text.chars()
        .filter(|c| *c >= ' ' || *c == '\n' || *c == '\t')
        .map(|c| match c {
            '&' => "&amp;".to_owned(),
            '<' => "&lt;".to_owned(),
            '>' => "&gt;".to_owned(),
            '"' => "&quot;".to_owned(),
            '\'' => "&#39;".to_owned(),
            '\n' | '\t' => " ".to_owned(),
            _ => c.to_string(),
        })
        .collect()
}

pub fn sheet_svg(
    project: &Project,
    plan: &Plan,
    index: usize,
    language: &str,
) -> Result<String, String> {
    validate_plan(project, plan)?;
    let sheet = plan.sheets.get(index).ok_or("sheet_not_found")?;
    Ok(render_sheet(project, sheet, index, language))
}

fn render_sheet(project: &Project, sheet: &SheetPlan, index: usize, language: &str) -> String {
    let t = |key| tr(language, key);
    let root = sheet.nodes[0].rect;
    let stock = project
        .stock
        .iter()
        .find(|s| s.id == sheet.stock_id)
        .expect("verified stock id");
    let u = project.unit;
    let scale = (820.0 / root.w as f64).min(500.0 / root.h as f64);
    let (width, height) = (root.w as f64 * scale, root.h as f64 * scale);
    let sx = |x| 84.0 + x as f64 * scale;
    let sy = |y| 100.0 + y as f64 * scale;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" role=\"img\" aria-labelledby=\"title{index}\" viewBox=\"0 0 1000 {}\" style=\"width:100%;height:auto\"><title id=\"title{index}\">{} {} — {}</title><rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/><g font-family=\"Segoe UI,DejaVu Sans,sans-serif\" fill=\"#18332e\"><text x=\"36\" y=\"30\" font-size=\"20\" font-weight=\"600\">{} {} · {} #{}</text>",
        height + 156.0,
        t("sheet"),
        index + 1,
        escape(&stock.name),
        t("sheet"),
        index + 1,
        escape(&stock.name),
        sheet.copy + 1
    );
    let _ = write!(
        svg,
        "<text x=\"{}\" y=\"76\" text-anchor=\"middle\" font-size=\"15\">{}</text><text x=\"26\" y=\"{}\" transform=\"rotate(-90 26 {})\" text-anchor=\"middle\" font-size=\"15\">{}</text><path d=\"M84 84 H{} M68 100 V{}\" fill=\"none\" stroke=\"#526a65\"/>",
        84.0 + width / 2.0,
        u.label(root.w),
        100.0 + height / 2.0,
        100.0 + height / 2.0,
        u.label(root.h),
        sx(root.w),
        sy(root.h)
    );
    let _ = write!(
        svg,
        "<rect x=\"84\" y=\"100\" width=\"{width}\" height=\"{height}\" fill=\"#eef1ef\" stroke=\"#526a65\"/>"
    );
    for node in &sheet.nodes {
        let NodeKind::Part { id, copy, rotated } = node.kind else {
            continue;
        };
        let p = project
            .parts
            .iter()
            .find(|p| p.id == id)
            .expect("verified part id");
        let color =
            PART_COLORS[project.parts.iter().position(|p| p.id == id).unwrap() % PART_COLORS.len()];
        let r = node.rect;
        let (w, h) = (r.w as f64 * scale, r.h as f64 * scale);
        let name = format!("{} #{}", p.name, copy + 1);
        let _ = write!(
            svg,
            "<g><title>{} · {} × {}{}</title><rect x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" fill=\"{color}\" stroke=\"#52776b\" stroke-width=\"0.7\"/>",
            escape(&name),
            u.format(r.w),
            u.label(r.h),
            if rotated {
                format!(" · {}", t("rotated"))
            } else {
                String::new()
            },
            sx(r.x),
            sy(r.y)
        );
        if w >= 68.0 && h >= 32.0 {
            // Text length is bounded to avoid covering neighbouring geometry.
            let max_chars = ((w - 12.0) / 7.0).max(1.0) as usize;
            let label: String = name.chars().take(max_chars).collect();
            let _ = write!(
                svg,
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"12\">{}</text>",
                sx(r.x) + w / 2.0,
                sy(r.y) + h / 2.0 - 3.0,
                escape(&label)
            );
            if w >= 120.0 && h >= 48.0 {
                let _ = write!(
                    svg,
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"11\">{} × {}</text>",
                    sx(r.x) + w / 2.0,
                    sy(r.y) + h / 2.0 + 13.0,
                    u.format(r.w),
                    u.format(r.h)
                );
            }
        }
        svg.push_str("</g>");
    }
    for (i, step) in sheet.steps().iter().enumerate() {
        let r = step.rect;
        let (x1, y1, x2, y2) = match step.axis {
            Axis::Vertical => (
                sx(r.x + step.offset),
                sy(r.y),
                sx(r.x + step.offset),
                sy(r.y + r.h),
            ),
            Axis::Horizontal => (
                sx(r.x),
                sy(r.y + step.offset),
                sx(r.x + r.w),
                sy(r.y + step.offset),
            ),
        };
        let _ = write!(
            svg,
            "<path d=\"M{x1} {y1} L{x2} {y2}\" fill=\"none\" stroke=\"#9a5228\" stroke-width=\"0.7\" stroke-dasharray=\"4 3\"><title>{} {}</title></path>",
            t("step"),
            i + 1
        );
    }
    let arrow = match stock.grain {
        Grain::None => "",
        Grain::Width => " →",
        Grain::Height => " ↓",
    };
    let _ = write!(
        svg,
        "<text x=\"84\" y=\"{}\" font-size=\"12\">{}: {}{arrow} · {}</text></g></svg>",
        height + 128.0,
        t("grain"),
        t(stock.grain.key()),
        u.symbol()
    );
    svg
}

/// Self-contained report with printable per-sheet diagrams, dimension tables and cut order.
pub fn html_report(project: &Project, plan: &Plan, language: &str) -> Result<String, String> {
    validate_plan(project, plan)?;
    let t = |key| tr(language, key);
    let u = project.unit;
    let mut html = format!(
        "<!doctype html><html lang=\"{}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><title>{} — Cut Planner</title><style>body{{font:15px 'Segoe UI',sans-serif;color:#18332e;max-width:1050px;margin:32px auto;padding:0 20px}}h1{{font-size:30px}}h2{{font-size:23px}}h3{{margin-top:28px}}p{{line-height:1.5}}.muted{{color:#516760}}table{{border-collapse:collapse;width:100%;margin:16px 0 24px;font-size:12px}}th,td{{text-align:left;padding:8px;border-bottom:1px solid #d8e1dc;overflow-wrap:anywhere}}th{{background:#edf3ef}}tr{{break-inside:avoid}}svg{{max-height:650px}}.sheet{{break-before:page}}@media print{{body{{margin:0;max-width:none;padding:0}}@page{{size:A4;margin:12mm}}thead{{display:table-header-group}}svg{{max-height:145mm}}h2,h3{{break-after:avoid}}}}</style><header><h1>{}</h1><p>{} · {}</p><p class=\"muted\">{}</p><p><b>{}:</b> {} · <b>{}:</b> {} / {} · <b>{}:</b> {:.1}%</p><p><b>{}:</b> {} · <b>{}:</b> {} · <b>{}:</b> {:.4} m²</p><p>{}</p><p class=\"muted\">{}</p></header>",
        escape(language),
        escape(&project.name),
        escape(&project.name),
        escape(&project.material),
        u.symbol(),
        t("heuristic"),
        t("sheets_used"),
        plan.sheets.len(),
        t("placed"),
        plan.stats.placed,
        plan.stats.required,
        t("utilization"),
        plan.stats.utilization(),
        t("kerf"),
        u.label(project.constraints.kerf),
        t("trim"),
        u.label(project.constraints.trim),
        t("waste"),
        plan.stats.waste_area() as f64 / 1e18,
        t("report_note"),
        t("unit_note")
    );
    if !plan.unplaced.is_empty() {
        let _ = write!(
            html,
            "<h2>{}</h2><table><thead><tr><th>{}</th><th>{}</th><th>{}</th></tr></thead><tbody>",
            t("unplaced"),
            t("name"),
            t("quantity"),
            t("reason")
        );
        for part in &project.parts {
            for reason in [
                UnplacedReason::Size,
                UnplacedReason::Grain,
                UnplacedReason::Stock,
                UnplacedReason::Cancelled,
            ] {
                let quantity = plan
                    .unplaced
                    .iter()
                    .filter(|m| m.id == part.id && m.reason == reason)
                    .count();
                if quantity > 0 {
                    let _ = write!(
                        html,
                        "<tr><td>{}</td><td>{quantity}</td><td>{}</td></tr>",
                        escape(&part.name),
                        t(reason.key())
                    );
                }
            }
        }
        html.push_str("</tbody></table>");
    }
    for (i, sheet) in plan.sheets.iter().enumerate() {
        let _ = write!(
            html,
            "<section class=\"sheet\"><h2>{} {}</h2>{}<h3>{}</h3><table><thead><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr></thead><tbody>",
            t("sheet"),
            i + 1,
            render_sheet(project, sheet, i, language),
            t("parts"),
            t("name"),
            t("width"),
            t("height"),
            t("origin"),
            t("rotate")
        );
        for node in &sheet.nodes {
            if let NodeKind::Part { id, copy, rotated } = node.kind {
                let p = project.parts.iter().find(|p| p.id == id).unwrap();
                let r = node.rect;
                let _ = write!(
                    html,
                    "<tr><td>{} #{}</td><td>{}</td><td>{}</td><td>{}, {}</td><td>{}</td></tr>",
                    escape(&p.name),
                    copy + 1,
                    u.label(r.w),
                    u.label(r.h),
                    u.format(r.x),
                    u.format(r.y),
                    if rotated { t("rotated") } else { "—" }
                );
            }
        }
        let _ = write!(
            html,
            "</tbody></table><h3>{}</h3><table><thead><tr><th>#</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr></thead><tbody>",
            t("cuts"),
            t("workpiece"),
            t("origin"),
            t("offset"),
            t("kerf"),
            t("reason")
        );
        for (n, step) in sheet.steps().iter().enumerate() {
            let _ = write!(
                html,
                "<tr><td>{}</td><td>{} × {}</td><td>{}, {}</td><td>{} · {}</td><td>{}</td><td>{}</td></tr>",
                n + 1,
                u.format(step.rect.w),
                u.label(step.rect.h),
                u.format(step.rect.x),
                u.format(step.rect.y),
                t(match step.axis {
                    Axis::Vertical => "vertical",
                    Axis::Horizontal => "horizontal",
                }),
                u.label(step.offset),
                u.label(step.kerf),
                t(match step.purpose {
                    Purpose::Trim => "trim_cut",
                    Purpose::Separate => "separate_cut",
                })
            );
        }
        let _ = write!(
            html,
            "</tbody></table><h3>{}</h3><table><thead><tr><th>#</th><th>{}</th><th>{}</th><th>{}</th></tr></thead><tbody>",
            t("offcuts"),
            t("width"),
            t("height"),
            t("origin")
        );
        for (n, r) in sheet.offcuts(project.constraints.min_offcut).enumerate() {
            let _ = write!(
                html,
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}, {}</td></tr>",
                n + 1,
                u.label(r.w),
                u.label(r.h),
                u.format(r.x),
                u.format(r.y)
            );
        }
        html.push_str("</tbody></table></section>");
    }
    html.push_str("<footer><p class=\"muted\">Cut Planner · v");
    html.push_str(env!("CARGO_PKG_VERSION"));
    html.push_str("</p></footer></html>");
    Ok(html)
}

/// Explicitly used by the UI and tests for matching dimension labels.
pub fn size_label(unit: Unit, rect: Rect) -> String {
    format!(
        "{} × {} {}",
        unit.format(rect.w),
        unit.format(rect.h),
        unit.symbol()
    )
}
