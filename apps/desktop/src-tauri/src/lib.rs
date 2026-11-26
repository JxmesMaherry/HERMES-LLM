mod api;
mod config;
mod credentials;
mod error;
mod services;
mod sync;

use crate::{
    api::{Api, Conversation, Message},
    config::Config,
    error::{message, Result},
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

pub struct AppState {
    root: PathBuf,
    config: Mutex<Config>,
    vault: Arc<Mutex<credentials::Vault>>,
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
        let config = self.settings();
        let token = self
            .vault
            .lock()
            .expect("credential cache lock")
            .token(config.port)?;
        Ok(Api::new(self.client.clone(), &config, token))
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
    let config = state.settings();
    let (token, unlocked) = state
        .vault
        .try_lock()
        .map(|vault| (vault.token(config.port).ok(), vault.unlocked()))
        .unwrap_or((None, false));
    Ok(services::health(&state.client, &config, &state.root, token, unlocked).await)
}
fn require_unlocked(state: &AppState) -> Result<()> {
    if state
        .vault
        .lock()
        .expect("credential cache lock")
        .unlocked()
    {
        Ok(())
    } else {
        Err(message("Unlock HERMES using the password you chose in the app. No Mac or Keychain password is needed."))
    }
}
#[tauri::command]
fn credential_status(state: State<'_, AppState>) -> credentials::Status {
    state.vault.lock().expect("credential cache lock").status()
}
#[tauri::command]
async fn unlock_credentials(state: State<'_, AppState>, password: String) -> Result<()> {
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| message("Wait for the current operation."))?;
    let vault = state.vault.clone();
    let password = zeroize::Zeroizing::new(password);
    tokio::task::spawn_blocking(move || {
        vault
            .lock()
            .expect("credential cache lock")
            .unlock(&password)
    })
    .await
    .map_err(|_| message("HERMES unlock was interrupted. Retry when ready."))?
}
#[tauri::command]
async fn create_credentials(
    state: State<'_, AppState>,
    password: String,
    confirmation: String,
    reset: bool,
) -> Result<()> {
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| message("Wait for the current operation."))?;
    let password = zeroize::Zeroizing::new(password);
    let confirmation = zeroize::Zeroizing::new(confirmation);
    if *password != *confirmation {
        return Err(message("The HERMES passwords do not match."));
    }
    credentials::validate_password(&password)?;
    let cached = state
        .vault
        .lock()
        .expect("credential cache lock")
        .backend_secret()
        .ok();
    let secret = match cached {
        Some(secret) => secret,
        None => services::secret_for_password_setup(reset).await?,
    };
    let vault = state.vault.clone();
    tokio::task::spawn_blocking(move || {
        vault
            .lock()
            .expect("credential cache lock")
            .create(&password, secret, reset)
    })
    .await
    .map_err(|_| message("Could not finish password setup."))?
}
#[tauri::command]
async fn recovery_accounts(state: State<'_, AppState>) -> Result<Value> {
    require_unlocked(&state)?;
    services::account_recovery(json!({"action":"list"})).await
}
#[tauri::command]
async fn reset_local_account(
    state: State<'_, AppState>,
    email: String,
    password: String,
    confirmation: String,
    confirmed: bool,
) -> Result<()> {
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| message("Wait for the current operation."))?;
    require_unlocked(&state)?;
    let password = zeroize::Zeroizing::new(password);
    let confirmation = zeroize::Zeroizing::new(confirmation);
    if !confirmed || *password != *confirmation {
        return Err(message(
            "Confirm the account reset and matching new passwords.",
        ));
    }
    credentials::validate_password(&password)?;
    if !email.contains('@') || email.len() > 254 {
        return Err(message("Select the local administrator to recover."));
    }
    services::account_recovery(
        json!({"action":"reset","email":email,"password":password.as_str()}),
    )
    .await?;
    let config = state.settings();
    let token = api::authenticate(&state.client, &config, &email, &password, false).await?;
    let vault = state.vault.clone();
    tokio::task::spawn_blocking(move || {
        vault
            .lock()
            .expect("credential cache lock")
            .set_token(config.port, Some(token))
    })
    .await
    .map_err(|_| message("The account was reset; sign in with your new local-account password."))?
}
#[tauri::command]
async fn backend_action(state: State<'_, AppState>, action: String) -> Result<String> {
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| message("A backend or sync operation is already running."))?;
    let config = state.settings();
    let secret = if action == "start" || action == "restart" {
        require_unlocked(&state)?;
        Some(
            state
                .vault
                .lock()
                .expect("credential cache lock")
                .backend_secret()?,
        )
    } else {
        None
    };
    let output = services::lifecycle(&state.root, &config, &action, secret).await?;
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
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| message("Wait for the current operation before signing in."))?;
    require_unlocked(&state)?;
    let config = state.settings();
    let token = api::authenticate(&state.client, &config, &email, &password, create).await?;
    let vault = state.vault.clone();
    tokio::task::spawn_blocking(move || {
        vault
            .lock()
            .expect("credential cache lock")
            .set_token(config.port, Some(token))
    })
    .await
    .map_err(|_| message("Could not finish saving the local session."))?
}
#[tauri::command]
async fn sign_out(state: State<'_, AppState>) -> Result<()> {
    let _guard = state
        .operation
        .try_lock()
        .map_err(|_| message("Wait for the current operation before signing out."))?;
    let port = state.settings().port;
    let vault = state.vault.clone();
    tokio::task::spawn_blocking(move || {
        vault
            .lock()
            .expect("credential cache lock")
            .set_token(port, None)
    })
    .await
    .map_err(|_| message("Could not finish signing out."))?
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
                root: root.clone(),
                config: Mutex::new(settings),
                vault: Arc::new(Mutex::new(credentials::Vault::open(credentials::path(
                    &root,
                )))),
                client,
                plan: Mutex::new(None),
                chats: Mutex::new(HashMap::new()),
                operation: tokio::sync::Mutex::new(()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            unlock_credentials,
            credential_status,
            create_credentials,
            recovery_accounts,
            reset_local_account,
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

// Used only by source-development CLI tools. Password input is JSON on stdin,
// never argv. This branch runs before Tauri/Cocoa and never calls Keychain.
pub fn run_vault_cli() -> bool {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--vault-protocol") {
        println!("HERMES_VAULT_CLI_V1");
        return true;
    }
    if !args.get(1).is_some_and(|a| a == "--vault-read") {
        return false;
    }
    let result = (|| -> Result<String> {
        use std::io::Read;
        if args.len() != 5 || !["backend-secret", "token"].contains(&args[2].as_str()) {
            return Err(message("Invalid vault helper request."));
        }
        let port: u16 = args[4]
            .parse()
            .map_err(|_| message("Invalid backend port."))?;
        let mut input = zeroize::Zeroizing::new(Vec::new());
        std::io::stdin()
            .take(16 * 1024 + 1)
            .read_to_end(&mut input)?;
        if input.len() > 16 * 1024 {
            return Err(message("Vault helper request is too large."));
        }
        #[derive(serde::Deserialize)]
        struct Input {
            password: String,
        }
        let input: Input =
            serde_json::from_slice(&input).map_err(|_| message("Invalid vault helper input."))?;
        let password = zeroize::Zeroizing::new(input.password);
        let mut vault = credentials::Vault::open(credentials::path(std::path::Path::new(&args[3])));
        vault.unlock(&password)?;
        if args[2] == "backend-secret" {
            vault.backend_secret()
        } else {
            vault.token(port)
        }
    })();
    match result {
        Ok(secret) => print!("{secret}"),
        Err(error) => {
            eprintln!("HERMES: {error}");
            std::process::exit(1);
        }
    }
    true
}
