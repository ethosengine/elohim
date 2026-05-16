/// Helpers for extracting typed values from CozoDB `DataValue` result rows.
///
/// These are the only conversions needed across lamad + shefa view builders —
/// lift them here rather than duplicating in each builder file.
use cozo::DataValue;

/// Extract a String from the given column index, returning an empty String on mismatch.
pub fn str_at(row: &[DataValue], i: usize) -> String {
    match row.get(i) {
        Some(DataValue::Str(s)) => s.to_string(),
        _ => String::new(),
    }
}

/// Extract an optional String from the given column index.
/// Returns None if the value is Null or any non-string type.
pub fn opt_str_at(row: &[DataValue], i: usize) -> Option<String> {
    match row.get(i) {
        Some(DataValue::Str(s)) => Some(s.to_string()),
        _ => None,
    }
}

/// Extract an i64 from the given column index, returning 0 on mismatch.
pub fn i64_at(row: &[DataValue], i: usize) -> i64 {
    match row.get(i) {
        Some(DataValue::Num(cozo::Num::Int(n))) => *n,
        _ => 0,
    }
}

/// Extract an f64 from the given column index, returning 0.0 on mismatch.
pub fn f64_at(row: &[DataValue], i: usize) -> f64 {
    match row.get(i) {
        Some(DataValue::Num(cozo::Num::Float(f))) => *f,
        Some(DataValue::Num(cozo::Num::Int(n))) => *n as f64,
        _ => 0.0,
    }
}

/// Extract a Vec<String> from a List column, skipping non-string items.
pub fn list_str_at(row: &[DataValue], i: usize) -> Vec<String> {
    match row.get(i) {
        Some(DataValue::List(items)) => items
            .iter()
            .filter_map(|v| match v {
                DataValue::Str(s) => Some(s.to_string()),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

/// Extract a Vec<f64> from a List column, skipping non-numeric items.
pub fn list_f64_at(row: &[DataValue], i: usize) -> Vec<f64> {
    match row.get(i) {
        Some(DataValue::List(items)) => items
            .iter()
            .filter_map(|v| match v {
                DataValue::Num(cozo::Num::Float(f)) => Some(*f),
                DataValue::Num(cozo::Num::Int(n)) => Some(*n as f64),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}
