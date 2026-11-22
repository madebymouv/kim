//! Step 107 (ADR 0071, docs/INTEROP.md): Jarak as a provider of Kim interop capabilities.
//!
//! `jarak interop invoke --json --no-input` reads one `kim.interop.request` from stdin and runs
//! one capability through the same code as the matching command, so the answer is that command's
//! `--json` data:
//!
//! | Capability | Interface | Effect | Command |
//! | --- | --- | --- | --- |
//! | `document.info` | 1.0.0 | read | `jarak info <path>` |
//! | `text.find` | 1.0.0 | read | `jarak find <path> <text>` |
//! | `document.export` | 1.0.0 | write | `jarak export <path> --output <output>` |
//! | `markdown.insert-image` | 1.0.0 | write | `jarak assets add <document> <image>` |
//! | `document.open` | 1.0.0 | write, opens a window | `jarak open <path>[:line[:column]]` |
//!
//! The caller (Kim) enforces consent: `write` only with `--allow-write`, a window only with
//! `--allow-window`; Jarak still applies its own rules (never replacing a file without
//! `overwrite`, never the document itself, images only from local files).

use serde_json::Value;

/// (id, interface version, effect, opens a window, summary) — also in `capabilities.json` under
/// `provides` (a test keeps them equal).
pub const PROVIDES: [(&str, &str, &str, bool, &str); 5] = [
    (
        "document.info",
        "1.0.0",
        "read",
        false,
        "A text file's encoding, byte-order mark, line endings, lines, language and whether Jarak opens it read-only (params: path)",
    ),
    (
        "text.find",
        "1.0.0",
        "read",
        false,
        "Find text in a file or a folder (params: path, text, regex, match_case, whole_word); every hit with its line and column",
    ),
    (
        "document.export",
        "1.0.0",
        "write",
        false,
        "Export a document as HTML, RTF, a published folder or PDF (params: path, output, format, overwrite); nothing remote is fetched and the document is never changed",
    ),
    (
        "markdown.insert-image",
        "1.0.0",
        "write",
        false,
        "Copy a local image into a Markdown document's assets folder (never replacing a file) and insert its reference at a line or at the end (params: document, image, line, alt)",
    ),
    (
        "document.open",
        "1.0.0",
        "write",
        true,
        "Open a file in Jarak's window at a line and column (params: path, line, column); the running window takes it",
    ),
];

/// Why a request is refused: (code, exit code, message).
pub type Refusal = (&'static str, u8, String);

/// Checks a request; returns (capability, params, dry_run).
pub fn parse(text: &str) -> Result<(String, Value, bool), Refusal> {
    if text.len() > 64 * 1024 {
        return Err(("usage", 2, "the request is larger than 64 KiB".into()));
    }
    let req: Value = serde_json::from_str(text)
        .map_err(|e| ("usage", 2, format!("the request on stdin is not JSON: {e}")))?;
    if req["schema"] != "kim.interop.request" || req["schema_version"] != 1 {
        return Err((
            "tool_incompatible",
            5,
            "not a kim.interop.request version 1".into(),
        ));
    }
    let cap = req["capability"].as_str().unwrap_or_default().to_string();
    let Some((_, version, ..)) = PROVIDES.iter().find(|(id, ..)| *id == cap) else {
        return Err((
            "capability_missing",
            5,
            format!(
                "Jarak does not provide `{cap}` (`jarak capabilities --json` lists what it does)"
            ),
        ));
    };
    let major = version.split('.').next().unwrap_or("1");
    if req["interface"].as_str() != Some(major) {
        return Err((
            "tool_incompatible",
            5,
            format!(
                "Jarak speaks `{cap}` interface {major}, not {}",
                req["interface"].as_str().unwrap_or("?")
            ),
        ));
    }
    let params = match &req["params"] {
        Value::Null => Value::Object(serde_json::Map::new()),
        p @ Value::Object(_) => p.clone(),
        _ => return Err(("usage", 2, "`params` must be an object".into())),
    };
    Ok((cap, params, req["dry_run"].as_bool().unwrap_or(false)))
}

/// A string parameter.
pub fn text<'a>(params: &'a Value, key: &str) -> Result<&'a str, Refusal> {
    params[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            (
                "usage",
                2,
                format!("the parameter `{key}` (text) is missing"),
            )
        })
}

/// An optional boolean parameter.
pub fn flag(params: &Value, key: &str) -> bool {
    params[key].as_bool().unwrap_or(false)
}

/// An optional positive whole-number parameter.
pub fn number(params: &Value, key: &str) -> Result<Option<u64>, Refusal> {
    match &params[key] {
        Value::Null => Ok(None),
        v => v
            .as_u64()
            .filter(|n| *n > 0)
            .map(Some)
            .ok_or_else(|| ("usage", 2, format!("`{key}` must be a whole number from 1"))),
    }
}
