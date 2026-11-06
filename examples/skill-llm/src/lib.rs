//! Shared by the example binaries: choose a stub or a live Ollama client.
//!
//! The stub is the default and needs no network. A live Ollama is used only when
//! `OLLAMA_BASE_URL` is set, or `RUN_OLLAMA_SMOKE=1` (then `http://localhost:11434`).
//! `OLLAMA_MODEL` names the model (default `llama3.1`).

use std::collections::VecDeque;
use std::sync::Mutex;

use serde_json::Value;
use turul_llm_core::{CompletionRequest, CompletionResponse, LlmClient, LlmError};
use turul_llm_ollama::OllamaClient;

/// Replays canned structured responses in order, one per call.
pub struct Stub(Mutex<VecDeque<Value>>);

impl Stub {
    pub fn new(responses: impl IntoIterator<Item = Value>) -> Self {
        Stub(Mutex::new(responses.into_iter().collect()))
    }
}

#[async_trait::async_trait]
impl LlmClient for Stub {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let next = self.0.lock().unwrap().pop_front();
        Ok(CompletionResponse::new(
            next.expect("stub has a response for every call"),
        ))
    }
}

fn ollama_base_url() -> Option<String> {
    if let Ok(v) = std::env::var("OLLAMA_BASE_URL")
        && !v.trim().is_empty()
    {
        return Some(v.trim().trim_end_matches('/').to_string());
    }
    if std::env::var("RUN_OLLAMA_SMOKE").ok().as_deref() == Some("1") {
        return Some("http://localhost:11434".to_string());
    }
    None
}

/// The mode label and client: live Ollama when opted in, otherwise `stub`.
pub fn client(stub: Stub) -> (&'static str, Box<dyn LlmClient>) {
    match ollama_base_url() {
        Some(base) => {
            let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.1".to_string());
            ("live-ollama", Box::new(OllamaClient::new(base, model)))
        }
        None => ("offline-stub", Box::new(stub)),
    }
}
