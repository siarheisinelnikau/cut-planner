//! Bounded RFC 4180 exchange. An optional unit column overrides the display unit.
use crate::{
    model::{Grain, MAX_ROWS, Part},
    storage::MAX_FILE_SIZE,
    units::Unit,
};

const HEADERS: [&str; 6] = ["name", "width", "height", "quantity", "rotate", "grain"];

pub fn import_parts(text: &str, unit: Unit) -> Result<Vec<Part>, String> {
    let rows = parse(text)?;
    let first = rows.first().ok_or("csv_empty")?;
    let headers = first
        .iter()
        .map(|s| s.trim().to_lowercase())
        .collect::<Vec<_>>();
    let has_unit = headers.len() == 7 && headers[6] == "unit";
    if headers.get(..6) != Some(&HEADERS.map(str::to_owned)) || (!has_unit && headers.len() != 6) {
        return Err("csv_headers".into());
    }
    if rows.len() > MAX_ROWS + 1 {
        return Err("row_limit".into());
    }
    let mut parts = Vec::new();
    for (i, row) in rows.iter().enumerate().skip(1) {
        if row.len() != headers.len() {
            return Err(format!("csv_columns ({})", i + 1));
        }
        let parse = || -> Result<Part, String> {
            let unit = if has_unit {
                match row[6].trim() {
                    "mm" => Unit::Mm,
                    "cm" => Unit::Cm,
                    "in" => Unit::Inch,
                    _ => return Err("csv_unit".into()),
                }
            } else {
                unit
            };
            let grain = match row[5].trim().to_ascii_lowercase().as_str() {
                "none" => Grain::None,
                "width" => Grain::Width,
                "height" => Grain::Height,
                _ => return Err("csv_grain".into()),
            };
            let rotate = match row[4].trim().to_ascii_lowercase().as_str() {
                "true" => true,
                "false" => false,
                _ => return Err("csv_boolean".into()),
            };
            Ok(Part {
                id: i as u64,
                name: row[0].clone(),
                width: unit.parse_csv(&row[1])?,
                height: unit.parse_csv(&row[2])?,
                quantity: row[3].trim().parse().map_err(|_| "positive_quantity")?,
                rotate,
                grain,
            })
        };
        parts.push(parse().map_err(|e| format!("{e} ({})", i + 1))?);
    }
    // Validate all rows and total quantities before the caller replaces its input.
    let mut project = crate::model::Project::empty();
    project.parts = parts;
    project.validate().map_err(|e| e.to_string())?;
    Ok(project.parts)
}

pub fn export_parts(parts: &[Part], _display_unit: Unit) -> String {
    // Millimetres represent every internal nanometre exactly. Inches may be repeating decimals.
    let unit = Unit::Mm;
    let mut result = format!("{},unit\r\n", HEADERS.join(","));
    for part in parts {
        let grain = match part.grain {
            Grain::None => "none",
            Grain::Width => "width",
            Grain::Height => "height",
        };
        result.push_str(
            &[
                quote(&part.name),
                unit.format(part.width),
                unit.format(part.height),
                part.quantity.to_string(),
                part.rotate.to_string(),
                grain.to_owned(),
                "mm".to_owned(),
            ]
            .join(","),
        );
        result.push_str("\r\n");
    }
    result
}

fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn parse(text: &str) -> Result<Vec<Vec<String>>, String> {
    if text.len() as u64 > MAX_FILE_SIZE {
        return Err("file_size_limit".into());
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut chars = text.chars().peekable();
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut closed_quote = false;
    let mut at_start = true;
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    cell.push('"');
                } else {
                    quoted = false;
                    closed_quote = true;
                }
            } else {
                cell.push(c);
            }
            continue;
        }
        match c {
            '"' if at_start => {
                quoted = true;
                at_start = false;
            }
            ',' => {
                row.push(std::mem::take(&mut cell));
                at_start = true;
                closed_quote = false;
                if row.len() > 7 {
                    return Err("csv_columns".into());
                }
            }
            '\r' | '\n' => {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
                at_start = true;
                closed_quote = false;
                if rows.len() > MAX_ROWS + 1 {
                    return Err("row_limit".into());
                }
            }
            _ if closed_quote || c == '"' => return Err("csv_quotes".into()),
            _ => {
                cell.push(c);
                at_start = false;
            }
        }
    }
    if quoted {
        return Err("csv_quotes".into());
    }
    if !at_start || !row.is_empty() || !cell.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    Ok(rows)
}
