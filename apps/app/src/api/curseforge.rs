use super::Result;
use anyhow::{Context, bail};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Component, Path, PathBuf};
use tauri::{Manager, Runtime};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_http::reqwest;
use theseus::data::ProjectType;

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("curseforge")
        .invoke_handler(tauri::generate_handler![request, install, save_file])
        .build()
}

const PROXY_URL: &str = "http://88.86.222.162:4000";

fn client() -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .redirect(reqwest::redirect::Policy::none())
        .build()?)
}

async fn api(
    path: &str,
    query: &HashMap<String, String>,
) -> anyhow::Result<Value> {
    let response = client()?
        .get(format!("{PROXY_URL}/v1/{path}"))
        .query(query)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        bail!("CurseForge API: HTTP {status}");
    }
    Ok(serde_json::from_slice(&response.bytes().await?)?)
}

#[tauri::command]
async fn request(
    path: String,
    query: HashMap<String, String>,
) -> Result<Value> {
    if !(path == "categories"
        || path == "mods/search"
        || (path.starts_with("mods/")
            && path
                .split('/')
                .skip(1)
                .all(|p| p == "files" || p.parse::<u64>().is_ok())))
    {
        return Err(anyhow::anyhow!("Unsupported CurseForge endpoint").into());
    }
    Ok(api(&path, &query).await?)
}

fn safe_path(name: &str) -> anyhow::Result<&Path> {
    let path = Path::new(name);
    if name.is_empty()
        || name.contains(['\\', ':', '\0'])
        || !path.components().all(|c| matches!(c, Component::Normal(_)))
    {
        bail!("Unsafe archive path");
    }
    Ok(path)
}

async fn file(project: u64, id: u64) -> anyhow::Result<Value> {
    Ok(
        api(&format!("mods/{project}/files/{id}"), &HashMap::new())
            .await?["data"]
            .take(),
    )
}

#[tauri::command]
async fn save_file<R: Runtime>(
    app: tauri::AppHandle<R>,
    project_id: u64,
    file_id: u64,
) -> Result<bool> {
    let metadata = file(project_id, file_id).await?;
    let filename = metadata["fileName"].as_str().context("Missing filename")?;
    safe_path(filename)?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_file_name(filename)
        .save_file(|path| {
            let _ = tx.send(path);
        });
    let Some(destination) = rx.await.unwrap_or(None) else {
        return Ok(false);
    };
    let destination = destination
        .into_path()
        .map_err(|_| anyhow::anyhow!("Invalid destination"))?;
    let temporary = tempfile::tempdir()?;
    let source = download(&metadata, temporary.path()).await?;
    tokio::fs::copy(source, destination).await?;
    Ok(true)
}

async fn download(file: &Value, directory: &Path) -> anyhow::Result<PathBuf> {
    let name = file["fileName"].as_str().context("Missing filename")?;
    let relative = safe_path(name)?;
    if relative.components().count() != 1 {
        bail!("Invalid filename");
    }
    if file["downloadUrl"].as_str().is_none() {
        bail!(
            "The author does not allow downloads through third-party launchers"
        );
    }
    let project = file["modId"].as_u64().context("Missing project ID")?;
    let id = file["id"].as_u64().context("Missing file ID")?;
    let url = format!("{PROXY_URL}/downloads/{project}/{id}");
    let mut response = client()?.get(url).send().await?.error_for_status()?;
    tokio::fs::create_dir_all(directory).await?;
    let path = directory.join(relative);
    let mut output = tokio::fs::File::create(&path).await?;
    let mut size = 0u64;
    let mut hash = Sha1::new();
    while let Some(chunk) = response.chunk().await? {
        size += chunk.len() as u64;
        hash.update(&chunk);
        if size > file["fileLength"].as_u64().context("Missing file length")? {
            bail!("Download exceeds expected size");
        }
        tokio::io::AsyncWriteExt::write_all(&mut output, &chunk).await?;
    }
    drop(output);
    if Some(size) != file["fileLength"].as_u64() {
        bail!("Incomplete CurseForge download");
    }
    let expected = file["hashes"]
        .as_array()
        .and_then(|hashes| hashes.iter().find(|h| h["algo"] == 1))
        .and_then(|h| h["value"].as_str())
        .context("Missing SHA-1 checksum")?;
    let actual = hash
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if !actual.eq_ignore_ascii_case(expected) {
        bail!("CurseForge checksum mismatch");
    }
    Ok(path)
}

