//! Bounded navigation over the existing context projection, never a second authority reader.
use std::fmt::Write;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use super::{context, FlowError, FlowResult};

const PREVIEW_CHARS: usize = 256;
const ITEM_BYTES: usize = 32 * 1024;

#[derive(Debug, Serialize)]
pub struct Item {
    pub key: String,
    pub section: String,
    pub kind: String,
    pub unit: String,
    pub total_items: usize,
    pub value: Value,
    pub preview: bool,
    pub omitted_items: usize,
}

#[derive(Debug, Serialize)]
pub struct Page {
    pub offset: usize,
    pub limit: usize,
    pub returned_items: usize,
    pub omitted_items: usize,
    pub next_offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct Section {
    pub scope: String,
    pub projection_identity: Value,
    pub section: String,
    pub found: bool,
    pub kind: String,
    pub unit: String,
    pub total_items: usize,
    pub page: Page,
    pub items: Vec<Item>,
    pub omissions: Vec<String>,
}

pub fn context_section(
    root: &Path,
    target: &str,
    section: &str,
    notes: usize,
    offset: usize,
    limit: usize,
) -> FlowResult<Section> {
    let native = context::context_with(root, target, notes)?;
    let mut result = project_section(
        &serde_json::to_value(native)?,
        target,
        section,
        offset,
        limit,
    )?;
    result.omissions.push(format!("Native context note window is {notes}; use --notes to change it. Section totals describe this context projection, not every historical record."));
    Ok(result)
}

/// Project one dot-path from a native serialized context. Positions locate this snapshot;
/// durable decisions must use the CIDs found inside it, never the array position.
pub fn project_section(
    native: &Value,
    scope: &str,
    section: &str,
    offset: usize,
    limit: usize,
) -> FlowResult<Section> {
    if !(1..=100).contains(&limit) {
        return Err(FlowError::InvalidArguments(
            "--limit must be between 1 and 100 entries or characters".into(),
        ));
    }
    let section = section.trim_matches('.');
    let mut selected = Some(native);
    if !section.is_empty() {
        for key in section.split('.') {
            selected = selected.and_then(|v| match v {
                Value::Object(map) => map.get(key),
                Value::Array(array) => key.parse::<usize>().ok().and_then(|i| array.get(i)),
                _ => None,
            });
        }
    }
    let section = if section.is_empty() { "." } else { section };
    let mut result = Section {
        scope: scope.into(),
        projection_identity: serde_json::json!({
            "context_cid": eprfs_core::BlobCid::compute_raw(&serde_json::to_vec(native)?).as_cid().to_string(),
            "scope_cid": native.pointer("/identity/cid"),
            "record_set_fingerprint": native.pointer("/reconciliation/record_set_fingerprint"),
            "evaluator": native.pointer("/reconciliation/evaluator"),
        }), section: section.into(), found: selected.is_some(),
        kind: selected.map(kind).unwrap_or("missing").into(),
        unit: selected.map(unit).unwrap_or("entries").into(),
        total_items: selected.map(total).unwrap_or(0),
        page: Page {offset,limit,returned_items: 0,omitted_items:0,next_offset:None},
        items: Vec::new(), omissions: vec![
            "Child collections are summaries; object values show the first four immediate scalar fields in object key order, each string capped at 64 Unicode characters with … marking truncation. Follow their exact section paths to inspect complete evidence. String previews are limited to 256 Unicode characters; direct string pages use character offsets.".into(),
            "Pages re-read native context; array positions are navigation only, not stable assertion identities. Reconcile CIDs after source or record changes.".into(),
            "Output paging does not bound the native graph computation. A 32KiB item budget may return fewer entries than requested, with an explicit next offset.".into(),
        ],
    };
    let Some(value) = selected else {
        result.omissions.push("Requested section does not exist in the current native context; no empty-history conclusion is warranted.".into());
        return Ok(result);
    };
    let mut used = 0;
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter().skip(offset).take(limit) {
                if !push_item(&mut result, child_item(section, key, child), &mut used)? {
                    break;
                }
            }
            result.page.returned_items = result.items.len();
        }
        Value::Array(array) => {
            for (i, child) in array.iter().enumerate().skip(offset).take(limit) {
                if !push_item(
                    &mut result,
                    child_item(section, &i.to_string(), child),
                    &mut used,
                )? {
                    break;
                }
            }
            result.page.returned_items = result.items.len();
        }
        Value::String(text) => {
            let chunk: String = text.chars().skip(offset).take(limit).collect();
            result.page.returned_items = chunk.chars().count();
            let returned = result.page.returned_items;
            result.items.push(Item {
                key: "value".into(),
                section: section.into(),
                kind: "string".into(),
                unit: "characters".into(),
                total_items: result.total_items,
                value: Value::String(chunk),
                preview: returned < result.total_items,
                omitted_items: result.total_items.saturating_sub(returned),
            });
        }
        _ if offset == 0 => {
            let mut item = child_item(".", "value", value);
            item.section = section.into();
            result.items.push(item);
            result.page.returned_items = 1;
        }
        _ => {}
    }
    let next = offset.saturating_add(result.page.returned_items);
    result.page.omitted_items = result
        .total_items
        .saturating_sub(result.page.returned_items);
    result.page.next_offset = (next < result.total_items).then_some(next);
    Ok(result)
}

