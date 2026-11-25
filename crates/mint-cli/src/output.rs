use serde::Serialize;
use serde_json::{Value, json};
use std::io::{self, Write};

pub fn emit<T: Serialize>(kind: &str, data: &T, machine: bool) -> Result<(), String> {
    let data = serde_json::to_value(data).map_err(|_| "could not encode output")?;
    let text = if machine {
        serde_json::to_string(&json!({"schema_version":1,"kind":kind,"data":data}))
            .map_err(|_| "could not encode output")?
    } else {
        human(&data)
    };
    match writeln!(io::stdout().lock(), "{text}") {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(_) => Err("could not write output".into()),
    }
}
fn human(data: &Value) -> String {
    match data {
        Value::Object(fields) => fields
            .iter()
            .map(|(k, v)| {
                let value = match v {
                    Value::Null => "unknown".into(),
                    Value::String(s) => s.escape_debug().to_string(),
                    _ => v.to_string(),
                };
                format!("{}: {value}", k.replace('_', " "))
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => data.to_string(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn server_strings_cannot_control_terminal() {
        assert!(!human(&json!({"status":"\u{1b}[2J\nforged"})).contains('\u{1b}'));
    }
}
