use crate::{
    config::Config,
    error::{message, Result},
};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::process::Command;

#[derive(Clone, Debug, Serialize)]
pub struct ServiceStatus {
    pub state: String,
    pub detail: String,
}
impl ServiceStatus {
    fn new(state: &str, detail: &str) -> Self {
        Self {
            state: state.into(),
            detail: detail.into(),
        }
    }
}
#[derive(Serialize)]
pub struct Health {
    pub docker: ServiceStatus,
    pub ollama: ServiceStatus,
    pub backend: ServiceStatus,
    pub authenticated: bool,
    pub credentials_unlocked: bool,
    pub folder: ServiceStatus,
    pub data_dir: String,
}

pub fn docker_path() -> Option<PathBuf> {
    // GUI launches do not inherit a Terminal PATH. Only search known installation locations.
    [
        "/Applications/Docker.app/Contents/Resources/bin/docker",
        "/usr/local/bin/docker",
        "/opt/homebrew/bin/docker",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|p| p.is_file())
}
async fn run_docker(binary: &Path, args: &[&str], limit: u64) -> Result<std::process::Output> {
    let mut cmd = Command::new(binary);
    cmd.args(args).kill_on_drop(true);
    tokio::time::timeout(Duration::from_secs(limit), cmd.output())
        .await
        .map_err(|_| message("Docker did not respond in time. Open Docker Desktop and retry."))?
        .map_err(Into::into)
}
async fn docker_status(binary: Option<&Path>) -> ServiceStatus {
    let Some(binary) = binary else {
        return ServiceStatus::new("missing", "Install Docker Desktop from docker.com");
    };
    match run_docker(binary, &["info", "--format", "{{.ServerVersion}}"], 8).await {
        Ok(output) if output.status.success() => {
            ServiceStatus::new("running", "Docker daemon is ready")
        }
        _ => ServiceStatus::new("stopped", "Open Docker Desktop and wait for the engine"),
    }
}
async fn ollama_status(client: &reqwest::Client, url: &str, installed: bool) -> ServiceStatus {
    if let Ok(response) = client.get(url).timeout(Duration::from_secs(4)).send().await {
        if response.status().is_success()
            && response
                .json::<serde_json::Value>()
                .await
                .is_ok_and(|v| v["models"].is_array())
        {
            return ServiceStatus::new("running", "Native Ollama API is ready");
        }
    }
    if installed {
        ServiceStatus::new("stopped", "Open Ollama to start the local API")
    } else {
        ServiceStatus::new("missing", "Install Ollama from ollama.com")
    }
}
async fn backend_status(client: &reqwest::Client, base: &str) -> ServiceStatus {
    match client
        .get(format!("{base}/health"))
        .timeout(Duration::from_secs(4))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            if let Ok(response) = client
                .get(format!("{base}/api/version"))
                .timeout(Duration::from_secs(4))
                .send()
                .await
            {
                if response.status().is_success()
                    && response
                        .json::<serde_json::Value>()
                        .await
                        .is_ok_and(|v| v["version"] == "0.13.5")
                {
                    return ServiceStatus::new("running", "Open WebUI v0.13.5 is responding");
                }
            }
            ServiceStatus::new("failed", "This port is not running supported Open WebUI v0.13.5. Check port settings and backend logs")
        }
        Ok(_) => ServiceStatus::new("failed", "Backend health check failed; inspect logs"),
        Err(_) => ServiceStatus::new(
            "stopped",
            "Start the backend, or inspect logs if startup failed",
        ),
    }
}
fn action_args(action: &str) -> Result<Vec<&'static str>> {
    match action {
        "start" => Ok(vec!["up", "-d"]),
        "stop" => Ok(vec!["stop"]),
        "restart" => Ok(vec!["up", "-d", "--force-recreate"]),
        "logs" => Ok(vec!["logs", "--tail", "120", "--no-color", "open-webui"]),
        _ => Err(message("Unsupported service action.")),
    }
}
pub async fn health(
    client: &reqwest::Client,
    config: &Config,
    root: &Path,
    token: Option<String>,
    credentials_unlocked: bool,
) -> Health {
    let docker = docker_status(docker_path().as_deref()).await;
    let installed = Path::new("/Applications/Ollama.app").exists()
        || Path::new("/usr/local/bin/ollama").exists()
        || Path::new("/opt/homebrew/bin/ollama").exists();
    let ollama = ollama_status(client, "http://127.0.0.1:11434/api/tags", installed).await;
    let backend = backend_status(client, &config.base_url()).await;
    let authenticated = match token {
        Some(token) if backend.state == "running" => client
            .get(format!("{}/api/v1/auths/", config.base_url()))
            .bearer_auth(token)
            .timeout(Duration::from_secs(4))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success()),
        _ => false,
    };
    let folder = match &config.folder {
        Some(p) if p.is_dir() && std::fs::read_dir(p).is_ok() => {
            ServiceStatus::new("ready", &p.display().to_string())
        }
        Some(_) => ServiceStatus::new(
            "failed",
            "Document folder is missing or inaccessible; select it again",
        ),
        None => ServiceStatus::new("unconfigured", "Choose a document folder"),
    };
    Health {
        docker,
        ollama,
        backend,
        authenticated,
        credentials_unlocked,
        folder,
        data_dir: root.display().to_string(),
    }
}

