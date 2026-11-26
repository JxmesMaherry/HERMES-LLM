use crate::{
    api::Api,
    config::{self, Config},
    error::{message, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use walkdir::WalkDir;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub filename: String,
    pub path: String,
    pub checksum: String,
    pub size: u64,
}
impl ManifestEntry {
    fn relative(&self) -> PathBuf {
        Path::new(&self.path).join(&self.filename)
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Change {
    pub filename: String,
    #[serde(default)]
    pub path: String,
    pub stale_file_id: Option<String>,
    pub file_id: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Diff {
    pub added: Vec<Change>,
    pub modified: Vec<Change>,
    pub deleted: Vec<Change>,
    pub mkdir: Vec<String>,
    pub rmdir: Vec<String>,
    pub unmodified_count: usize,
    pub directory_map: HashMap<String, String>,
}
#[derive(Clone, Serialize)]
pub struct Preview {
    pub id: String,
    pub added: usize,
    pub modified: usize,
    pub unchanged: usize,
    pub skipped: usize,
    pub deletions: Vec<String>,
    pub scanned: usize,
}
pub struct Plan {
    pub preview: Preview,
    pub manifest: Vec<ManifestEntry>,
    pub diff: Diff,
    pub folder: PathBuf,
    pub knowledge: String,
    pub created: Instant,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SyncResult {
    pub added: usize,
    pub modified: usize,
    pub unchanged: usize,
    pub skipped: usize,
    pub failed: usize,
    pub deleted: usize,
    pub retained: usize,
    pub errors: Vec<String>,
    pub completed_at: u64,
    pub folder: PathBuf,
    pub knowledge_id: String,
}
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn approved_root(path: &Path, home: &Path) -> Result<PathBuf> {
    let root = path.canonicalize()?;
    let home = home.canonicalize()?;
    if !root.is_dir() || home.starts_with(&root) {
        return Err(message("Choose a specific document directory, rather than your home directory or a parent of it."));
    }
    std::fs::read_dir(&root)?;
    Ok(root)
}
pub fn excluded(name: &str) -> bool {
    name.starts_with('.')
        || [
            "node_modules",
            "vendor",
            "target",
            "dist",
            "build",
            "Library",
            "Applications",
            "__pycache__",
            "venv",
        ]
        .contains(&name)
}
pub fn supported(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        ["pdf", "txt", "md", "markdown"].contains(&e.to_ascii_lowercase().as_str())
    })
}
fn safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}
fn read_document(dir: &cap_std::fs::Dir, path: &Path, limit: u64) -> Result<Vec<u8>> {
    if !safe_relative(path) {
        return Err(message("Invalid document path."));
    }
    // Capability-relative open prevents a replaced symlink from escaping the selected root.
    let file = dir.open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > limit {
        return Err(message(
            "Document is not a regular file or exceeds the configured limit.",
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(message("Document grew beyond the configured limit."));
    }
    Ok(bytes)
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn scan(root: &Path, max_mb: u64) -> Result<(Vec<ManifestEntry>, usize)> {
    if !root.is_dir() {
        return Err(message("Document folder is unavailable. Select it again."));
    }
    let dir = cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority())?;
    let mut files = vec![];
    let mut skipped = 0usize;
    let entries = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !excluded(&e.file_name().to_string_lossy()));
    let mut visited = 0;
    for entry in entries {
        let entry = entry.map_err(|_| message("A document or subfolder could not be read. Sync stopped to protect indexed content. Check folder permissions."))?;
        visited += 1;
        if visited > 20_000 {
            return Err(message(
                "Folder contains over 20,000 entries. Select a smaller directory.",
            ));
        }
        if entry.file_type().is_symlink() {
            skipped += 1;
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if !supported(entry.path()) {
            skipped += 1;
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| message("Document escaped the selected directory."))?;
        let Some(filename) = relative.file_name().and_then(|s| s.to_str()) else {
            skipped += 1;
            continue;
        };
        let Some(parent) = relative.parent().and_then(|s| s.to_str()) else {
            skipped += 1;
            continue;
        };
        let size = entry
            .metadata()
            .map_err(|_| message("Cannot read document metadata. Check folder permissions."))?
            .len();
        if size == 0 || size > max_mb * 1024 * 1024 {
            skipped += 1;
            continue;
        }
        let bytes = read_document(&dir, relative, max_mb * 1024 * 1024)?;
        if !filename.to_ascii_lowercase().ends_with(".pdf")
            && (bytes.contains(&0) || std::str::from_utf8(&bytes).is_err())
        {
            skipped += 1;
            continue;
        }
        files.push(ManifestEntry {
            filename: filename.into(),
            path: parent.replace('\\', "/"),
            checksum: hash(&bytes),
            size: bytes.len() as u64,
        });
        if files.len() > 5000 {
            return Err(message(
                "Select a smaller folder: the initial release supports up to 5,000 documents.",
            ));
        }
    }
    files.sort_by_key(|f| f.relative());
    Ok((files, skipped))
}
pub async fn preview(api: &Api, config: &Config) -> Result<Plan> {
    let folder = config
        .folder
        .clone()
        .ok_or_else(|| message("Select a document folder first."))?;
    let knowledge = config
        .knowledge_id
        .clone()
        .ok_or_else(|| message("Create or select a knowledge base first."))?;
    config::validate_id(&knowledge)?;
    let scan_root = folder.clone();
    let max_mb = config.max_file_mb;
    let (manifest, skipped) = tokio::task::spawn_blocking(move || scan(&scan_root, max_mb))
        .await
        .map_err(|_| message("Folder scan was interrupted."))??;
    let diff: Diff = serde_json::from_value(
        Api::json(
            api.post(&format!("/api/v1/knowledge/{knowledge}/sync/diff"))
                .json(&json!({"manifest":manifest})),
        )
        .await?,
    )?;
    let preview = Preview {
        id: uuid::Uuid::new_v4().to_string(),
        added: diff.added.len(),
        modified: diff.modified.len(),
        unchanged: diff.unmodified_count,
        skipped,
        deletions: diff.deleted.iter().map(|c| c.filename.clone()).collect(),
        scanned: manifest.len(),
    };
    Ok(Plan {
        preview,
        manifest,
        diff,
        folder,
        knowledge,
        created: Instant::now(),
    })
}
pub fn cleanup_ids(diff: &Diff, successful: &[String], allow_deletions: bool) -> Vec<String> {
    let mut ids = successful.to_vec();
    if allow_deletions {
        ids.extend(diff.deleted.iter().filter_map(|c| c.file_id.clone()));
    }
    ids.sort();
    ids.dedup();
    ids
}
async fn upload(
    api: &Api,
    kb: &str,
    dir: &cap_std::fs::Dir,
    entry: &ManifestEntry,
    directory_id: Option<&String>,
    limit: u64,
) -> Result<()> {
    let bytes = read_document(dir, &entry.relative(), limit)?;
    if hash(&bytes) != entry.checksum {
        return Err(message(
            "File changed after preview. Preview again before syncing.",
        ));
    }
    let mime = match entry
        .relative()
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "pdf" => "application/pdf",
        "md" | "markdown" => "text/markdown",
        _ => "text/plain",
    };
    let mut metadata = json!({"knowledge_id":kb,"file_hash":entry.checksum});
    if let Some(id) = directory_id {
        metadata["directory_id"] = json!(id);
    }
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(entry.filename.clone())
        .mime_str(mime)?;
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("metadata", metadata.to_string());
    let file = Api::json(api.post("/api/v1/files/").multipart(form)).await?;
    let id = file["id"]
        .as_str()
        .ok_or_else(|| message("Upload returned no file identifier."))?;
    config::validate_id(id)?;
    // v0.13.5 auto-links and indexes knowledge metadata BEFORE reporting completed.
    for _ in 0..240 {
        let status = Api::json(api.get(&format!("/api/v1/files/{id}/process/status"))).await?;
        match status["status"].as_str() {
            Some("completed") => return Ok(()),
            Some("failed") => return Err(message("Text extraction or embedding failed. A scanned PDF may need OCR outside HERMES. Check local backend logs.")),
            Some("pending" | "processing") => {},
            _ => return Err(message("Unexpected processing status; verify Open WebUI v0.13.5.")),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err(message("Document processing exceeded 8 minutes. No stale version was removed. Inspect the knowledge base before retrying."))
}
pub async fn apply(
    api: &Api,
    config: &Config,
    plan: Plan,
    allow_deletions: bool,
    progress: &tauri::ipc::Channel<Value>,
) -> Result<SyncResult> {
    if plan.created.elapsed() > Duration::from_secs(600)
        || config.folder.as_ref() != Some(&plan.folder)
        || config.knowledge_id.as_ref() != Some(&plan.knowledge)
    {
        return Err(message(
            "Preview expired or source changed. Preview changes again.",
        ));
    }
    let dir = cap_std::fs::Dir::open_ambient_dir(&plan.folder, cap_std::ambient_authority())?;
    let mut map = plan.diff.directory_map.clone();
    let mut mkdir = plan.diff.mkdir.clone();
    mkdir.sort_by_key(|p| p.split('/').count());
    for path in mkdir {
        if !safe_relative(Path::new(&path)) {
            return Err(message("Backend returned an unsafe directory path."));
        }
        let (parent, name) = path.rsplit_once('/').unwrap_or(("", &path));
        let parent_id = if parent.is_empty() {
            None
        } else {
            Some(
                map.get(parent)
                    .ok_or_else(|| message("Missing parent directory in sync response."))?,
            )
        };
        let created = Api::json(
            api.post(&format!("/api/v1/knowledge/{}/dirs/create", plan.knowledge))
                .json(&json!({"name":name,"parent_id":parent_id})),
        )
        .await?;
        let id = created["id"]
            .as_str()
            .ok_or_else(|| message("Directory creation failed."))?;
        config::validate_id(id)?;
        map.insert(path, id.into());
    }
    let mut result = SyncResult {
        unchanged: plan.diff.unmodified_count,
        skipped: plan.preview.skipped,
        folder: plan.folder.clone(),
        knowledge_id: plan.knowledge.clone(),
        ..Default::default()
    };
    let changes: Vec<_> = plan
        .diff
        .added
        .iter()
        .chain(plan.diff.modified.iter())
        .collect();
    let mut stale = vec![];
    for (index, change) in changes.iter().enumerate() {
        let entry = plan
            .manifest
            .iter()
            .find(|e| e.path == change.path && e.filename == change.filename)
            .ok_or_else(|| message("Backend requested a file outside the preview manifest."))?;
        let _ =
            progress.send(json!({"done":index,"total":changes.len(),"filename":entry.filename}));
        match upload(
            api,
            &plan.knowledge,
            &dir,
            entry,
            map.get(&entry.path),
            config.max_file_mb * 1024 * 1024,
        )
        .await
        {
            Ok(()) => {
                if let Some(id) = &change.stale_file_id {
                    stale.push(id.clone());
                    result.modified += 1;
                } else {
                    result.added += 1;
                }
            }
            Err(e) => {
                result.failed += 1;
                result
                    .errors
                    .push(format!("{}: {}", entry.relative().display(), e));
            }
        }
    }
    // Any failure prevents ALL cleanup, so an interrupted batch preserves its old index.
    if result.failed == 0 {
        let ids = cleanup_ids(&plan.diff, &stale, allow_deletions);
        for id in &ids {
            config::validate_id(id)?;
        }
        if !ids.is_empty() {
            Api::json(
                api.post(&format!(
                    "/api/v1/knowledge/{}/sync/cleanup",
                    plan.knowledge
                ))
                .json(&json!({"file_ids":ids,"dir_ids":[]})),
            )
            .await?;
        }
        if allow_deletions {
            result.deleted = plan.diff.deleted.len();
        } else {
            result.retained = plan.diff.deleted.len();
        }
        // Never delete directories: upstream cleanup can remove late-arriving files recursively.
    } else {
        result.retained = plan.diff.deleted.len() + stale.len();
    }
    result.completed_at = now();
    let _ =
        progress.send(json!({"done":changes.len(),"total":changes.len(),"filename":"Complete"}));
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash_is_sha256() {
        assert_eq!(
            hash(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
    #[test]
    fn filters_and_caps_scan() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join("node_modules")).unwrap();
        for file in [
            "notes.md",
            "paper.txt",
            ".env",
            "secret.docx",
            "node_modules/a.txt",
        ] {
            std::fs::write(d.path().join(file), "facts").unwrap();
        }
        let (entries, skipped) = scan(d.path(), 20).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(skipped, 1);
        assert!(entries.iter().all(|e| e.checksum == hash(b"facts")));
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_escape_root() {
        let d = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        std::fs::write(out.path().join("secret.txt"), "secret").unwrap();
        std::os::unix::fs::symlink(out.path().join("secret.txt"), d.path().join("link.txt"))
            .unwrap();
        assert!(scan(d.path(), 20).unwrap().0.is_empty());
        let dir =
            cap_std::fs::Dir::open_ambient_dir(d.path(), cap_std::ambient_authority()).unwrap();
        assert!(read_document(&dir, Path::new("link.txt"), 100).is_err());
        assert!(read_document(&dir, Path::new("../secret.txt"), 100).is_err());
    }
    #[test]
    fn missing_folder_does_not_become_empty_manifest() {
        assert!(scan(Path::new("/nonexistent-hermes-folder"), 20).is_err());
    }
    #[test]
    fn deletions_are_opt_in_and_failed_replacements_are_retained() {
        let diff = Diff {
            modified: vec![Change {
                stale_file_id: Some("old-failed".into()),
                ..Default::default()
            }],
            deleted: vec![Change {
                file_id: Some("deleted".into()),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(cleanup_ids(&diff, &[], false).is_empty());
        assert_eq!(
            cleanup_ids(&diff, &["replaced-ok".into()], false),
            vec!["replaced-ok"]
        );
        assert_eq!(cleanup_ids(&diff, &[], true), vec!["deleted"]);
    }
    #[test]
    fn prevents_home_and_parent_selection() {
        let d = tempfile::tempdir().unwrap();
        let home = d.path().join("home");
        std::fs::create_dir(&home).unwrap();
        assert!(approved_root(d.path(), &home).is_err());
        assert!(approved_root(&home, &home).is_err());
        let docs = home.join("docs");
        std::fs::create_dir(&docs).unwrap();
        assert!(approved_root(&docs, &home).is_ok());
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use wiremock::{
        matchers::{body_json, body_string_contains, method, path},
        Mock, MockServer, ResponseTemplate,
    };
    fn channel() -> tauri::ipc::Channel<Value> {
        tauri::ipc::Channel::new(|_| Ok(()))
    }
    fn fixture(root: &Path, kb: &str, stale: bool) -> (Config, Plan) {
        std::fs::write(root.join("fact.md"), "test content").unwrap();
        let (manifest, _) = scan(root, 20).unwrap();
        let change = Change {
            filename: "fact.md".into(),
            stale_file_id: if stale {
                Some("11111111-1111-4111-8111-111111111111".into())
            } else {
                None
            },
            ..Default::default()
        };
        let diff = if stale {
            Diff {
                modified: vec![change],
                ..Default::default()
            }
        } else {
            Diff {
                added: vec![change],
                ..Default::default()
            }
        };
        let config = Config {
            folder: Some(root.into()),
            knowledge_id: Some(kb.into()),
            ..Default::default()
        };
        let plan = Plan {
            preview: Preview {
                id: "p".into(),
                added: usize::from(!stale),
                modified: usize::from(stale),
                unchanged: 0,
                skipped: 0,
                deletions: vec![],
                scanned: 1,
            },
            manifest,
            diff,
            folder: root.into(),
            knowledge: kb.into(),
            created: Instant::now(),
        };
        (config, plan)
    }
    async fn mock_upload(server: &MockServer, status: &str, hash: &str) {
        Mock::given(method("POST"))
            .and(path("/api/v1/files/"))
            .and(body_string_contains(hash))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"id":"22222222-2222-4222-8222-222222222222"})),
            )
            .expect(1)
            .mount(server)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/api/v1/files/22222222-2222-4222-8222-222222222222/process/status",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status":status})))
            .expect(1)
            .mount(server)
            .await;
    }
    #[tokio::test]
    async fn processes_new_version_before_cleaning_old() {
        let server = MockServer::start().await;
        let d = tempfile::tempdir().unwrap();
        let kb = "33333333-3333-4333-8333-333333333333";
        let (c, p) = fixture(d.path(), kb, true);
        mock_upload(&server, "completed", &p.manifest[0].checksum).await;
        Mock::given(method("POST"))
            .and(path(format!("/api/v1/knowledge/{kb}/sync/cleanup")))
            .and(body_json(
                json!({"file_ids":["11111111-1111-4111-8111-111111111111"],"dir_ids":[]}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status":true})))
            .expect(1)
            .mount(&server)
            .await;
        let r = apply(
            &Api::testing(server.uri(), "test".into()),
            &c,
            p,
            false,
            &channel(),
        )
        .await
        .unwrap();
        assert_eq!(r.modified, 1);
        assert_eq!(r.failed, 0);
    }
    #[tokio::test]
    async fn failed_processing_never_cleans_existing_index() {
        let server = MockServer::start().await;
        let d = tempfile::tempdir().unwrap();
        let kb = "33333333-3333-4333-8333-333333333333";
        let (c, p) = fixture(d.path(), kb, true);
        mock_upload(&server, "failed", &p.manifest[0].checksum).await;
        Mock::given(method("POST"))
            .and(path(format!("/api/v1/knowledge/{kb}/sync/cleanup")))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let r = apply(
            &Api::testing(server.uri(), "test".into()),
            &c,
            p,
            true,
            &channel(),
        )
        .await
        .unwrap();
        assert_eq!(r.failed, 1);
        assert_eq!(r.modified, 0);
    }
    #[tokio::test]
    async fn changed_file_after_preview_is_not_uploaded() {
        let server = MockServer::start().await;
        let d = tempfile::tempdir().unwrap();
        let (c, p) = fixture(d.path(), "33333333-3333-4333-8333-333333333333", true);
        std::fs::write(d.path().join("fact.md"), "new unreviewed content").unwrap();
        let r = apply(
            &Api::testing(server.uri(), "test".into()),
            &c,
            p,
            true,
            &channel(),
        )
        .await
        .unwrap();
        assert_eq!(r.failed, 1);
        assert!(server.received_requests().await.unwrap().is_empty());
    }
    #[tokio::test]
    async fn unchanged_diff_does_not_upload_or_cleanup() {
        let server = MockServer::start().await;
        let d = tempfile::tempdir().unwrap();
        let (c, mut p) = fixture(d.path(), "33333333-3333-4333-8333-333333333333", false);
        p.diff.added.clear();
        p.diff.unmodified_count = 1;
        let r = apply(
            &Api::testing(server.uri(), "test".into()),
            &c,
            p,
            false,
            &channel(),
        )
        .await
        .unwrap();
        assert_eq!(r.unchanged, 1);
        assert!(server.received_requests().await.unwrap().is_empty());
    }
    #[tokio::test]
    async fn expired_preview_cannot_mutate_backend() {
        let server = MockServer::start().await;
        let d = tempfile::tempdir().unwrap();
        let (c, mut p) = fixture(d.path(), "33333333-3333-4333-8333-333333333333", false);
        p.created = Instant::now() - Duration::from_secs(601);
        assert!(apply(
            &Api::testing(server.uri(), "test".into()),
            &c,
            p,
            true,
            &channel()
        )
        .await
        .is_err());
        assert!(server.received_requests().await.unwrap().is_empty());
    }
    #[tokio::test]
    #[ignore = "Requires isolated Open WebUI and native Ollama; run scripts/integration.mjs"]
    async fn live_backend_retrieval_and_incremental_sync() {
        let port: u16 = std::env::var("HERMES_TEST_PORT").unwrap().parse().unwrap();
        let api = Api::testing(
            format!("http://127.0.0.1:{port}"),
            std::env::var("HERMES_TEST_TOKEN").unwrap(),
        );
        let kb = std::env::var("HERMES_TEST_KB").unwrap();
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("hermes-facts.md");
        let facts = include_str!("../../../../tests/fixtures/hermes-facts.md");
        std::fs::write(&path, facts).unwrap();
        let c = Config {
            port,
            folder: Some(d.path().into()),
            knowledge_id: Some(kb),
            ..Default::default()
        };
        let p = preview(&api, &c).await.unwrap();
        assert_eq!(p.preview.added, 1);
        let r = apply(&api, &c, p, false, &channel()).await.unwrap();
        assert_eq!(r.failed, 0, "{:?}", r.errors);
        assert_eq!(r.added, 1);
        let p = preview(&api, &c).await.unwrap();
        assert_eq!(p.preview.unchanged, 1);
        assert_eq!(p.preview.added, 0);
        let answer = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let capture = answer.clone();
        let events = tauri::ipc::Channel::new(move |body| {
            if let tauri::ipc::InvokeResponseBody::Json(text) = body {
                let value: Value = serde_json::from_str(&text).unwrap();
                if let Some(s) = value["choices"][0]["delta"]["content"].as_str() {
                    capture.lock().unwrap().push_str(s);
                }
            }
            Ok(())
        });
        crate::api::stream_chat(&api,crate::api::chat_body(&c.model,&[crate::api::Message {role:"user".into(),content:"What is the departure code of the vessel Aster? Answer using the indexed document.".into()}],c.knowledge_id.as_deref()),tokio_util::sync::CancellationToken::new(),events).await.unwrap();
        assert!(
            answer.lock().unwrap().contains("AMBER-7319"),
            "Actual retrieved answer: {}",
            answer.lock().unwrap()
        );
        std::fs::write(
            &path,
            format!("{facts}\nThe second test fact is a silver lantern.\n"),
        )
        .unwrap();
        let p = preview(&api, &c).await.unwrap();
        assert_eq!(p.preview.modified, 1);
        let r = apply(&api, &c, p, false, &channel()).await.unwrap();
        assert_eq!(r.failed, 0);
        assert_eq!(r.modified, 1);
        std::fs::remove_file(&path).unwrap(); // Only our own tempfile fixture; never host documents.
        let p = preview(&api, &c).await.unwrap();
        assert_eq!(p.preview.deletions.len(), 1);
        let r = apply(&api, &c, p, false, &channel()).await.unwrap();
        assert_eq!(r.deleted, 0);
        assert_eq!(r.retained, 1);
        let p = preview(&api, &c).await.unwrap();
        let r = apply(&api, &c, p, true, &channel()).await.unwrap();
        assert_eq!(r.deleted, 1);
        assert!(preview(&api, &c)
            .await
            .unwrap()
            .preview
            .deletions
            .is_empty());
        println!("Verified real retrieval (AMBER-7319), unchanged/modified sync, retained deletion and explicit cleanup.");
    }
}
