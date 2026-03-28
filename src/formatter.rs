use serde_json::Value;
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum OutputFormat {
    #[default]
    Json,
    Table,
    Yaml,
    Csv,
    Ids,
    Slack,
}

impl OutputFormat {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "table" => Ok(Self::Table),
            "yaml" | "yml" => Ok(Self::Yaml),
            "csv" => Ok(Self::Csv),
            "ids" => Ok(Self::Ids),
            "slack" => Ok(Self::Slack),
            other => Err(other.to_string()),
        }
    }

    pub fn from_str(s: &str) -> Self {
        Self::parse(s).unwrap_or(Self::Json)
    }
}

pub fn format_value(value: &Value, format: &OutputFormat) -> String {
    match format {
        OutputFormat::Json => serde_json::to_string_pretty(value).unwrap_or_default(),
        OutputFormat::Table => format_table(value),
        OutputFormat::Yaml => format_yaml(value),
        OutputFormat::Csv => format_csv(value),
        OutputFormat::Ids => format_ids(value),
        // Slack falls back to JSON in the generic formatter;
        // commands that support Slack handle it before calling format_value.
        OutputFormat::Slack => serde_json::to_string_pretty(value).unwrap_or_default(),
    }
}

fn extract_items(value: &Value) -> Option<(&str, &Vec<Value>)> {
    if let Value::Object(obj) = value {
        for (key, val) in obj {
            if key == "nextPageToken" || key == "cursor" || key == "total" || key.starts_with('_') {
                continue;
            }
            if let Value::Array(arr) = val {
                if !arr.is_empty() {
                    return Some((key, arr));
                }
            }
        }
    }
    None
}

fn format_table(value: &Value) -> String {
    let items = extract_items(value);

    if let Some((_key, arr)) = items {
        format_array_as_table(arr)
    } else if let Value::Array(arr) = value {
        format_array_as_table(arr)
    } else if let Value::Object(obj) = value {
        let mut output = String::new();
        let max_key_len = obj.keys().map(|k| k.len()).max().unwrap_or(0);
        for (key, val) in obj {
            let _ = writeln!(
                output,
                "{:width$}  {}",
                key,
                value_to_cell(val),
                width = max_key_len
            );
        }
        output
    } else {
        value.to_string()
    }
}

fn format_array_as_table(arr: &[Value]) -> String {
    if arr.is_empty() {
        return "(empty)\n".to_string();
    }

    let mut columns: Vec<String> = Vec::new();
    for item in arr {
        if let Value::Object(obj) = item {
            for key in obj.keys() {
                if !columns.contains(key) {
                    columns.push(key.clone());
                }
            }
        }
    }

    if columns.is_empty() {
        let mut output = String::new();
        for item in arr {
            let _ = writeln!(output, "{}", value_to_cell(item));
        }
        return output;
    }

    let mut widths: Vec<usize> = columns.iter().map(|c| c.len()).collect();
    let rows: Vec<Vec<String>> = arr
        .iter()
        .map(|item| {
            columns
                .iter()
                .enumerate()
                .map(|(i, col)| {
                    let cell = if let Value::Object(obj) = item {
                        value_to_cell(obj.get(col).unwrap_or(&Value::Null))
                    } else {
                        String::new()
                    };
                    if cell.len() > widths[i] {
                        widths[i] = cell.len();
                    }
                    if widths[i] > 60 {
                        widths[i] = 60;
                    }
                    cell
                })
                .collect()
        })
        .collect();

    let mut output = String::new();

    // Header
    let header: Vec<String> = columns
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{:width$}", c, width = widths[i]))
        .collect();
    let _ = writeln!(output, "{}", header.join("  "));

    // Separator
    let sep: Vec<String> = widths.iter().map(|w| "─".repeat(*w)).collect();
    let _ = writeln!(output, "{}", sep.join("  "));

    // Rows
    for row in &rows {
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let truncated = if c.len() > widths[i] {
                    format!("{}…", &c[..widths[i] - 1])
                } else {
                    c.clone()
                };
                format!("{:width$}", truncated, width = widths[i])
            })
            .collect();
        let _ = writeln!(output, "{}", cells.join("  "));
    }

    output
}

fn format_yaml(value: &Value) -> String {
    json_to_yaml(value, 0)
}

