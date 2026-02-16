use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::constants::KIWI_RELEASES_API_BASE;
use crate::error::{KiwiError, Result};

#[derive(Debug, Clone)]
pub(crate) struct PreparedAssets {
    pub(crate) tag_name: String,
    pub(crate) cache_dir: PathBuf,
    pub(crate) library_path: PathBuf,
    pub(crate) model_path: PathBuf,
}

pub(crate) fn prepare_assets(version: &str) -> Result<PreparedAssets> {
    let release_json = fetch_release_metadata(version)?;
    let tag_name = extract_json_string_field(&release_json, "tag_name").ok_or_else(|| {
        KiwiError::Bootstrap(
            "could not parse release tag_name from GitHub API response".to_string(),
        )
    })?;
    let version_no_v = tag_name.trim_start_matches('v');
    if version_no_v.is_empty() {
        return Err(KiwiError::Bootstrap(format!(
            "resolved invalid release tag: {tag_name}"
        )));
    }

    let cache_dir = resolve_cache_root()?.join("kiwi-rs").join(version_no_v);
    fs::create_dir_all(&cache_dir).map_err(|error| {
        KiwiError::Bootstrap(format!(
            "failed to create cache directory {}: {}",
            cache_dir.display(),
            error
        ))
    })?;

    let library_path = cache_dir.join("lib").join(platform_library_filename());
    let model_path = cache_dir.join("models").join("cong").join("base");

    if library_path.exists() && model_path.exists() {
        return Ok(PreparedAssets {
            tag_name,
            cache_dir,
            library_path,
            model_path,
        });
    }

    let download_dir = cache_dir.join("downloads");
    fs::create_dir_all(&download_dir).map_err(|error| {
        KiwiError::Bootstrap(format!(
            "failed to create download directory {}: {}",
            download_dir.display(),
            error
        ))
    })?;

    let lib_asset_name = platform_library_asset_name(version_no_v)?;
    let model_asset_name = format!("kiwi_model_v{version_no_v}_base.tgz");

    let lib_archive = download_dir.join(&lib_asset_name);
    let model_archive = download_dir.join(&model_asset_name);

    download_release_asset(&release_json, &lib_asset_name, &lib_archive)?;
    download_release_asset(&release_json, &model_asset_name, &model_archive)?;

    extract_archive(&lib_archive, &cache_dir)?;
    extract_tgz_archive(&model_archive, &cache_dir)?;

    if !library_path.exists() {
        return Err(KiwiError::Bootstrap(format!(
            "library file was not found after extraction: {}",
            library_path.display()
        )));
    }
    if !model_path.exists() {
        return Err(KiwiError::Bootstrap(format!(
            "model directory was not found after extraction: {}",
            model_path.display()
        )));
    }

    Ok(PreparedAssets {
        tag_name,
        cache_dir,
        library_path,
        model_path,
    })
}