fn push_item(result: &mut Section, item: Item, used: &mut usize) -> FlowResult<bool> {
    let bytes = serde_json::to_vec(&item)?.len();
    if *used + bytes > ITEM_BYTES {
        if result.items.is_empty() {
            return Err(FlowError::InvalidArguments(
                "section child locator exceeds output budget; narrow the section path".into(),
            ));
        }
        return Ok(false);
    }
    *used += bytes;
    result.items.push(item);
    Ok(true)
}

fn child_item(parent: &str, key: &str, value: &Value) -> Item {
    let section = if parent == "." {
        key.into()
    } else {
        format!("{parent}.{key}")
    };
    let (shown, omitted) = match value {
        Value::Array(_) => (Value::Null, total(value)),
        Value::Object(map) => {
            let mut preview = serde_json::Map::new();
            for (key, child) in map
                .iter()
                .filter(|(_, v)| !v.is_array() && !v.is_object())
                .take(4)
            {
                let shown = match child {
                    Value::String(text) => {
                        let cap = PREVIEW_CHARS / 4;
                        let shown = if text.chars().count() > cap {
                            format!("{}…", text.chars().take(cap - 1).collect::<String>())
                        } else {
                            text.clone()
                        };
                        Value::String(shown)
                    }
                    _ => child.clone(),
                };
                preview.insert(key.clone(), shown);
            }
            let omitted = map.len().saturating_sub(preview.len());
            (Value::Object(preview), omitted)
        }
        Value::String(text) => (
            Value::String(text.chars().take(PREVIEW_CHARS).collect()),
            total(value).saturating_sub(PREVIEW_CHARS),
        ),
        _ => (value.clone(), 0),
    };
    Item {
        key: key.into(),
        section,
        kind: kind(value).into(),
        unit: unit(value).into(),
        total_items: total(value),
        value: shown,
        preview: matches!(value, Value::Array(_) | Value::Object(_)) || omitted > 0,
        omitted_items: omitted,
    }
}
fn kind(value: &Value) -> &'static str {
    match value {
        Value::Object(_) => "object",
        Value::Array(_) => "array",
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Null => "null",
    }
}
fn unit(value: &Value) -> &'static str {
    if value.is_string() {
        "characters"
    } else {
        "entries"
    }
}
fn total(value: &Value) -> usize {
    match value {
        Value::Object(v) => v.len(),
        Value::Array(v) => v.len(),
        Value::String(v) => v.chars().count(),
        _ => 1,
    }
}
impl Section {
    pub fn render_text(&self) -> String {
        let mut text = format!("epr flow context — {} section {} [{}]\n{} {} total; {} shown; {} omitted; next offset {:?}\n",self.scope,self.section,self.kind,self.total_items,self.unit,self.page.returned_items,self.page.omitted_items,self.page.next_offset);
        for item in &self.items {
            let _ = writeln!(
                text,
                "{} [{}; {} {}; {} omitted] {} → --section {}",
                item.key,
                item.kind,
                item.total_items,
                item.unit,
                item.omitted_items,
                item.value,
                item.section
            );
        }
        for omission in &self.omissions {
            let _ = writeln!(text, "Limit: {omission}");
        }
        text
    }
}
