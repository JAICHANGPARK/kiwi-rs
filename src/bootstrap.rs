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

#[cfg(test)]
mod bootstrap_tests {
    use super::{
        command_stderr, download_release_asset, extract_archive, extract_json_string_field,
        extract_tgz_archive, fetch_release_metadata, find_asset_url, platform_library_asset_name,
        platform_library_filename, prepare_assets, resolve_cache_root,
    };
    use crate::test_support::with_env_vars;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::{ExitStatus, Output};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn output_with(stderr: &[u8], exit_code: i32) -> Output {
        Output {
            status: status_with_code(exit_code),
            stdout: Vec::new(),
            stderr: stderr.to_vec(),
        }
    }

    #[cfg(unix)]
    fn status_with_code(exit_code: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(exit_code << 8)
    }

    #[cfg(windows)]
    fn status_with_code(exit_code: i32) -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(exit_code as u32)
    }

    fn make_temp_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("kiwi-rs-bootstrap-{name}-{suffix}"));
        fs::create_dir_all(&path).expect("failed to create temp dir");
        path
    }

    fn remove_tree(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    #[cfg(unix)]
    fn write_executable(path: &Path, body: &str) {
        fs::write(path, body).expect("failed to write script");
        let mut perms = fs::metadata(path)
            .expect("failed to read script metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("failed to set script mode");
    }

    #[cfg(unix)]
    fn install_fake_tools(root: &Path) -> PathBuf {
        let bin = root.join("bin");
        fs::create_dir_all(&bin).expect("failed to create bin dir");
        write_executable(
            &bin.join("curl"),
            r#"#!/bin/sh
set -eu
last=""
out=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "-o" ]; then
    out="$arg"
  fi
  prev="$arg"
  last="$arg"
done
if [ -n "${FAKE_CURL_LOG:-}" ]; then
  printf '%s\n' "$last" >> "$FAKE_CURL_LOG"
fi
if [ "${FAKE_CURL_FAIL:-0}" = "1" ]; then
  printf 'forced curl failure\n' >&2
  exit 22
fi
if [ -n "$out" ]; then
  mkdir -p "$(dirname "$out")"
  printf 'archive for %s\n' "$last" > "$out"
  exit 0
fi
if [ "${FAKE_CURL_BAD_TAG:-0}" = "1" ]; then
  printf '{"tag_name":"v","assets":[]}'
  exit 0
fi
lib="${FAKE_LIB_ASSET_NAME:-lib.tgz}"
model="${FAKE_MODEL_ASSET_NAME:-model.tgz}"
tag="${FAKE_RELEASE_TAG:-v9.9.9}"
printf '{"tag_name":"%s","assets":[{"name":"%s","browser_download_url":"https://example/%s"},{"name":"%s","browser_download_url":"https://example/%s"}]}' "$tag" "$lib" "$lib" "$model" "$model"
"#,
        );
        write_executable(
            &bin.join("tar"),
            r#"#!/bin/sh
set -eu
archive=""
outdir=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "-xzf" ]; then
    archive="$arg"
  fi
  if [ "$prev" = "-C" ]; then
    outdir="$arg"
  fi
  prev="$arg"
done
if [ "${FAKE_TAR_FAIL:-0}" = "1" ]; then
  printf 'forced tar failure\n' >&2
  exit 9
fi
mkdir -p "$outdir"
case "$archive" in
  *model*)
    mkdir -p "$outdir/models/cong/base"
    printf 'ok\n' > "$outdir/models/cong/base/model.ok"
    ;;
  *)
    mkdir -p "$outdir/lib"
    if [ "${FAKE_SKIP_LIB_FILE:-0}" = "1" ]; then
      exit 0
    fi
    : > "$outdir/lib/${FAKE_LIBRARY_FILENAME:-libkiwi.dylib}"
    ;;