// Read only the explicitly HERMES-labelled container. No Keychain API is used.
async fn managed_container(include_stopped: bool) -> Result<(PathBuf, String)> {
    let docker = docker_path().ok_or_else(|| message("Install Docker Desktop first."))?;
    let mut args = vec!["ps"];
    if include_stopped {
        args.push("-a");
    }
    args.extend([
        "--filter",
        "label=com.docker.compose.project=hermes-llm",
        "--filter",
        "label=com.docker.compose.service=open-webui",
        "--format",
        "{{.ID}}",
    ]);
    let output = run_docker(&docker, &args, 5).await?;
    if !output.status.success() {
        return Err(message(
            "Open Docker Desktop before password setup or recovery.",
        ));
    }
    let ids = String::from_utf8(output.stdout)
        .map_err(|_| message("Docker returned invalid container metadata."))?;
    let id = ids
        .lines()
        .next()
        .ok_or_else(|| message("Start the HERMES backend before recovering its local account."))?
        .to_owned();
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(message("Invalid HERMES container identity."));
    }
    Ok((docker, id))
}
pub async fn secret_for_password_setup(reset: bool) -> Result<String> {
    let docker = docker_path()
        .ok_or_else(|| message("Open Docker Desktop before setting your HERMES password."))?;
    if !run_docker(&docker, &["info", "--format", "{{.ServerVersion}}"], 5)
        .await?
        .status
        .success()
    {
        return Err(message("Open Docker Desktop before password setup so existing backend credentials can be preserved."));
    }
    if let Ok((docker, id)) = managed_container(true).await {
        let output = run_docker(
            &docker,
            &["inspect", "--format", "{{json .Config.Env}}", &id],
            5,
        )
        .await?;
        if output.status.success() {
            let values: Vec<String> = serde_json::from_slice(&output.stdout)?;
            if let Some(secret) = values.into_iter().find_map(|v| {
                v.strip_prefix("WEBUI_SECRET_KEY=")
                    .filter(|s| s.len() >= 24 && s.len() <= 1024)
                    .map(str::to_owned)
            }) {
                return Ok(secret);
            }
        }
    }
    let volume = run_docker(
        &docker,
        &[
            "volume",
            "inspect",
            "hermes-llm-open-webui-data",
            "--format",
            "{{.Name}}",
        ],
        5,
    )
    .await?;
    if volume.status.success() && !reset {
        return Err(message("Existing backend data was found without its old container. Choose Reset saved sign-ins to explicitly replace only authentication credentials; books and chats will be kept."));
    }
    Ok(crate::credentials::fresh_backend_secret())
}
pub async fn account_recovery(payload: serde_json::Value) -> Result<serde_json::Value> {
    use std::process::Stdio;
    use tokio::io::AsyncWriteExt;
    let (docker, id) = managed_container(false).await?;
    let mounts = run_docker(
        &docker,
        &["inspect", "--format", "{{json .Mounts}}", &id],
        5,
    )
    .await?;
    let mounts: serde_json::Value = serde_json::from_slice(&mounts.stdout)?;
    if !mounts.as_array().is_some_and(|items| {
        items.iter().any(|m| {
            m["Name"] == "hermes-llm-open-webui-data" && m["Destination"] == "/app/backend/data"
        })
    }) {
        return Err(message(
            "Recovery is restricted to the HERMES-managed local data volume.",
        ));
    }
    let input = zeroize::Zeroizing::new(serde_json::to_vec(&payload)?);
    let mut cmd = Command::new(docker);
    cmd.args([
        "exec",
        "-i",
        &id,
        "python",
        "-c",
        include_str!("../resources/account_recovery.py"),
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .kill_on_drop(true);
    let mut child = cmd.spawn()?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| message("Could not open local recovery input."))?;
    stdin.write_all(&input).await?;
    drop(stdin);
    let output = tokio::time::timeout(Duration::from_secs(90), child.wait_with_output())
        .await
        .map_err(|_| {
            message("Local recovery timed out. Existing books and chats were not removed.")
        })??;
    if !output.status.success() {
        return Err(message("Local account recovery failed. Check the selected administrator and backend availability. No books or chats were removed."));
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

pub async fn lifecycle(
    root: &Path,
    config: &Config,
    action: &str,
    secret: Option<String>,
) -> Result<String> {
    let secret = match action {
        "start" | "restart" => secret
            .ok_or_else(|| message("Unlock saved credentials before starting the backend."))?,
        _ => "unused-for-stop-or-logs".into(),
    };
    let docker = docker_path().ok_or_else(|| message("Install Docker Desktop first."))?;
    let args = action_args(action)?;
    let daemon = run_docker(&docker, &["info", "--format", "{{.ServerVersion}}"], 10).await?;
    if !daemon.status.success() {
        return Err(message("Start Docker Desktop and wait for its engine."));
    }
    if (action == "start" || action == "restart")
        && std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, config.port)).is_err()
    {
        // Only tolerate a listener owned by the existing HERMES Compose service.
        let inspect = run_docker(
            &docker,
            &[
                "ps",
                "--filter",
                "label=com.docker.compose.project=hermes-llm",
                "--filter",
                "label=com.docker.compose.service=open-webui",
                "--format",
                "{{.Ports}}",
            ],
            10,
        )
        .await?;
        let expected = format!("127.0.0.1:{}->8080/tcp", config.port);
        if !inspect.status.success()
            || !String::from_utf8_lossy(&inspect.stdout).contains(&expected)
        {
            return Err(message(
                "The backend port is already in use. Choose another port in Settings.",
            ));
        }
    }
    let mut cmd = Command::new(docker);
    cmd.args(["compose", "--project-name", "hermes-llm", "--file"])
        .arg(root.join("compose.yaml"));
    cmd.args(args)
        .env("HERMES_PORT", config.port.to_string())
        .env(
            "HERMES_HF_OFFLINE",
            if config.allow_downloads { "0" } else { "1" },
        )
        .env("WEBUI_SECRET_KEY", secret)
        .kill_on_drop(true);
    // First pull can take several minutes; never retry forever.
    let output = tokio::time::timeout(Duration::from_secs(900), cmd.output()).await.map_err(|_| message("Docker exceeded 15 minutes. Check Docker Desktop and retry; cached downloads are preserved."))??;
    if !output.status.success() {
        return Err(message("Docker Compose failed. Check Docker Desktop, available storage, port settings, and network access. Inspect backend logs."));
    }
    if action == "logs" {
        return Ok(format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(format!("Backend {action} command completed."))
}
pub async fn wait_ready(client: &reqwest::Client, config: &Config) -> Result<()> {
    for _ in 0..180 {
        let status = backend_status(client, &config.base_url()).await;
        if status.state == "running" {
            return Ok(());
        }
        if status.state == "failed" {
            return Err(message(status.detail));
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err(message("Backend was not ready within 6 minutes. Inspect logs, then retry. First launch may need the consented embedding download."))
}
pub async fn open_target(target: &str, config: &Config) -> Result<()> {
    let args: Vec<String> = match target {
        "docker" => vec!["-a".into(), "Docker".into()],
        "ollama" => vec!["-a".into(), "Ollama".into()],
        "docker-download" => vec!["https://www.docker.com/products/docker-desktop/".into()],
        "ollama-download" => vec!["https://ollama.com/download/mac".into()],
        "webui" => vec![config.base_url()],
        "knowledge" => vec![format!(
            "{}/workspace/knowledge/{}",
            config.base_url(),
            config.knowledge_id.as_deref().unwrap_or("")
        )],
        _ => return Err(message("Unsupported destination.")),
    };
    let out = Command::new("/usr/bin/open").args(args).output().await?;
    if !out.status.success() {
        return Err(message(
            "macOS could not open that application or page. Check its installation.",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn unavailable_command_is_an_error() {
        assert!(run_docker(Path::new("/not/a/docker"), &["info"], 1)
            .await
            .is_err());
    }
    #[test]
    fn rejects_arbitrary_service_commands() {
        assert!(action_args("start; rm -rf /").is_err());
        assert!(action_args("down --volumes").is_err());
        assert_eq!(action_args("stop").unwrap(), vec!["stop"]);
    }
    #[tokio::test]
    async fn distinguishes_missing_docker_from_stopped_daemon() {
        assert_eq!(docker_status(None).await.state, "missing");
        assert_eq!(
            docker_status(Some(Path::new("/usr/bin/false"))).await.state,
            "stopped"
        );
    }
    #[tokio::test]
    async fn unavailable_ollama_does_not_report_success() {
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        assert_eq!(
            ollama_status(&client, "http://127.0.0.1:1/api/tags", true)
                .await
                .state,
            "stopped"
        );
        assert_eq!(
            ollama_status(&client, "http://127.0.0.1:1/api/tags", false)
                .await
                .state,
            "missing"
        );
    }
    #[tokio::test]
    async fn unrelated_health_endpoint_is_not_a_ready_backend() {
        use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(path("/health"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        assert_eq!(backend_status(&client, &server.uri()).await.state, "failed");
    }
}