async fn project_type(
    project: u64,
    categories: &[Value],
) -> anyhow::Result<(String, Option<ProjectType>)> {
    let project = api(&format!("mods/{project}"), &HashMap::new()).await?;
    let class = categories
        .iter()
        .find(|c| c["id"] == project["data"]["classId"])
        .context("Unknown content class")?;
    let slug = class["slug"].as_str().context("Missing content class")?;
    Ok(match slug {
        "mc-mods" => ("mods".into(), Some(ProjectType::Mod)),
        "texture-packs" => {
            ("resourcepacks".into(), Some(ProjectType::ResourcePack))
        }
        "shaders" => ("shaderpacks".into(), Some(ProjectType::ShaderPack)),
        "data-packs" => ("datapacks".into(), Some(ProjectType::DataPack)),
        "modpacks" => ("modpacks".into(), None),
        _ => {
            bail!("This content type cannot be installed in a Minecraft client")
        }
    })
}

#[tauri::command]
async fn install<R: Runtime>(
    app: tauri::AppHandle<R>,
    project_id: u64,
    file_id: u64,
    instance_id: Option<String>,
) -> Result<Value> {
    Ok(install_inner(app, project_id, file_id, instance_id).await?)
}

async fn install_inner<R: Runtime>(
    app: tauri::AppHandle<R>,
    project_id: u64,
    file_id: u64,
    instance_id: Option<String>,
) -> anyhow::Result<Value> {
    let categories = api(
        "categories",
        &HashMap::from([("gameId".into(), "432".into())]),
    )
    .await?;
    let categories = categories["data"]
        .as_array()
        .context("Missing categories")?;
    let (folder, kind) = project_type(project_id, categories).await?;
    let selected = file(project_id, file_id).await?;
    let temp = tempfile::tempdir()?;
    if folder == "modpacks" {
        let archive = download(&selected, temp.path()).await?;
        let mut archive = zip::ZipArchive::new(std::fs::File::open(archive)?)?;
        let manifest: Value =
            serde_json::from_reader(archive.by_name("manifest.json")?)?;
        let mut dependencies = serde_json::Map::new();
        dependencies.insert(
            "minecraft".into(),
            manifest["minecraft"]["version"].clone(),
        );
        if let Some(loaders) = manifest["minecraft"]["modLoaders"].as_array() {
            if let Some(loader) = loaders
                .iter()
                .find(|l| l["primary"] == true)
                .or_else(|| loaders.first())
            {
                let id = loader["id"].as_str().context("Missing mod loader")?;
                let (loader, version) =
                    id.split_once('-').context("Invalid mod loader")?;
                let name = match loader {
                    "forge" => "forge",
                    "neoforge" => "neoforge",
                    "fabric" => "fabric-loader",
                    "quilt" => "quilt-loader",
                    _ => bail!("Unsupported mod loader"),
                };
                dependencies.insert(name.into(), json!(version));
            }
        }
        let output_dir = app.path().app_cache_dir()?.join("curseforge");
        tokio::fs::create_dir_all(&output_dir).await?;
        let output_path =
            output_dir.join(format!("{}.mrpack", uuid::Uuid::new_v4()));
        let mut temporary = tempfile::NamedTempFile::new_in(&output_dir)?;
        let mut output = zip::ZipWriter::new(temporary.as_file_mut());
        let options = zip::write::SimpleFileOptions::default();
        output.start_file("modrinth.index.json", options)?;
        serde_json::to_writer(
            &mut output,
            &json!({"game":"minecraft", "formatVersion":1, "versionId": file_id.to_string(), "name":manifest["name"], "files":[], "dependencies":dependencies}),
        )?;
        let overrides = manifest["overrides"].as_str().unwrap_or("overrides");
        safe_path(overrides)?;
        let prefix = format!("{overrides}/");
        let mut paths = HashSet::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            if entry.is_dir() {
                continue;
            }
            let Some(relative) =
                entry.name()?.strip_prefix(&prefix).map(str::to_string)
            else {
                continue;
            };
            safe_path(&relative)?;
            if entry.size() > 512 * 1024 * 1024 {
                bail!("Override file too large");
            }
            paths.insert(relative.clone());
            output.start_file(format!("overrides/{relative}"), options)?;
            std::io::copy(&mut entry, &mut output)?;
        }
        for item in manifest["files"]
            .as_array()
            .context("Missing modpack files")?
        {
            if item["required"] == false {
                continue;
            }
            let project =
                item["projectID"].as_u64().context("Missing project ID")?;
            let id = item["fileID"].as_u64().context("Missing file ID")?;
            let metadata = file(project, id).await?;
            let (folder, kind) = project_type(project, categories).await?;
            if kind.is_none() {
                bail!("Nested modpacks are unsupported");
            }
            let path = download(&metadata, temp.path()).await?;
            let relative = format!(
                "{folder}/{}",
                metadata["fileName"].as_str().context("Missing filename")?
            );
            if !paths.insert(relative.clone()) {
                bail!("Duplicate modpack file: {relative}");
            }
            output.start_file(format!("overrides/{relative}"), options)?;
            std::io::copy(&mut std::fs::File::open(path)?, &mut output)?;
        }
        output.finish()?;
        temporary.persist(&output_path)?;
        return Ok(json!({"packPath":output_path}));
    }
    let instance_id = instance_id.context("Select an instance first")?;
    let instance = theseus::instance::get(&instance_id)
        .await?
        .context("Instance not found")?;
    let version = instance.applied_content_set.game_version.to_string();
    let loader = serde_json::to_value(instance.applied_content_set.loader)?;
    let loader = match loader.as_str().unwrap_or_default() {
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    };
    let mut pending = VecDeque::from([(project_id, selected, kind)]);
    let mut seen = HashSet::new();
    let mut staged = Vec::new();
    while let Some((project, metadata, kind)) = pending.pop_front() {
        if !seen.insert(project) {
            continue;
        }
        if !metadata["gameVersions"].as_array().is_some_and(|v| {
            v.iter().any(|v| v.as_str() == Some(version.as_str()))
        }) {
            bail!("File is incompatible with Minecraft {version}");
        }
        if matches!(kind, Some(ProjectType::Mod)) {
            let loader_name = match loader {
                Some(1) => "Forge",
                Some(4) => "Fabric",
                Some(5) => "Quilt",
                Some(6) => "NeoForge",
                _ => bail!("This instance needs a mod loader"),
            };
            if !metadata["gameVersions"].as_array().is_some_and(|v| {
                v.iter().any(|v| v.as_str() == Some(loader_name))
            }) {
                bail!("File is incompatible with {loader_name}");
            }
        }
        let path =
            download(&metadata, &temp.path().join(project.to_string())).await?;
        staged.push((path, kind));
        if let Some(dependencies) = metadata["dependencies"].as_array() {
            for dependency in
                dependencies.iter().filter(|d| d["relationType"] == 3)
            {
                let id = dependency["modId"]
                    .as_u64()
                    .context("Missing dependency ID")?;
                if seen.contains(&id) {
                    continue;
                }
                let (_, kind) = project_type(id, categories).await?;
                let mut query = HashMap::from([
                    ("gameVersion".into(), version.clone()),
                    ("pageSize".into(), "50".into()),
                ]);
                if matches!(kind, Some(ProjectType::Mod)) {
                    if let Some(loader) = loader {
                        query
                            .insert("modLoaderType".into(), loader.to_string());
                    }
                }
                let files = api(&format!("mods/{id}/files"), &query).await?;
                let file = files["data"]
                    .as_array()
                    .and_then(|v| v.first())
                    .context("No compatible required dependency")?
                    .clone();
                pending.push_back((id, file, kind));
            }
        }
    }
    for (path, kind) in staged {
        theseus::instance::add_project_from_path(&instance_id, &path, kind)
            .await?;
    }
    Ok(json!({"instanceId":instance_id}))
}

#[cfg(test)]
mod tests {
    use super::safe_path;

    #[test]
    fn rejects_paths_outside_the_download_directory() {
        for path in [
            "",
            "../outside.jar",
            "/outside.jar",
            "C:/outside.jar",
            "a/../../outside.jar",
            "a\\outside.jar",
            "a\0.jar",
        ] {
            assert!(safe_path(path).is_err(), "accepted {path:?}");
        }
    }

    #[test]
    fn accepts_nested_override_paths() {
        assert!(safe_path("config/example/settings.json").is_ok());
        assert!(safe_path("example-1.0.jar").is_ok());
    }
}