esac
"#,
        );
        bin
    }

    #[cfg(unix)]
    fn with_fake_tools_env<T>(
        root: &Path,
        overrides: &[(&str, Option<&str>)],
        f: impl FnOnce() -> T,
    ) -> T {
        let bin = install_fake_tools(root);
        let inherited_path = std::env::var("PATH").unwrap_or_default();
        let path = format!("{}:{inherited_path}", bin.display());

        let mut env_overrides: Vec<(&str, Option<&str>)> = vec![("PATH", Some(path.as_str()))];
        env_overrides.extend_from_slice(overrides);
        with_env_vars(&env_overrides, f)
    }

    #[test]
    fn extract_json_string_field_handles_basic_and_escaped_values() {
        let json = r#"{"name":"kiwi","message":"line\n\"quoted\"","num":3}"#;
        assert_eq!(
            extract_json_string_field(json, "name").as_deref(),
            Some("kiwi")
        );
        assert_eq!(
            extract_json_string_field(json, "message").as_deref(),
            Some("line\n\"quoted\"")
        );
        assert!(extract_json_string_field(json, "num").is_none());
    }

    #[test]
    fn extract_json_string_field_returns_none_for_missing_or_unclosed_values() {
        assert!(extract_json_string_field("{}", "tag_name").is_none());
        assert!(extract_json_string_field(r#"{"tag_name":"v0.1"#, "tag_name").is_none());
    }

    #[test]
    fn find_asset_url_returns_expected_url() {
        let json = r#"{
            "assets": [
                {"name":"a.tgz","browser_download_url":"https://example/a.tgz"},
                {"name":"b.tgz","browser_download_url":"https://example/b.tgz"}
            ]
        }"#;
        assert_eq!(
            find_asset_url(json, "b.tgz").as_deref(),
            Some("https://example/b.tgz")
        );
    }

    #[test]
    fn find_asset_url_returns_none_when_url_field_missing() {
        let json = r#"{"assets":[{"name":"a.tgz"}]}"#;
        assert!(find_asset_url(json, "a.tgz").is_none());
    }

    #[test]
    fn command_stderr_prefers_trimmed_stderr_text() {
        let output = output_with(b"  failure details \n", 2);
        assert_eq!(command_stderr(&output), "failure details");
    }

    #[test]
    fn command_stderr_falls_back_to_exit_status_when_stderr_is_empty() {
        let output = output_with(b"   \n\t", 5);
        assert!(command_stderr(&output).starts_with("process exited with status"));
    }

    #[test]
    fn resolve_cache_root_prefers_env_override() {
        with_env_vars(
            &[
                ("KIWI_RS_CACHE_DIR", Some("/tmp/kiwi-rs-custom-cache")),
                ("XDG_CACHE_HOME", None),
                ("HOME", None),
                ("LOCALAPPDATA", None),
                ("USERPROFILE", None),
            ],
            || {
                let cache = resolve_cache_root().expect("cache path should resolve");
                assert_eq!(cache, Path::new("/tmp/kiwi-rs-custom-cache"));
            },
        );
    }

    #[test]
    fn platform_library_filename_matches_target() {
        #[cfg(target_os = "windows")]
        assert_eq!(platform_library_filename(), "kiwi.dll");
        #[cfg(target_os = "macos")]
        assert_eq!(platform_library_filename(), "libkiwi.dylib");
        #[cfg(all(unix, not(target_os = "macos")))]
        assert_eq!(platform_library_filename(), "libkiwi.so");
    }

    #[test]
    fn platform_library_asset_name_uses_target_pattern() {
        let asset = platform_library_asset_name("0.22.2").expect("asset name should be supported");

        #[cfg(target_os = "windows")]
        assert!(asset.starts_with("kiwi_win_") && asset.ends_with("_v0.22.2.zip"));
        #[cfg(target_os = "macos")]
        assert!(asset.starts_with("kiwi_mac_") && asset.ends_with("_v0.22.2.tgz"));
        #[cfg(target_os = "linux")]
        assert!(asset.starts_with("kiwi_lnx_") && asset.ends_with("_v0.22.2.tgz"));
    }

    #[test]
    fn extract_archive_rejects_unknown_extension() {
        let result = extract_archive(Path::new("archive.unknown"), Path::new("/tmp"));
        assert!(result.is_err());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn extract_archive_zip_is_not_supported_on_non_windows() {
        let result = extract_archive(Path::new("archive.zip"), Path::new("/tmp"));
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn fetch_release_metadata_normalizes_requested_version() {
        let root = make_temp_dir("fetch-metadata");
        let log = root.join("curl.log");
        let log_path = log.to_str().expect("temp path should be utf-8");

        with_fake_tools_env(
            &root,
            &[
                ("FAKE_CURL_LOG", Some(log_path)),
                ("FAKE_RELEASE_TAG", Some("v0.9.9")),
            ],
            || {
                let latest = fetch_release_metadata("latest").expect("latest fetch should succeed");
                assert!(latest.contains("\"tag_name\":\"v0.9.9\""));

                let no_v =
                    fetch_release_metadata("0.22.2").expect("non-prefixed tag fetch should work");
                assert!(no_v.contains("\"tag_name\":\"v0.9.9\""));

                let with_v =
                    fetch_release_metadata("v0.22.2").expect("prefixed tag fetch should work");
                assert!(with_v.contains("\"tag_name\":\"v0.9.9\""));
            },
        );

        let logged = fs::read_to_string(&log).expect("failed to read curl log");
        assert!(logged.contains("/latest"));
        assert!(logged.contains("/tags/v0.22.2"));

        remove_tree(&root);
    }

    #[cfg(unix)]
    #[test]
    fn fetch_release_metadata_propagates_curl_failure() {
        let root = make_temp_dir("fetch-metadata-failure");
        with_fake_tools_env(&root, &[("FAKE_CURL_FAIL", Some("1"))], || {
            let err = fetch_release_metadata("latest").expect_err("curl failure should bubble up");
            assert!(err
                .to_string()
                .contains("curl failed while fetching release metadata"));
        });
        remove_tree(&root);
    }

    #[cfg(unix)]
    #[test]
    fn download_release_asset_skips_existing_file() {
        let root = make_temp_dir("download-skip");
        let output = root.join("existing.tgz");
        fs::write(&output, b"already here").expect("failed to seed archive");

        with_fake_tools_env(&root, &[("FAKE_CURL_FAIL", Some("1"))], || {
            download_release_asset("{}", "ignored", &output).expect("existing file should skip");
        });

        let content = fs::read(&output).expect("failed to read file");
        assert_eq!(content, b"already here");
        remove_tree(&root);
    }

    #[cfg(unix)]
    #[test]
    fn download_release_asset_uses_resolved_asset_url() {
        let root = make_temp_dir("download-ok");
        let output = root.join("downloaded.tgz");
        let release_json = r#"{
            "assets": [
                {"name":"target.tgz","browser_download_url":"https://example/target.tgz"}
            ]
        }"#;

        with_fake_tools_env(&root, &[], || {
            download_release_asset(release_json, "target.tgz", &output)
                .expect("download should succeed with fake curl");
        });

        let content = fs::read_to_string(&output).expect("failed to read downloaded file");
        assert!(content.contains("https://example/target.tgz"));
        remove_tree(&root);
    }

    #[cfg(unix)]
    #[test]
    fn download_release_asset_errors_when_asset_is_missing() {
        let root = make_temp_dir("download-missing");
        let output = root.join("missing.tgz");

        with_fake_tools_env(&root, &[], || {
            let err = download_release_asset(r#"{"assets":[]}"#, "target.tgz", &output)
                .expect_err("missing asset should error");
            assert!(err.to_string().contains("release asset not found"));
        });
        remove_tree(&root);
    }

    #[cfg(unix)]
    #[test]
    fn extract_tgz_archive_propagates_tar_failure() {
        let root = make_temp_dir("extract-failure");
        let archive = root.join("archive.tgz");
        fs::write(&archive, b"dummy").expect("failed to write archive");

        with_fake_tools_env(&root, &[("FAKE_TAR_FAIL", Some("1"))], || {
            let err = extract_tgz_archive(&archive, &root).expect_err("tar failure should error");
            assert!(err.to_string().contains("tar extraction failed"));
        });
        remove_tree(&root);
    }

    #[cfg(unix)]
    #[test]
    fn prepare_assets_downloads_and_reuses_cache() {
        let root = make_temp_dir("prepare-assets-success");
        let cache_root = root.join("cache");
        let version = "9.9.9";
        let tag = format!("v{version}");
        let lib_asset = platform_library_asset_name(version).expect("platform should be supported");
        let model_asset = format!("kiwi_model_v{version}_base.tgz");
        let library_filename = platform_library_filename();
        let cache_root_str = cache_root.to_str().expect("temp path should be utf-8");

        let prepared = with_fake_tools_env(
            &root,
            &[
                ("KIWI_RS_CACHE_DIR", Some(cache_root_str)),
                ("FAKE_RELEASE_TAG", Some(tag.as_str())),
                ("FAKE_LIB_ASSET_NAME", Some(lib_asset.as_str())),
                ("FAKE_MODEL_ASSET_NAME", Some(model_asset.as_str())),
                ("FAKE_LIBRARY_FILENAME", Some(library_filename)),
            ],
            || prepare_assets("latest").expect("prepare assets should succeed"),
        );
        assert_eq!(prepared.tag_name, tag);
        assert!(prepared.cache_dir.exists());
        assert!(prepared.library_path.exists());
        assert!(prepared.model_path.exists());

        let cached = with_fake_tools_env(
            &root,
            &[
                ("KIWI_RS_CACHE_DIR", Some(cache_root_str)),
                ("FAKE_RELEASE_TAG", Some(tag.as_str())),
                ("FAKE_LIB_ASSET_NAME", Some(lib_asset.as_str())),
                ("FAKE_MODEL_ASSET_NAME", Some(model_asset.as_str())),
                ("FAKE_LIBRARY_FILENAME", Some(library_filename)),
                ("FAKE_TAR_FAIL", Some("1")),
            ],
            || prepare_assets("latest").expect("cache hit should bypass extraction"),
        );
        assert_eq!(cached.cache_dir, prepared.cache_dir);
        assert_eq!(cached.library_path, prepared.library_path);
        assert_eq!(cached.model_path, prepared.model_path);

        remove_tree(&root);
    }

    #[cfg(unix)]
    #[test]
    fn prepare_assets_rejects_invalid_resolved_tag() {
        let root = make_temp_dir("prepare-assets-bad-tag");
        let cache_root = root.join("cache");
        let cache_root_str = cache_root.to_str().expect("temp path should be utf-8");

        with_fake_tools_env(
            &root,
            &[
                ("KIWI_RS_CACHE_DIR", Some(cache_root_str)),
                ("FAKE_CURL_BAD_TAG", Some("1")),
            ],
            || {
                let err =
                    prepare_assets("latest").expect_err("invalid release tag should fail fast");
                assert!(err.to_string().contains("resolved invalid release tag"));
            },
        );
        remove_tree(&root);
    }

    #[cfg(unix)]
    #[test]
    fn prepare_assets_errors_when_library_is_missing_after_extraction() {
        let root = make_temp_dir("prepare-assets-missing-lib");
        let cache_root = root.join("cache");
        let version = "9.9.9";
        let lib_asset = platform_library_asset_name(version).expect("platform should be supported");
        let model_asset = format!("kiwi_model_v{version}_base.tgz");
        let cache_root_str = cache_root.to_str().expect("temp path should be utf-8");

        with_fake_tools_env(
            &root,
            &[
                ("KIWI_RS_CACHE_DIR", Some(cache_root_str)),
                ("FAKE_RELEASE_TAG", Some("v9.9.9")),
                ("FAKE_LIB_ASSET_NAME", Some(lib_asset.as_str())),
                ("FAKE_MODEL_ASSET_NAME", Some(model_asset.as_str())),
                ("FAKE_SKIP_LIB_FILE", Some("1")),
            ],
            || {
                let err =
                    prepare_assets("latest").expect_err("missing library output should error");
                assert!(err
                    .to_string()
                    .contains("library file was not found after extraction"));
            },
        );
        remove_tree(&root);
    }
}
