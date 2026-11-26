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
    let mut buffer = Vec::new();
    while let Some(chunk) = tokio::time::timeout(Duration::from_secs(180), stream.next())
        .await
        .map_err(|_| message("Model download stalled. Retry when the connection is available."))?
    {
        buffer.extend_from_slice(&chunk?);
        if buffer.len() > 1024 * 1024 {
            return Err(message("Ollama sent an oversized progress event."));
        }
        while let Some(pos) = buffer.iter().position(|c| *c == b'\n') {
            let line: Vec<u8> = buffer.drain(..=pos).collect();
            if let Ok(value) = serde_json::from_slice::<Value>(&line) {
                if value.get("error").is_some() {
                    return Err(message("Ollama could not download this model. Check its name, storage, and connection."));
                }
                let _ = channel.send(value);
            }
        }
    }
    Ok(())
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
}
impl SseDecoder {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Value>> {
        self.pending.extend_from_slice(bytes);
        if self.pending.len() > 4 * 1024 * 1024 {
            return Err(message("Backend stream event exceeded the safety limit."));
        }
        let mut output = Vec::new();
        while let Some(pos) = self.pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=pos).collect();
            let text = String::from_utf8_lossy(&line);
            if let Some(data) = text.trim().strip_prefix("data:") {
                let data = data.trim();
                if data.is_empty() || data == "[DONE]" {
                    continue;
                }
                let event: Value = serde_json::from_str(data)?;
                if event.get("error").is_some() {
                    return Err(message("The model returned an error. Verify the selected model and inspect backend logs."));
                }
                output.push(event);
            }
        }
        Ok(output)
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
