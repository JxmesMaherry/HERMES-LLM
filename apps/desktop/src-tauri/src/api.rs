use crate::{
    config::{self, Config},
    error::{message, Result},
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct Api {
    pub client: reqwest::Client,
    pub base: String,
    token: String,
}
impl Api {
    #[cfg(test)]
    pub fn testing(base: String, token: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap(),
            base,
            token,
        }
    }

    pub fn new(client: reqwest::Client, config: &Config, token: String) -> Self {
        Self {
            client,
            base: config.base_url(),
            token,
        }
    }
    pub fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .get(format!("{}{path}", self.base))
            .bearer_auth(&self.token)
    }
    pub fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .post(format!("{}{path}", self.base))
            .bearer_auth(&self.token)
    }
    pub async fn json(request: reqwest::RequestBuilder) -> Result<Value> {
        let response = checked(request.send().await?).await?;
        Ok(response.json().await?)
    }
}
pub async fn checked(response: reqwest::Response) -> Result<reqwest::Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let text = match response.status().as_u16() {
        401 => "Open WebUI sign-in expired. Sign in again in Settings.",
        403 => "Open WebUI denied access. Check your account and knowledge-base permissions.",
        404 => "The local resource is unavailable. Refresh knowledge bases or verify the pinned Open WebUI version.",
        400 | 422 => "Open WebUI rejected the request. Check your inputs, model, account, and backend version.",
        429 => "Open WebUI is busy or rate-limited. Wait a moment and retry.",
        _ => "Open WebUI failed to process the request. Inspect backend logs locally.",
    };
    Err(message(text))
}
pub async fn authenticate(
    client: &reqwest::Client,
    config: &Config,
    email: &str,
    password: &str,
    create: bool,
) -> Result<String> {
    if !email.contains('@') || email.len() > 254 || password.is_empty() || password.len() > 1024 {
        return Err(message("Enter a valid email and password."));
    }
    let endpoint = if create { "signup" } else { "signin" };
    let response = Api::json(
        client
            .post(format!("{}/api/v1/auths/{endpoint}", config.base_url()))
            .json(&json!({"email":email,"password":password,"name":"HERMES user"})),
    )
    .await?;
    let token = response["token"]
        .as_str()
        .ok_or_else(|| message("Sign-in did not return a token. Check the local account."))?;
    Ok(token.to_owned())
}
pub async fn models(client: &reqwest::Client) -> Result<Value> {
    Api::json(client.get("http://127.0.0.1:11434/api/tags")).await
}
pub async fn pull(
    client: &reqwest::Client,
    model: &str,
    channel: &tauri::ipc::Channel<Value>,
) -> Result<()> {
    if !config::valid_model(model) {
        return Err(message("Invalid model name."));
    }
    let response = checked(
        client
            .post("http://127.0.0.1:11434/api/pull")
            .timeout(Duration::from_secs(3600))
            .json(&json!({"model":model,"stream":true}))
            .send()
            .await?,
    )
    .await?;
    let mut stream = response.bytes_stream();
    let mut decoder = PullDecoder::default();
    while let Some(chunk) = tokio::time::timeout(Duration::from_secs(180), stream.next())
        .await
        .map_err(|_| message("Model download stalled. Retry when the connection is available."))?
    {
        for value in decoder.push(&chunk?)? {
            let _ = channel.send(value);
        }
    }
    for value in decoder.finish()? {
        let _ = channel.send(value);
    }
    Ok(())
}

