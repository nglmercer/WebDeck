use super::error::{AdminError, ErrorInfo};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Human,
    Json,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub ok: bool,
    pub data: Value,
    pub error: Option<ErrorInfo>,
    pub human: String,
    pub exit: i32,
}

impl Outcome {
    pub fn ok(data: Value, human: impl Into<String>) -> Self {
        Self {
            ok: true,
            data,
            error: None,
            human: human.into(),
            exit: 0,
        }
    }
    pub fn failed(data: Value, error: ErrorInfo, exit: i32, human: impl Into<String>) -> Self {
        Self {
            ok: false,
            data,
            error: Some(error),
            human: human.into(),
            exit,
        }
    }
    pub fn with_exit(mut self, exit: i32) -> Self {
        self.exit = exit;
        self
    }
    pub fn with_error(mut self, error: ErrorInfo) -> Self {
        self.ok = false;
        self.error = Some(error);
        self
    }
}

#[derive(Serialize)]
struct SuccessEnvelope<'a> {
    ok: bool,
    data: &'a Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a ErrorInfo>,
}

#[derive(Serialize)]
struct FailureEnvelope<'a> {
    ok: bool,
    error: &'a ErrorInfo,
}

pub struct Emitter {
    mode: OutputMode,
}

impl Emitter {
    pub fn new(mode: OutputMode) -> Self {
        Self { mode }
    }
    pub fn mode(&self) -> OutputMode {
        self.mode
    }
    pub fn is_json(&self) -> bool {
        self.mode == OutputMode::Json
    }
    pub fn emit(&self, outcome: &Outcome) -> i32 {
        match self.mode {
            OutputMode::Json => {
                let envelope = SuccessEnvelope {
                    ok: outcome.ok,
                    data: &outcome.data,
                    error: outcome.error.as_ref(),
                };
                match serde_json::to_string(&envelope) {
                    Ok(text) => println!("{text}"),
                    Err(e) => {
                        eprintln!("webdeckctl: failed to encode output: {e}");
                        return super::error::ExitCode::Failure as i32;
                    }
                }
            }
            OutputMode::Human => {
                if outcome.human.is_empty() {
                    match serde_json::to_string_pretty(&outcome.data) {
                        Ok(text) => println!("{text}"),
                        Err(e) => eprintln!("webdeckctl: failed to encode output: {e}"),
                    }
                } else {
                    println!("{}", outcome.human);
                }
                if let Some(error) = &outcome.error {
                    eprintln!("webdeckctl: {}: {}", error.code, error.message);
                }
            }
        }
        outcome.exit
    }
    pub fn emit_error(&self, error: &AdminError) -> i32 {
        let info = error.info();
        let exit = error.kind().exit() as i32;
        match self.mode {
            OutputMode::Json => {
                let envelope = FailureEnvelope {
                    ok: false,
                    error: &info,
                };
                match serde_json::to_string(&envelope) {
                    Ok(text) => println!("{text}"),
                    Err(e) => eprintln!("webdeckctl: failed to encode error: {e}"),
                }
            }
            OutputMode::Human => {
                eprintln!("webdeckctl: error [{}]: {}", info.code, info.message);
                if let Some(details) = &info.details {
                    if let Ok(text) = serde_json::to_string_pretty(details) {
                        eprintln!("{text}");
                    }
                }
            }
        }
        exit
    }
}

/// Render data with optional catalog presentation; JSON output always keeps the raw result.
pub fn render_command(
    command: &crate::contracts::Command,
    result: &Value,
    catalog: Option<&crate::contracts::CatalogResponse>,
) -> String {
    let fallback_label = catalog.and_then(|catalog| {
        if let crate::contracts::Command::Plugin {
            plugin_id,
            action_id,
            ..
        } = command
        {
            catalog
                .plugins
                .iter()
                .find(|p| &p.id == plugin_id)
                .and_then(|p| p.actions.iter().find(|a| &a.id == action_id))
                .map(|a| a.label.as_str())
        } else {
            let value = serde_json::to_value(command).ok()?;
            catalog
                .commands
                .iter()
                .find(|c| value["type"] == c.id)
                .and_then(|c| c.label.as_deref())
        }
    });
    let metadata = catalog.and_then(|c| c.automation.as_ref());
    let command = serde_json::to_value(command).expect("serializable command");
    let presentation = metadata.and_then(|m| {
        m.presentations
            .iter()
            .filter(|p| crate::automation::matches(&p.selector, &command))
            .max_by_key(|p| p.selector.len())
    });
    let result = presentation
        .and_then(|p| p.result_pointer.as_ref())
        .and_then(|pointer| result.pointer(pointer))
        .unwrap_or(result);
    if let Some(view) = presentation.and_then(|p| p.result_view.as_ref()) {
        if let Some(items) = result
            .pointer(&view.items_pointer)
            .and_then(Value::as_array)
        {
            if !view.columns.is_empty()
                && items
                    .iter()
                    .all(|item| view.columns.values().all(|p| item.pointer(p).is_some()))
            {
                let mut lines = vec![view.columns.keys().cloned().collect::<Vec<_>>().join("\t")];
                lines.extend(items.iter().map(|item| {
                    view.columns
                        .values()
                        .map(|p| display_value(item.pointer(p).expect("checked pointer")))
                        .collect::<Vec<_>>()
                        .join("\t")
                }));
                return lines.join("\n");
            }
        }
    }
    if result.is_null() || result.as_object().is_some_and(|o| o.is_empty()) {
        if let Some(presentation) = presentation {
            let args = presentation
                .arguments
                .iter()
                .filter_map(|(label, pointer)| {
                    command
                        .pointer(pointer)
                        .map(|v| format!("{label}: {}", display_value(v)))
                })
                .collect::<Vec<_>>();
            return if args.is_empty() {
                format!("{} completed.", presentation.label)
            } else {
                format!("{} ({}).", presentation.label, args.join(", "))
            };
        }
        if let Some(label) = fallback_label {
            return format!("{label} completed.");
        }
        return format!(
            "Action '{}' completed.",
            command["type"].as_str().unwrap_or("unknown")
        );
    }
    serde_json::to_string_pretty(result).unwrap_or_else(|_| "Action completed.".into())
}
fn display_value(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}
