use crate::units::{Length, MAX_LENGTH, MM, Unit};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_ROWS: usize = 256;
pub const MAX_PARTS: usize = 2000;
pub const MAX_SHEETS: usize = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grain {
    #[default]
    None,
    Width,
    Height,
}

impl Grain {
    pub const ALL: [Self; 3] = [Self::None, Self::Width, Self::Height];
    pub fn turned(self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Width => Self::Height,
            Self::Height => Self::Width,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::None => "grain_none",
            Self::Width => "grain_width",
            Self::Height => "grain_height",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stock {
    pub id: u64,
    pub name: String,
    pub width: Length,
    pub height: Length,
    pub quantity: u32,
    pub grain: Grain,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    pub id: u64,
    pub name: String,
    pub width: Length,
    pub height: Length,
    pub quantity: u32,
    pub rotate: bool,
    pub grain: Grain,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constraints {
    pub kerf: Length,
    pub trim: Length,
    pub min_offcut: Length,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub name: String,
    pub material: String,
    pub unit: Unit,
    pub stock: Vec<Stock>,
    pub parts: Vec<Part>,
    pub constraints: Constraints,
}

pub fn next_row_id(ids: impl Iterator<Item = u64>) -> u64 {
    let used: HashSet<_> = ids.collect();
    (1..)
        .find(|id| !used.contains(id))
        .expect("finite bounded row list")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputError {
    pub key: &'static str,
    pub row: Option<usize>,
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key)?;
        if let Some(row) = self.row {
            write!(f, " (row {})", row + 1)?;
        }
        Ok(())
    }
}
impl std::error::Error for InputError {}

impl Project {
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            material: String::new(),
            unit: Unit::Mm,
            stock: vec![],
            parts: vec![],
            constraints: Constraints {
                kerf: 3 * MM,
                trim: 5 * MM,
                min_offcut: 50 * MM,
            },
        }
    }

    pub fn example() -> Self {
        Self {
            name: "Workshop shelves".into(),
            material: "Birch plywood · 18 mm".into(),
            unit: Unit::Mm,
            stock: vec![Stock {
                id: 1,
                name: "Plywood".into(),
                width: 2440 * MM,
                height: 1220 * MM,
                quantity: 2,
                grain: Grain::Width,
            }],
            parts: vec![
                Part {
                    id: 1,
                    name: "Side".into(),
                    width: 1000 * MM,
                    height: 300 * MM,
                    quantity: 4,
                    rotate: true,
                    grain: Grain::Width,
                },
                Part {
                    id: 2,
                    name: "Shelf".into(),
                    width: 600 * MM,
                    height: 300 * MM,
                    quantity: 6,
                    rotate: true,
                    grain: Grain::Width,
                },
                Part {
                    id: 3,
                    name: "Brace".into(),
                    width: 600 * MM,
                    height: 80 * MM,
                    quantity: 4,
                    rotate: true,
                    grain: Grain::None,
                },
            ],
            constraints: Constraints {
                kerf: 3 * MM,
                trim: 5 * MM,
                min_offcut: 50 * MM,
            },
        }
    }

    /// Empty lists are valid while editing; the solver requires both lists.
    pub fn validate(&self) -> Result<(), InputError> {
        let error = |key, row| InputError { key, row };
        if self.name.len() > 256 || self.material.len() > 256 {
            return Err(error("name_limit", None));
        }
        if self.stock.len() > MAX_ROWS || self.parts.len() > MAX_ROWS {
            return Err(error("row_limit", None));
        }
        let c = &self.constraints;
        if [c.kerf, c.trim, c.min_offcut]
            .iter()
            .any(|v| *v < 0 || *v > MAX_LENGTH)
        {
            return Err(error("invalid_constraints", None));
        }
        if c.trim > 0 && c.trim < c.kerf {
            return Err(error("trim_kerf", None));
        }
        let mut ids = HashSet::new();
        let mut sheets = 0_u64;
        for (i, s) in self.stock.iter().enumerate() {
            if !ids.insert(s.id) {
                return Err(error("duplicate_id", Some(i)));
            }
            validate_row(&s.name, s.width, s.height, s.quantity)
                .map_err(|key| error(key, Some(i)))?;
            if s.width <= 2 * c.trim || s.height <= 2 * c.trim {
                return Err(error("trim_size", Some(i)));
            }
            sheets += u64::from(s.quantity);
        }
        if sheets > MAX_SHEETS as u64 {
            return Err(error("sheet_limit", None));
        }
        ids.clear();
        let mut parts = 0_u64;
        for (i, p) in self.parts.iter().enumerate() {
            if !ids.insert(p.id) {
                return Err(error("duplicate_id", Some(i)));
            }
            validate_row(&p.name, p.width, p.height, p.quantity)
                .map_err(|key| error(key, Some(i)))?;
            parts += u64::from(p.quantity);
        }
        if parts > MAX_PARTS as u64 {
            return Err(error("part_limit", None));
        }
        Ok(())
    }
}

