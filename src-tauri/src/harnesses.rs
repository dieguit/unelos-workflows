use unelos_core::HarnessStatus;

fn sidecar_source() -> Result<camino::Utf8PathBuf, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let parent = executable.parent().ok_or("Executable has no parent")?;
    camino::Utf8PathBuf::from_path_buf(
        parent.join(format!("unelos{}", std::env::consts::EXE_SUFFIX)),
    )
    .map_err(|_| "Sidecar path must be UTF-8".to_string())
}

pub fn provision_bundled_cli() {
    let result = sidecar_source()
        .and_then(|source| unelos_core::provision_cli(&source).map_err(|e| e.to_string()));
    if let Err(error) = result {
        log::warn!("Could not provision bundled CLI: {error}");
    }
}

#[tauri::command]
pub async fn list_harnesses() -> Result<Vec<HarnessStatus>, String> {
    tauri::async_runtime::spawn_blocking(unelos_core::list_harnesses)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_harness(id: String) -> Result<Vec<HarnessStatus>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source = sidecar_source()?;
        unelos_core::install_harness(&id, &source).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn uninstall_harness(id: String) -> Result<Vec<HarnessStatus>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        unelos_core::uninstall_harness(&id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
