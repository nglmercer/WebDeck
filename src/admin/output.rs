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
