use std::process::Stdio;
use tauri::AppHandle;
use tokio::time::{timeout, Duration};

use crate::engine;

use sha2::{Digest, Sha256};

#[cfg(target_os = "linux")]
const ASSET_NAME: &str = "yt-dlp_linux";
#[cfg(target_os = "windows")]
const ASSET_NAME: &str = "yt-dlp.exe";
#[cfg(target_os = "macos")]
const ASSET_NAME: &str = "yt-dlp_macos";

const SUMS_NAME: &str = "SHA2-256SUMS";

/// Parse a `sha256  filename` sums file and return the digest for `asset`.
fn find_checksum<'a>(sums: &'a str, asset: &str) -> Option<&'a str> {
    sums.lines().find_map(|line| {
        let (hash, name) = line.split_once(char::is_whitespace)?;
        // the filename column may be prefixed with `*` (binary mode)
        let name = name.trim().trim_start_matches('*');
        (name == asset && hash.len() == 64).then_some(hash)
    })
}

async fn run_version(path: &std::path::Path) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new(path);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);
    let out = timeout(Duration::from_secs(15), cmd.arg("--version").stdin(Stdio::null()).output())
        .await
        .map_err(|_| "yt-dlp --version timed out".to_string())?
        .map_err(|e| format!("failed to run yt-dlp: {e}"))?;
    if !out.status.success() {
        return Err("yt-dlp --version failed".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[tauri::command]
pub async fn get_ytdlp_version(app: AppHandle) -> Result<String, String> {
    run_version(&engine::ytdlp_path(&app)?).await
}

/// Downloads the latest yt-dlp release into the app data dir, where it
/// shadows the read-only bundled sidecar (see engine::ytdlp_path).
#[tauri::command]
pub async fn update_ytdlp(app: AppHandle) -> Result<String, String> {
    let base = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
    let url = format!("{base}/{ASSET_NAME}");
    let sums_url = format!("{base}/{SUMS_NAME}");
    let client = reqwest::Client::builder()
        .user_agent("youtube-downloader-app")
        .build()
        .map_err(|e| e.to_string())?;

    // Fetch the published checksums first — a release without them, or without
    // an entry for our asset, is not trustworthy to install.
    let sums = client
        .get(&sums_url)
        .send()
        .await
        .map_err(|e| format!("could not fetch checksums: {e}"))?
        .error_for_status()
        .map_err(|e| format!("could not fetch checksums: {e}"))?
        .text()
        .await
        .map_err(|e| format!("could not fetch checksums: {e}"))?;
    let expected = find_checksum(&sums, ASSET_NAME)
        .ok_or_else(|| format!("no checksum published for {ASSET_NAME}"))?
        .to_ascii_lowercase();

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("download failed: {e}"))?;
    let bytes = resp.bytes().await.map_err(|e| format!("download failed: {e}"))?;

    // Verify integrity/authenticity before the binary ever touches disk as an
    // executable. Mismatch => reject (MITM, corrupted download, tampered asset).
    let actual = hex::encode(Sha256::digest(&bytes));
    if actual != expected {
        return Err("downloaded yt-dlp failed checksum verification".into());
    }

    let dest = engine::updated_ytdlp_path(&app)?;
    let dir = dest.parent().ok_or("bad destination path")?;
    tokio::fs::create_dir_all(dir).await.map_err(|e| e.to_string())?;

    let staging = dest.with_extension("new");
    tokio::fs::write(&staging, &bytes).await.map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o700))
            .await
            .map_err(|e| e.to_string())?;
    }

    // verify the new binary actually runs before swapping it in
    let version = run_version(&staging).await.map_err(|e| {
        format!("downloaded binary failed verification: {e}")
    })?;
    tokio::fs::rename(&staging, &dest).await.map_err(|e| e.to_string())?;
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn find_checksum_matches_asset_line() {
        let sums = format!("{H}  yt-dlp_linux\n{} *yt-dlp.exe\n", &H.replace('a', "b"));
        assert_eq!(find_checksum(&sums, "yt-dlp_linux"), Some(H));
        // binary-mode `*` prefix on the filename is tolerated
        assert_eq!(find_checksum(&sums, "yt-dlp.exe"), Some(H.replace('a', "b").as_str()));
    }

    #[test]
    fn find_checksum_absent_or_malformed() {
        let sums = format!("{H}  yt-dlp_linux\n");
        assert_eq!(find_checksum(&sums, "yt-dlp_macos"), None);
        // wrong hash length is rejected even if the name matches
        assert_eq!(find_checksum("deadbeef  yt-dlp_linux", "yt-dlp_linux"), None);
        assert_eq!(find_checksum("", "yt-dlp_linux"), None);
    }
}