fn json_to_yaml(value: &Value, indent: usize) -> String {
    let prefix = "  ".repeat(indent);
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            if s.contains('\n') {
                format!(
                    "|\n{}",
                    s.lines()
                        .map(|l| format!("{prefix}  {l}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            } else {
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                format!("\"{escaped}\"")
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                return "[]".to_string();
            }
            let mut out = String::new();
            for item in arr {
                let val_str = json_to_yaml(item, indent + 1);
                let _ = write!(out, "\n{prefix}- {val_str}");
            }
            out
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                return "{}".to_string();
            }
            let mut out = String::new();
            for (key, val) in obj {
                match val {
                    Value::Object(_) | Value::Array(_) => {
                        let val_str = json_to_yaml(val, indent + 1);
                        let _ = write!(out, "\n{prefix}{key}:{val_str}");
                    }
                    _ => {
                        let val_str = json_to_yaml(val, indent);
                        let _ = write!(out, "\n{prefix}{key}: {val_str}");
                    }
                }
            }
            out
        }
    }
}

fn format_ids(value: &Value) -> String {
    match extract_items(value) {
        Some((_key, arr)) => arr
            .iter()
            .filter_map(|item| {
                item.get("id").and_then(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
            })
            .collect::<Vec<_>>()
            .join("\n"),
        None => String::new(),
    }
}

fn format_csv(value: &Value) -> String {
    let items = extract_items(value);

    let arr = if let Some((_key, arr)) = items {
        arr.as_slice()
    } else if let Value::Array(arr) = value {
        arr.as_slice()
    } else {
        return value_to_cell(value);
    };

    if arr.is_empty() {
        return String::new();
    }

    let mut columns: Vec<String> = Vec::new();
    for item in arr {
        if let Value::Object(obj) = item {
            for key in obj.keys() {
                if !columns.contains(key) {
                    columns.push(key.clone());
                }
            }
        }
    }

    let mut output = String::new();
    let _ = writeln!(output, "{}", columns.join(","));

    for item in arr {
        let cells: Vec<String> = columns
            .iter()
            .map(|col| {
                if let Value::Object(obj) = item {
                    csv_escape(&value_to_cell(obj.get(col).unwrap_or(&Value::Null)))
                } else {
                    String::new()
                }
            })
            .collect();
        let _ = writeln!(output, "{}", cells.join(","));
    }

    output
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn value_to_cell(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(value_to_cell).collect();
            items.join(", ")
        }
        Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn output_format_from_str() {
        assert_eq!(OutputFormat::from_str("json"), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("table"), OutputFormat::Table);
        assert_eq!(OutputFormat::from_str("yaml"), OutputFormat::Yaml);
        assert_eq!(OutputFormat::from_str("csv"), OutputFormat::Csv);
        assert_eq!(OutputFormat::from_str("slack"), OutputFormat::Slack);
        assert_eq!(OutputFormat::from_str("unknown"), OutputFormat::Json);
    }

    #[test]
    fn format_json_output() {
        let val = json!({"name": "test"});
        let output = format_value(&val, &OutputFormat::Json);
        assert!(output.contains("\"name\""));
    }

    #[test]
    fn format_table_array() {
        let val = json!({
            "items": [
                {"id": "1", "name": "Alpha"},
                {"id": "2", "name": "Beta"}
            ]
        });
        let output = format_value(&val, &OutputFormat::Table);
        assert!(output.contains("id"));
        assert!(output.contains("Alpha"));
        assert!(output.contains("──"));
    }

    #[test]
    fn output_format_ids_variant() {
        assert_eq!(OutputFormat::from_str("ids"), OutputFormat::Ids);
        assert_eq!(OutputFormat::parse("ids"), Ok(OutputFormat::Ids));
    }

    #[test]
    fn format_ids_output() {
        let val = json!({
            "items": [
                {"id": "item-1", "name": "A"},
                {"id": "item-2", "name": "B"}
            ]
        });
        let output = format_value(&val, &OutputFormat::Ids);
        assert_eq!(output, "item-1\nitem-2");
    }

    #[test]
    fn format_ids_output_numeric() {
        let val = json!({
            "items": [
                {"id": 53, "name": "A"},
                {"id": 192, "name": "B"}
            ]
        });
        let output = format_value(&val, &OutputFormat::Ids);
        assert_eq!(output, "53\n192");
    }

    #[test]
    fn format_ids_empty_on_single_object() {
        let val = json!({"id": "item-1", "name": "A"});
        let output = format_value(&val, &OutputFormat::Ids);
        assert_eq!(output, "");
    }

    #[test]
    fn format_csv_output() {
        let val = json!({
            "items": [
                {"id": "1", "name": "Alpha"},
                {"id": "2", "name": "Beta"}
            ]
        });
        let output = format_value(&val, &OutputFormat::Csv);
        assert!(output.contains("id,name"));
        assert!(output.contains("1,Alpha"));
    }
}
