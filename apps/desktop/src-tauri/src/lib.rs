mod api;
mod config;
mod error;
mod services;
mod sync;

use crate::{
    api::{Api, Conversation, Message},
    config::Config,
    error::{message, Result},
};
use serde_json::{json, Value};
use std::{collections::HashMap, path::PathBuf, sync::Mutex, time::Duration};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

pub struct AppState {
    root: PathBuf,
    config: Mutex<Config>,
    client: reqwest::Client,
    plan: Mutex<Option<sync::Plan>>,
    chats: Mutex<HashMap<String, CancellationToken>>,
    operation: tokio::sync::Mutex<()>,
}
impl AppState {
    fn settings(&self) -> Config {
        self.config.lock().expect("config lock").clone()
    }
    fn api(&self) -> Result<Api> {
        Api::new(self.client.clone(), &self.settings())
    }
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Config {
    state.settings()
}
#[tauri::command]
async fn save_config(state: State<'_, AppState>, mut settings: Config) -> Result<Config> {
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| message("Wait for the current backend or sync operation to finish."))?;
    // A frontend cannot grant itself access to an arbitrary path; only the native picker changes it.
    let old = state.settings();
    if settings.port != old.port
        && std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, old.port)).is_err()
    {
        return Err(message("Stop the backend before changing its port."));
    }
    settings.folder = old.folder;
    config::save(&state.root, &settings)?;
    *state.config.lock().expect("config lock") = settings.clone();
    *state.plan.lock().expect("plan lock") = None;
    Ok(settings)
}
#[tauri::command]
async fn health(state: State<'_, AppState>) -> Result<services::Health> {
    Ok(services::health(&state.client, &state.settings(), &state.root).await)
}
#[tauri::command]
async fn backend_action(state: State<'_, AppState>, action: String) -> Result<String> {
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| message("A backend or sync operation is already running."))?;
    let config = state.settings();
    let output = services::lifecycle(&state.root, &config, &action).await?;
    if action == "start" || action == "restart" {
        services::wait_ready(&state.client, &config).await?;
    }
    Ok(output)
}
#[tauri::command]
async fn open_target(state: State<'_, AppState>, target: String) -> Result<()> {
    services::open_target(&target, &state.settings()).await
}
#[tauri::command]
async fn authenticate(
    state: State<'_, AppState>,
    email: String,
    password: String,
    create: bool,
) -> Result<()> {
    api::authenticate(&state.client, &state.settings(), &email, &password, create).await
}
#[tauri::command]
fn sign_out(state: State<'_, AppState>) -> Result<()> {
    let entry = config::credential(&format!("webui-token-{}", state.settings().port))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(message("Could not remove the session from Keychain.")),
    }
}
#[tauri::command]
async fn list_models(state: State<'_, AppState>) -> Result<Value> {
    api::models(&state.client).await
}
#[tauri::command]
async fn pull_model(
    state: State<'_, AppState>,
    model: String,
    progress: tauri::ipc::Channel<Value>,
) -> Result<()> {
    // Calling this command is a deliberate UI download action; it never occurs on launch.
    api::pull(&state.client, &model, &progress).await
}
#[tauri::command]
async fn choose_folder(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<Config> {
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| message("Wait for the current operation before changing source folders."))?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Select a document folder for local indexing")
        .pick_folder(move |folder| {
            let _ = sender.send(folder);
        });
    if let Some(folder) = receiver
        .await
        .map_err(|_| message("Folder picker closed unexpectedly."))?
    {
        let path = folder
            .into_path()
            .map_err(|_| message("Select a local document directory."))?;
        let path = sync::approved_root(
            &path,
            &app.path()
                .home_dir()
                .map_err(|_| message("Cannot locate your home directory."))?,
        )?;
        let mut config = state.settings();
        config.folder = Some(path);
        config::save(&state.root, &config)?;
        *state.config.lock().expect("config lock") = config;
        *state.plan.lock().expect("plan lock") = None;
    }
    Ok(state.settings())
}
#[tauri::command]
async fn list_knowledge(state: State<'_, AppState>) -> Result<Value> {
    // Traverse the upstream paginated list, rather than hiding KBs after the first page.
    let api = state.api()?;
    let mut items = vec![];
    for page in 1..=100 {
        let value = Api::json(api.get(&format!("/api/v1/knowledge/?page={page}"))).await?;
        let batch = value["items"]
            .as_array()
            .ok_or_else(|| message("Unsupported knowledge-base listing response."))?;
        items.extend(batch.clone());
        if batch.is_empty() || items.len() >= value["total"].as_u64().unwrap_or(0) as usize {
            break;
        }
    }
    Ok(json!({"items":items}))
}
#[tauri::command]
async fn create_knowledge(state: State<'_, AppState>, name: String) -> Result<Value> {
    if name.trim().is_empty() || name.len() > 100 {
        return Err(message(
            "Knowledge-base name must contain 1 to 100 characters.",
        ));
    }
    Api::json(state.api()?.post("/api/v1/knowledge/create").json(
        &json!({"name":name.trim(),"description":"Local documents synchronised by HERMES-LLM"}),
    ))
    .await
}
#[tauri::command]
async fn preview_sync(state: State<'_, AppState>) -> Result<sync::Preview> {
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| message("Wait for the current backend or sync operation."))?;
    let plan = sync::preview(&state.api()?, &state.settings()).await?;
    let preview = plan.preview.clone();
    *state.plan.lock().expect("plan lock") = Some(plan);
    Ok(preview)
}
#[tauri::command]
async fn apply_sync(
    state: State<'_, AppState>,
    preview_id: String,
    confirm_deletions: bool,
    progress: tauri::ipc::Channel<Value>,
) -> Result<sync::SyncResult> {
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| message("A backend or sync operation is already running."))?;
    let plan = state
        .plan
        .lock()
        .expect("plan lock")
        .take()
        .ok_or_else(|| message("Preview changes before syncing."))?;
    if plan.preview.id != preview_id {
        return Err(message("Preview no longer matches. Preview again."));
    }
    let result = sync::apply(
        &state.api()?,
        &state.settings(),
        plan,
        confirm_deletions,
        &progress,
    )
    .await?;
    config::atomic_write(
        &state.root.join("sync-attempt.json"),
        &serde_json::to_vec_pretty(&result)?,
    )?;
    if result.failed == 0 {
        config::atomic_write(
            &state.root.join("sync-success.json"),
            &serde_json::to_vec_pretty(&result)?,
        )?;
    }
    Ok(result)
}
#[tauri::command]
fn last_sync(state: State<'_, AppState>) -> Result<Value> {
    let read = |name| -> Result<Value> {
        let path = state.root.join(name);
        if path.exists() {
            Ok(serde_json::from_slice(&std::fs::read(path)?)?)
        } else {
            Ok(Value::Null)
        }
    };
    Ok(json!({"attempt":read("sync-attempt.json")?,"success":read("sync-success.json")?}))
}
#[tauri::command]
async fn chat_send(
    state: State<'_, AppState>,
    request_id: String,
    model: String,
    messages: Vec<Message>,
    use_knowledge: bool,
    events: tauri::ipc::Channel<Value>,
) -> Result<()> {
    config::validate_id(&request_id)?;
    if !config::valid_model(&model)
        || messages.is_empty()
        || messages.len() > 1000
        || messages
            .iter()
            .any(|m| !["user", "assistant"].contains(&m.role.as_str()) || m.content.len() > 200_000)
    {
        return Err(message("Invalid chat request or message too large."));
    }
    let config = state.settings();
    let knowledge = if use_knowledge {
        Some(
            config
                .knowledge_id
                .as_deref()
                .ok_or_else(|| message("Select and sync a knowledge base first."))?,
        )
    } else {
        None
    };
    let cancel = CancellationToken::new();
    {
        let mut chats = state.chats.lock().expect("chat lock");
        if !chats.is_empty() {
            return Err(message("Wait for the current response or cancel it."));
        }
        chats.insert(request_id.clone(), cancel.clone());
    }
    let result = match state.api() {
        Ok(api) => {
            api::stream_chat(
                &api,
                api::chat_body(&model, &messages, knowledge),
                cancel,
                events,
            )
            .await
        }
        Err(e) => Err(e),
    };
    state.chats.lock().expect("chat lock").remove(&request_id);
    result
}
#[tauri::command]
fn cancel_chat(state: State<'_, AppState>, request_id: String) {
    if let Some(token) = state.chats.lock().expect("chat lock").get(&request_id) {
        token.cancel();
    }
}
#[tauri::command]
fn load_history(state: State<'_, AppState>) -> Result<Vec<Conversation>> {
    let path = state.root.join("conversations.json");
    if !path.exists() {
        return Ok(vec![]);
    }
    if std::fs::metadata(&path)?.len() > 8 * 1024 * 1024 {
        return Err(message(
            "Conversation store exceeds 8 MB. Back it up before clearing it.",
        ));
    }
    let chats: Vec<Conversation> = serde_json::from_slice(&std::fs::read(path)?)?;
    api::validate_history(&chats)?;
    Ok(chats)
}
#[tauri::command]
fn save_history(state: State<'_, AppState>, conversations: Vec<Conversation>) -> Result<()> {
    api::validate_history(&conversations)?;
    let bytes = serde_json::to_vec(&conversations)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(message(
            "Conversation history exceeds 8 MB. Remove old desktop conversations.",
        ));
    }
    config::atomic_write(&state.root.join("conversations.json"), &bytes)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            let resource = app.path().resource_dir()?.join("infra/compose.yaml");
            let settings = config::initialize(&root, &resource)?;
            let client = reqwest::Client::builder()
                .no_proxy()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(120))
                .redirect(reqwest::redirect::Policy::none())
                .build()?;
            app.manage(AppState {
                root,
                config: Mutex::new(settings),
                client,
                plan: Mutex::new(None),
                chats: Mutex::new(HashMap::new()),
                operation: tokio::sync::Mutex::new(()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            health,
            backend_action,
            open_target,
            authenticate,
            sign_out,
            list_models,
            pull_model,
            choose_folder,
            list_knowledge,
            create_knowledge,
            preview_sync,
            apply_sync,
            last_sync,
            chat_send,
            cancel_chat,
            load_history,
            save_history
        ])
        .run(tauri::generate_context!())
        .expect("Unable to launch HERMES-LLM");
}
