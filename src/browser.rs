use std::io::{self, Read};

fn main() {
    let response = match execute() {
        Ok(response) => response,
        Err(error) => serde_json::json!({
            "ok": false,
            "error": slate_c::diagnostics::render(&error),
            "diagnostic": slate_c::diagnostics::to_json(&error),
        }),
    };
    println!("{response}");
}

fn execute() -> slate_c::diagnostics::Result<serde_json::Value> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let request: serde_json::Value = serde_json::from_str(&input)?;
    if request.get("operation").and_then(|value| value.as_str()) == Some("schema") {
        let target = request
            .get("target")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                slate_c::diagnostics::CompilerError::Configuration("missing schema target".into())
            })?;
        return Ok(serde_json::json!({"ok": true, "schema": slate_c::cross::flag_schema(target)?}));
    }
    let compilation = slate_c::cross::compile(&serde_json::from_value(request)?)?;
    Ok(serde_json::json!({"ok": true, "compilation": compilation}))
}