fn validate_row(
    name: &str,
    width: Length,
    height: Length,
    quantity: u32,
) -> Result<(), &'static str> {
    if name.trim().is_empty() {
        return Err("name_required");
    }
    if name.len() > 256 {
        return Err("name_limit");
    }
    if width <= 0 || height <= 0 {
        return Err("positive_size");
    }
    if width > MAX_LENGTH || height > MAX_LENGTH {
        return Err("size_limit");
    }
    if quantity == 0 {
        return Err("positive_quantity");
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: Length,
    pub y: Length,
    pub w: Length,
    pub h: Length,
}

impl Rect {
    pub fn area(self) -> i128 {
        i128::from(self.w) * i128::from(self.h)
    }
    pub fn overlaps(self, other: Self) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    Trim,
    Separate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Offcut {
        interior: bool,
    },
    Part {
        id: u64,
        copy: u32,
        rotated: bool,
    },
    Cut {
        axis: Axis,
        offset: Length,
        kerf: Length,
        low: Option<usize>,
        high: Option<usize>,
        purpose: Purpose,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub rect: Rect,
    pub kind: NodeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetPlan {
    pub stock_id: u64,
    pub copy: u32,
    /// A flat tree avoids recursive parsing, traversal and destruction at large quantities.
    pub nodes: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CutStep {
    pub node: usize,
    pub rect: Rect,
    pub axis: Axis,
    pub offset: Length,
    pub kerf: Length,
    pub purpose: Purpose,
}

impl SheetPlan {
    /// Parent before children: cut the workpiece before cutting either resulting piece.
    pub fn steps(&self) -> Vec<CutStep> {
        let mut steps = Vec::new();
        let mut pending = vec![0];
        while let Some(index) = pending.pop() {
            let node = &self.nodes[index];
            if let NodeKind::Cut {
                axis,
                offset,
                kerf,
                low,
                high,
                purpose,
            } = node.kind
            {
                steps.push(CutStep {
                    node: index,
                    rect: node.rect,
                    axis,
                    offset,
                    kerf,
                    purpose,
                });
                if let Some(high) = high {
                    pending.push(high);
                }
                if let Some(low) = low {
                    pending.push(low);
                }
            }
        }
        steps
    }

    pub fn offcuts(&self, minimum: Length) -> impl Iterator<Item = Rect> + '_ {
        self.nodes.iter().filter_map(move |n| match n.kind {
            NodeKind::Offcut { interior: true } if n.rect.w >= minimum && n.rect.h >= minimum => {
                Some(n.rect)
            }
            _ => None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnplacedReason {
    Size,
    Grain,
    Stock,
    Cancelled,
}
impl UnplacedReason {
    pub fn key(self) -> &'static str {
        match self {
            Self::Size => "reason_size",
            Self::Grain => "reason_grain",
            Self::Stock => "reason_stock",
            Self::Cancelled => "reason_cancelled",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unplaced {
    pub id: u64,
    pub copy: u32,
    pub reason: UnplacedReason,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Statistics {
    pub required: usize,
    pub placed: usize,
    pub stock_area: i128,
    pub part_area: i128,
    pub kerf_area: i128,
}

impl Statistics {
    pub fn utilization(&self) -> f64 {
        if self.stock_area == 0 {
            0.0
        } else {
            self.part_area as f64 / self.stock_area as f64 * 100.0
        }
    }
    pub fn waste_area(&self) -> i128 {
        self.stock_area - self.part_area
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub sheets: Vec<SheetPlan>,
    pub unplaced: Vec<Unplaced>,
    pub stats: Statistics,
    pub candidates: usize,
    pub cancelled: bool,
}

pub fn orientation_allowed(part: &Part, stock: &Stock, rotated: bool) -> bool {
    (!rotated || part.rotate)
        && (part.grain == Grain::None
            || (if rotated {
                part.grain.turned()
            } else {
                part.grain
            }) == stock.grain)
}

pub fn part_size(part: &Part, rotated: bool) -> (Length, Length) {
    if rotated {
        (part.height, part.width)
    } else {
        (part.width, part.height)
    }
}
