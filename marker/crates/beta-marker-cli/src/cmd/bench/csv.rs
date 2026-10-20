//! Small CSV reader/writer for the runner's quoted evidence files.
use anyhow::{Result, ensure};

pub(crate) fn records(input: &str) -> Result<Vec<Vec<String>>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut chars = input.trim_start_matches('\u{feff}').chars().peekable();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => record.push(std::mem::take(&mut field)),
            '\n' if !quoted => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            '\r' if !quoted => {}
            _ => field.push(c),
        }
    }
    ensure!(!quoted, "unterminated CSV quote");
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    Ok(records)
}

pub(crate) fn encode(record: &[String]) -> String {
    record
        .iter()
        .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quoted_fields_and_embedded_newlines_round_trip() -> Result<()> {
        let row = vec!["a,\"b\"".into(), "line\nline".into()];
        assert_eq!(records(&(encode(&row) + "\n"))?, vec![row]);
        assert!(records("\"unfinished").is_err());
        Ok(())
    }
}
