use serde::{Deserialize, Serialize};

/// All geometry uses integer nanometres. Input accepts up to four decimals.
pub type Length = i64;
pub const MM: Length = 1_000_000;
pub const MAX_LENGTH: Length = 100_000 * MM;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unit {
    #[default]
    Mm,
    Cm,
    Inch,
}

impl Unit {
    pub const ALL: [Self; 3] = [Self::Mm, Self::Cm, Self::Inch];

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Mm => "mm",
            Self::Cm => "cm",
            Self::Inch => "in",
        }
    }

    pub fn scale(self) -> Length {
        match self {
            Self::Mm => MM,
            Self::Cm => 10 * MM,
            Self::Inch => 25_400_000,
        }
    }

    /// Parse without floating point, rounding, exponents or implicit unit changes.
    pub fn parse(self, text: &str) -> Result<Length, &'static str> {
        self.parse_decimal(text, 4)
    }

    /// CSV exports use exact millimetres with up to six decimals, independent of display units.
    pub fn parse_csv(self, text: &str) -> Result<Length, &'static str> {
        self.parse_decimal(text, 6).map_err(|key| {
            if key == "precision_limit" {
                "csv_precision"
            } else {
                key
            }
        })
    }

    fn parse_decimal(self, text: &str, precision: usize) -> Result<Length, &'static str> {
        let text = text.trim();
        if text.is_empty() {
            return Err("number_required");
        }
        let normalized = text.replace(',', ".");
        let mut parts = normalized.split('.');
        let whole = parts.next().unwrap_or("");
        let decimals = parts.next().unwrap_or("");
        if parts.next().is_some()
            || (whole.is_empty() && decimals.is_empty())
            || !whole.bytes().all(|c| c.is_ascii_digit())
            || !decimals.bytes().all(|c| c.is_ascii_digit())
        {
            return Err("invalid_number");
        }
        if decimals.len() > precision {
            return Err("precision_limit");
        }
        let denominator = 10_i64.pow(decimals.len() as u32);
        let whole: Length = if whole.is_empty() {
            0
        } else {
            whole.parse().map_err(|_| "size_limit")?
        };
        let fraction: Length = if decimals.is_empty() {
            0
        } else {
            decimals.parse().map_err(|_| "invalid_number")?
        };
        let scaled_fraction = fraction * self.scale();
        if scaled_fraction % denominator != 0 {
            return Err("precision_limit");
        }
        let result = whole
            .checked_mul(self.scale())
            .and_then(|n| n.checked_add(scaled_fraction / denominator))
            .ok_or("size_limit")?;
        if result > MAX_LENGTH {
            Err("size_limit")
        } else {
            Ok(result)
        }
    }

    /// Display converted geometry to six decimals; input geometry is never changed.
    pub fn format(self, value: Length) -> String {
        let scale = self.scale() as i128;
        let rounded = (i128::from(value) * 1_000_000 + scale / 2) / scale;
        let mut result = format!("{}.{:06}", rounded / 1_000_000, rounded % 1_000_000);
        while result.ends_with('0') {
            result.pop();
        }
        if result.ends_with('.') {
            result.pop();
        }
        result
    }

    /// A compact label; the report also includes exact native dimensions.
    pub fn label(self, value: Length) -> String {
        format!("{} {}", self.format(value), self.symbol())
    }
}
