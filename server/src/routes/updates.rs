use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use tokio_util::io::ReaderStream;

/// Directory where update artifacts are stored.
/// Configurable via `UPDATES_DIR` env var, defaults to `./updates`.
fn updates_dir() -> std::path::PathBuf {
    std::env::var("UPDATES_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("./updates"))
}

/// `latest.json` format (written manually when publishing a new release):
///
/// ```json
/// {
///   "version": "0.3.0",
///   "notes": "Описание обновления",
///   "pub_date": "2026-04-12T00:00:00Z",
///   "platforms": {
///     "windows-x86_64": {
///       "signature": "contents of .sig file",
///       "url": "https://your-server.com/api/updates/download/LeagueEye_0.3.0_x64-setup.exe"
///     }
///   }
/// }
/// ```
#[derive(serde::Deserialize)]
struct LatestRelease {
    version: String,
    notes: Option<String>,
    pub_date: Option<String>,
    platforms: std::collections::HashMap<String, PlatformEntry>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct PlatformEntry {
    signature: String,
    url: String,
}

/// Tauri updater endpoint.
///
/// GET /api/updates/{target}/{arch}/{current_version}
///
/// Returns JSON with update info if a newer version is available,
/// or 204 No Content if the client is up-to-date.
pub async fn check_update(
    Path((target, arch, current_version)): Path<(String, String, String)>,
) -> Response {
    let latest_path = updates_dir().join("latest.json");

    let data = match tokio::fs::read_to_string(&latest_path).await {
        Ok(d) => d,
        Err(e) => {
            log::warn!("[updates] Cannot read latest.json: {}", e);
            return StatusCode::NO_CONTENT.into_response();
        }
    };

    let release: LatestRelease = match serde_json::from_str(&data) {
        Ok(r) => r,
        Err(e) => {
            log::error!("[updates] Invalid latest.json: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Compare versions using semver
    let current = match semver::Version::parse(&current_version) {
        Ok(v) => v,
        Err(_) => {
            log::warn!("[updates] Cannot parse current_version '{}'", current_version);
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    let latest = match semver::Version::parse(&release.version) {
        Ok(v) => v,
        Err(_) => {
            log::error!("[updates] Cannot parse latest version '{}' from latest.json", release.version);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if current >= latest {
        return StatusCode::NO_CONTENT.into_response();
    }

    // Build the platform key Tauri uses: "{target}-{arch}"
    let platform_key = format!("{}-{}", target, arch);

    let platform = match release.platforms.get(&platform_key) {
        Some(p) => p,
        None => {
            log::info!("[updates] No platform entry for '{}', returning 204", platform_key);
            return StatusCode::NO_CONTENT.into_response();
        }
    };

    // Build response in Tauri updater format
    let response = serde_json::json!({
        "version": release.version,
        "notes": release.notes.unwrap_or_default(),
        "pub_date": release.pub_date.unwrap_or_default(),
        "platforms": {
            &platform_key: {
                "signature": platform.signature,
                "url": platform.url,
            }
        }
    });

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&response).unwrap_or_default(),
    )
        .into_response()
}

/// Serves update artifact files.
///
/// GET /api/updates/download/{filename}
pub async fn download_update(
    Path(filename): Path<String>,
) -> Response {
    // Prevent path traversal
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let file_path = updates_dir().join(&filename);

    let file = match tokio::fs::File::open(&file_path).await {
        Ok(f) => f,
        Err(_) => {
            return StatusCode::NOT_FOUND.into_response();
        }
    };

    let metadata = match file.metadata().await {
        Ok(m) => m,
        Err(_) => {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CONTENT_LENGTH, metadata.len().to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        body,
    )
        .into_response()
}