fn fetch_release_metadata(version: &str) -> Result<String> {
    let normalized = if version.eq_ignore_ascii_case("latest") {
        "latest".to_string()
    } else if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{version}")
    };

    let url = if normalized == "latest" {
        format!("{KIWI_RELEASES_API_BASE}/latest")
    } else {
        format!("{KIWI_RELEASES_API_BASE}/tags/{normalized}")
    };

    let output = Command::new("curl")
        .arg("-fsSL")
        .arg(&url)
        .output()
        .map_err(|error| {
            KiwiError::Bootstrap(format!(
                "failed to execute curl for release metadata (url={url}): {error}"
            ))
        })?;

    if !output.status.success() {
        return Err(KiwiError::Bootstrap(format!(
            "curl failed while fetching release metadata (url={url}): {}",
            command_stderr(&output)
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn download_release_asset(release_json: &str, asset_name: &str, output_path: &Path) -> Result<()> {
    if output_path.exists() {
        return Ok(());
    }

    let asset_url = find_asset_url(release_json, asset_name).ok_or_else(|| {
        KiwiError::Bootstrap(format!(
            "release asset not found for current tag: {asset_name}"
        ))
    })?;

    let output = Command::new("curl")
        .arg("-fL")
        .arg("--retry")
        .arg("3")
        .arg("--retry-delay")
        .arg("1")
        .arg("-o")
        .arg(output_path)
        .arg(&asset_url)
        .output()
        .map_err(|error| {
            KiwiError::Bootstrap(format!(
                "failed to execute curl for asset download (asset={asset_name}): {error}"
            ))
        })?;

    if !output.status.success() {
        return Err(KiwiError::Bootstrap(format!(
            "curl failed while downloading asset {asset_name}: {}",
            command_stderr(&output)
        )));
    }

    Ok(())
}

fn extract_archive(archive: &Path, output_dir: &Path) -> Result<()> {
    let archive_name = archive
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            KiwiError::Bootstrap(format!("invalid archive path: {}", archive.display()))
        })?;

    if archive_name.ends_with(".tgz") || archive_name.ends_with(".tar.gz") {
        return extract_tgz_archive(archive, output_dir);
    }

    if archive_name.ends_with(".zip") {
        return extract_zip_archive(archive, output_dir);
    }

    Err(KiwiError::Bootstrap(format!(
        "unsupported archive type: {}",
        archive.display()
    )))
}

fn extract_tgz_archive(archive: &Path, output_dir: &Path) -> Result<()> {
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("-C")
        .arg(output_dir)
        .output()
        .map_err(|error| {
            KiwiError::Bootstrap(format!(
                "failed to execute tar for {}: {}",
                archive.display(),
                error
            ))
        })?;

    if !output.status.success() {
        return Err(KiwiError::Bootstrap(format!(
            "tar extraction failed for {}: {}",
            archive.display(),
            command_stderr(&output)
        )));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn extract_zip_archive(archive: &Path, output_dir: &Path) -> Result<()> {
    let script = format!(
        "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
        archive.display(),
        output_dir.display()
    );
    let output = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(script)
        .output()
        .map_err(|error| {
            KiwiError::Bootstrap(format!(
                "failed to execute PowerShell for zip extraction {}: {}",
                archive.display(),
                error
            ))
        })?;

    if !output.status.success() {
        return Err(KiwiError::Bootstrap(format!(
            "zip extraction failed for {}: {}",
            archive.display(),
            command_stderr(&output)
        )));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn extract_zip_archive(archive: &Path, _output_dir: &Path) -> Result<()> {
    Err(KiwiError::Bootstrap(format!(
        "zip extraction is only supported on Windows in kiwi-rs bootstrap: {}",
        archive.display()
    )))
}

fn command_stderr(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        format!("process exited with status {}", output.status)
    } else {
        stderr
    }
}

fn resolve_cache_root() -> Result<PathBuf> {
    if let Some(path) = env::var_os("KIWI_RS_CACHE_DIR") {
        return Ok(PathBuf::from(path));
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(path) = env::var_os("LOCALAPPDATA") {
            return Ok(PathBuf::from(path));
        }
        if let Some(home) = env::var_os("USERPROFILE") {
            return Ok(PathBuf::from(home).join("AppData").join("Local"));
        }
        return Err(KiwiError::Bootstrap(
            "failed to resolve cache directory on Windows. Set KIWI_RS_CACHE_DIR.".to_string(),
        ));
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = env::var_os("HOME") {
            return Ok(PathBuf::from(home).join("Library").join("Caches"));
        }
        return Err(KiwiError::Bootstrap(
            "failed to resolve cache directory on macOS. Set KIWI_RS_CACHE_DIR.".to_string(),
        ));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(path) = env::var_os("XDG_CACHE_HOME") {
            return Ok(PathBuf::from(path));
        }
        if let Some(home) = env::var_os("HOME") {
            return Ok(PathBuf::from(home).join(".cache"));
        }
        return Err(KiwiError::Bootstrap(
            "failed to resolve cache directory on Unix. Set KIWI_RS_CACHE_DIR.".to_string(),
        ));
    }

    #[allow(unreachable_code)]
    Err(KiwiError::Bootstrap(
        "failed to resolve cache directory on this platform. Set KIWI_RS_CACHE_DIR.".to_string(),
    ))
}

pub(crate) fn extract_json_string_field(haystack: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let start = haystack.find(&key)?;
    let mut index = start + key.len();

    index += haystack[index..].find(':')? + 1;
    let bytes = haystack.as_bytes();

    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    if index >= bytes.len() || bytes[index] != b'"' {
        return None;
    }
    index += 1;

    let mut out = String::new();
    let mut escaped = false;
    while index < bytes.len() {
        let ch = bytes[index];
        index += 1;

        if escaped {
            let decoded = match ch {
                b'"' => '"',
                b'\\' => '\\',
                b'/' => '/',
                b'b' => '\u{0008}',
                b'f' => '\u{000c}',
                b'n' => '\n',
                b'r' => '\r',
                b't' => '\t',
                _ => ch as char,
            };
            out.push(decoded);
            escaped = false;
            continue;
        }

        if ch == b'\\' {
            escaped = true;
            continue;
        }
        if ch == b'"' {
            return Some(out);
        }
        out.push(ch as char);
    }
    None
}

pub(crate) fn find_asset_url(release_json: &str, asset_name: &str) -> Option<String> {
    let needle = format!("\"{asset_name}\"");
    let mut search_from = 0;

    while let Some(found) = release_json[search_from..].find(&needle) {
        let absolute = search_from + found;
        let start = release_json[..absolute].rfind('{')?;
        let end = absolute + release_json[absolute..].find('}')? + 1;
        let object = &release_json[start..end];

        if let Some(url) = extract_json_string_field(object, "browser_download_url") {
            return Some(url);
        }

        search_from = absolute + needle.len();
    }
    None
}

fn platform_library_asset_name(version_no_v: &str) -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        return match env::consts::ARCH {
            "aarch64" => Ok(format!("kiwi_mac_arm64_v{version_no_v}.tgz")),
            "x86_64" => Ok(format!("kiwi_mac_x86_64_v{version_no_v}.tgz")),
            arch => Err(KiwiError::Bootstrap(format!(
                "unsupported macOS architecture for auto-download: {arch}"
            ))),
        };
    }

    #[cfg(target_os = "linux")]
    {
        return match env::consts::ARCH {
            "x86_64" => Ok(format!("kiwi_lnx_x86_64_v{version_no_v}.tgz")),
            "aarch64" => Ok(format!("kiwi_lnx_aarch64_v{version_no_v}.tgz")),
            "powerpc64" | "powerpc64le" => Ok(format!("kiwi_lnx_ppc64le_v{version_no_v}.tgz")),
            arch => Err(KiwiError::Bootstrap(format!(
                "unsupported Linux architecture for auto-download: {arch}"
            ))),
        };
    }

    #[cfg(target_os = "windows")]
    {
        return match env::consts::ARCH {
            "x86_64" => Ok(format!("kiwi_win_x64_v{version_no_v}.zip")),
            "x86" | "i686" => Ok(format!("kiwi_win_Win32_v{version_no_v}.zip")),
            arch => Err(KiwiError::Bootstrap(format!(
                "unsupported Windows architecture for auto-download: {arch}"
            ))),
        };
    }

    #[allow(unreachable_code)]
    Err(KiwiError::Bootstrap(
        "unsupported target OS for auto-download".to_string(),
    ))
}

fn platform_library_filename() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "kiwi.dll"
    }
    #[cfg(target_os = "macos")]
    {
        "libkiwi.dylib"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "libkiwi.so"
    }
}