#[derive(Default)]
struct PullDecoder {
    pending: Vec<u8>,
    completed: bool,
}
impl PullDecoder {
    fn line(&mut self, line: &[u8]) -> Result<Option<Value>> {
        if line.iter().all(u8::is_ascii_whitespace) {
            return Ok(None);
        }
        let value: Value = serde_json::from_slice(line)?;
        if value.get("error").is_some_and(|v| !v.is_null()) {
            return Err(message(
                "Ollama could not download this model. Check its name, storage, and connection.",
            ));
        }
        self.completed |= value["status"] == "success";
        Ok(Some(value))
    }
    fn push(&mut self, bytes: &[u8]) -> Result<Vec<Value>> {
        self.pending.extend_from_slice(bytes);
        if self.pending.len() > 1024 * 1024 {
            return Err(message("Ollama sent an oversized progress event."));
        }
        let mut events = vec![];
        while let Some(pos) = self.pending.iter().position(|c| *c == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=pos).collect();
            if let Some(value) = self.line(&line)? {
                events.push(value);
            }
        }
        Ok(events)
    }
    fn finish(&mut self) -> Result<Vec<Value>> {
        let pending = std::mem::take(&mut self.pending);
        let events = self.line(&pending)?.into_iter().collect();
        if !self.completed {
            return Err(message("Model download ended before Ollama confirmed success. Retry the download; cached data is preserved."));
        }
        Ok(events)
    }
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Message {
    pub role: String,
    pub content: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub messages: Vec<Message>,
    pub model: String,
    pub updated: u64,
}
pub fn validate_history(chats: &[Conversation]) -> Result<()> {
    if chats.len() > 200
        || chats.iter().any(|c| {
            c.messages.len() > 1000
                || c.title.len() > 200
                || c.messages.iter().any(|m| {
                    !["user", "assistant"].contains(&m.role.as_str()) || m.content.len() > 2_000_000
                })
        })
    {
        return Err(message(
            "Conversation history exceeds the local storage limit.",
        ));
    }
    Ok(())
}
pub fn chat_body(model: &str, messages: &[Message], knowledge: Option<&str>) -> Value {
    let mut body = json!({"model":model,"messages":messages,"stream":true,"params":{"function_calling":"legacy"}});
    if let Some(id) = knowledge {
        body["files"] = json!([{"type":"collection", "id":id}]);
    }
    body
}
#[derive(Default)]
pub struct SseDecoder {
    pending: Vec<u8>,
    completed: bool,
    answer: bool,
    sources: bool,
}
impl SseDecoder {
    fn line(&mut self, line: &[u8]) -> Result<Option<Value>> {
        let text = std::str::from_utf8(line)
            .map_err(|_| message("The backend returned invalid stream text."))?;
        let Some(data) = text.trim().strip_prefix("data:") else {
            return Ok(None);
        };
        let data = data.trim();
        if data.is_empty() {
            return Ok(None);
        }
        if data == "[DONE]" {
            self.completed = true;
            return Ok(None);
        }
        let event: Value = serde_json::from_str(data)?;
        if event.get("error").is_some_and(|v| !v.is_null()) {
            return Err(message(
                "The model returned an error. Verify the selected model and inspect backend logs.",
            ));
        }
        self.completed |= event["done"] == true
            || event["choices"].as_array().is_some_and(|choices| {
                choices
                    .iter()
                    .any(|c| c["finish_reason"].as_str().is_some_and(|s| !s.is_empty()))
            });
        self.answer |= event["choices"][0]["delta"]["content"]
            .as_str()
            .is_some_and(|s| !s.trim().is_empty());
        self.sources |= event["sources"].as_array().is_some_and(|s| !s.is_empty());
        Ok(Some(event))
    }
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Value>> {
        self.pending.extend_from_slice(bytes);
        if self.pending.len() > 4 * 1024 * 1024 {
            return Err(message("Backend stream event exceeded the safety limit."));
        }
        let mut output = Vec::new();
        while let Some(pos) = self.pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=pos).collect();
            if let Some(event) = self.line(&line)? {
                output.push(event);
            }
        }
        Ok(output)
    }
    pub fn finish(&mut self, require_sources: bool) -> Result<Vec<Value>> {
        let pending = std::mem::take(&mut self.pending);
        let events = self.line(&pending)?.into_iter().collect();
        if !self.completed {
            return Err(message("The response was interrupted before completion. Any partial text is incomplete; retry when the backend is ready."));
        }
        if !self.answer {
            return Err(message(
                "The model completed without answer text. Check the selected model and retry.",
            ));
        }
        if require_sources && !self.sources {
            return Err(message("No retrieved sources were returned for this knowledge question. Treat any answer as unverified; check indexing and the selected knowledge base."));
        }
        Ok(events)
    }
}
pub async fn stream_chat(
    api: &Api,
    body: Value,
    cancel: CancellationToken,
    channel: tauri::ipc::Channel<Value>,
) -> Result<()> {
    let response = tokio::select! {
        _ = cancel.cancelled() => return Err(message("Response cancelled.")),
        r = api.post("/api/chat/completions").json(&body).timeout(Duration::from_secs(600)).send() => checked(r?).await?,
    };
    if !response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .contains("text/event-stream")
    {
        return Err(message(
            "Open WebUI did not return a chat stream. Check the model and backend version.",
        ));
    }
    let mut stream = response.bytes_stream();
    let mut decoder = SseDecoder::default();
    let require_sources = body["files"]
        .as_array()
        .is_some_and(|files| !files.is_empty());
    let mut total = 0usize;
    loop {
        let chunk = tokio::select! {
            _ = cancel.cancelled() => return Err(message("Response cancelled.")),
            next = tokio::time::timeout(Duration::from_secs(180), stream.next()) => next.map_err(|_| message("No response for 3 minutes. Check Ollama and retry."))?,
        };
        let Some(chunk) = chunk else {
            break;
        };
        let chunk = chunk?;
        total += chunk.len();
        if total > 8 * 1024 * 1024 {
            return Err(message("Response exceeded the safety limit."));
        }
        for event in decoder.push(&chunk)? {
            channel
                .send(event)
                .map_err(|_| message("Chat window closed."))?;
        }
    }
    for event in decoder.finish(require_sources)? {
        channel
            .send(event)
            .map_err(|_| message("Chat window closed."))?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{header, method, path},
        Mock, MockServer, ResponseTemplate,
    };
    #[test]
    fn decodes_split_utf8_and_errors() {
        let mut d = SseDecoder::default();
        let bytes = "data: {\"choices\":[{\"delta\":{\"content\":\"hé\"}}]}\n\n".as_bytes();
        let mut all = vec![];
        for byte in bytes {
            all.extend(d.push(&[*byte]).unwrap());
        }
        assert_eq!(all[0]["choices"][0]["delta"]["content"], "hé");
        assert!(d.push(b"data: {\"error\":\"secret\"}\n").is_err());
    }
    #[test]
    fn rejects_incomplete_or_empty_chat_and_accepts_final_unterminated_line() {
        let answer = b"data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n";
        let mut interrupted = SseDecoder::default();
        interrupted.push(answer).unwrap();
        assert!(interrupted
            .finish(false)
            .unwrap_err()
            .to_string()
            .contains("interrupted"));
        let mut empty = SseDecoder::default();
        empty.push(b"data: [DONE]\n\n").unwrap();
        assert!(empty.finish(false).is_err());
        let mut complete = SseDecoder::default();
        complete.push(answer).unwrap();
        complete.push(b"data: [DONE]").unwrap();
        complete.finish(false).unwrap();
        let mut finish_reason = SseDecoder::default();
        finish_reason.push(b"data: {\"choices\":[{\"delta\":{\"content\":\"answer\"},\"finish_reason\":\"stop\"}]}\n").unwrap();
        finish_reason.finish(false).unwrap();
    }
    #[test]
    fn knowledge_chat_requires_actual_source_events() {
        let mut missing = SseDecoder::default();
        missing
            .push(b"data: {\"choices\":[{\"delta\":{\"content\":\"answer\"}}]}\n\ndata: [DONE]\n")
            .unwrap();
        assert!(missing
            .finish(true)
            .unwrap_err()
            .to_string()
            .contains("unverified"));
        let mut grounded = SseDecoder::default();
        grounded
            .push(b"data: {\"sources\":[{\"source\":{\"name\":\"fixture.md\"}}]}\n")
            .unwrap();
        grounded
            .push(b"data: {\"done\":true,\"choices\":[{\"delta\":{\"content\":\"answer\"}}]}\n")
            .unwrap();
        grounded.finish(true).unwrap();
    }
    #[test]
    fn model_download_requires_success_and_rejects_invalid_progress() {
        let mut partial = PullDecoder::default();
        partial.push(b"{\"status\":\"downloading\"}\n").unwrap();
        assert!(partial.finish().is_err());
        assert!(PullDecoder::default().finish().is_err());
        assert!(PullDecoder::default().push(b"not JSON\n").is_err());
        assert!(PullDecoder::default()
            .push(b"{\"error\":\"private failure\"}\n")
            .is_err());
        let mut complete = PullDecoder::default();
        for byte in b"{\"status\":\"success\"}" {
            complete.push(&[*byte]).unwrap();
        }
        assert_eq!(complete.finish().unwrap()[0]["status"], "success");
    }
    #[tokio::test]
    async fn native_chat_rejects_http_success_with_truncated_stream() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                "data: {\"choices\":[{\"delta\":{\"content\":\"cut short\"}}]}\n\n",
                "text/event-stream",
            ))
            .mount(&server)
            .await;
        let result = stream_chat(
            &Api::testing(server.uri(), "test-session".into()),
            json!({}),
            CancellationToken::new(),
            tauri::ipc::Channel::new(|_| Ok(())),
        )
        .await;
        let error = result.unwrap_err().to_string();
        assert!(error.contains("interrupted"), "Unexpected error: {error}");
    }
    #[tokio::test]
    async fn authenticated_chat_contract() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat/completions"))
            .and(header("Authorization", "Bearer test-session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok":true})))
            .expect(1)
            .mount(&server)
            .await;
        let api = Api {
            client: reqwest::Client::new(),
            base: server.uri(),
            token: "test-session".into(),
        };
        let body = chat_body(
            "qwen3:4b-instruct",
            &[Message {
                role: "user".into(),
                content: "hi".into(),
            }],
            Some("kb"),
        );
        assert_eq!(body["files"][0]["type"], "collection");
        assert_eq!(
            Api::json(api.post("/api/chat/completions").json(&body))
                .await
                .unwrap()["ok"],
            true
        );
    }
    #[tokio::test]
    async fn unavailable_service_is_actionable() {
        let result = Api::json(reqwest::Client::new().get("http://127.0.0.1:1/api/tags")).await;
        assert!(result.is_err());
        assert!(!result.unwrap_err().to_string().contains("token"));
    }
}
