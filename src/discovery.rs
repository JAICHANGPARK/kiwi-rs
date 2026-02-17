use std::env;
use std::path::PathBuf;

pub(crate) fn default_library_candidates() -> &'static [&'static str] {
    #[cfg(target_os = "windows")]
    {
        &["kiwi.dll", "libkiwi.dll"]
    }
    #[cfg(target_os = "macos")]
    {
        &[
            "libkiwi.dylib",
            "kiwi.dylib",
            "/usr/local/lib/libkiwi.dylib",
            "/opt/homebrew/lib/libkiwi.dylib",
            "@rpath/libkiwi.dylib",
            "@loader_path/libkiwi.dylib",
            "@loader_path/../Frameworks/libkiwi.dylib",
            "Kiwi.framework/Kiwi",
        ]
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        &[
            "libkiwi.so",
            "kiwi.so",
            "./libkiwi.so",
            "/usr/local/lib/libkiwi.so",
            "/usr/lib/libkiwi.so",
        ]
    }
}

pub(crate) fn discover_default_library_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
            let path = PathBuf::from(local_app_data)
                .join("kiwi")
                .join("lib")
                .join("kiwi.dll");
            if path.exists() {
                return Some(path);
            }
        }
        if let Some(user_profile) = env::var_os("USERPROFILE") {
            let path = PathBuf::from(user_profile)
                .join("AppData")
                .join("Local")
                .join("kiwi")
                .join("lib")
                .join("kiwi.dll");
            if path.exists() {
                return Some(path);
            }
        }
        let well_known = [
            PathBuf::from("C:\\kiwi\\lib\\kiwi.dll"),
            PathBuf::from("C:\\Program Files\\Kiwi\\lib\\kiwi.dll"),
        ];
        for path in well_known {
            if path.exists() {
                return Some(path);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = env::var_os("HOME") {
            let path = PathBuf::from(home)
                .join(".local")
                .join("kiwi")
                .join("lib")
                .join("libkiwi.dylib");
            if path.exists() {
                return Some(path);
            }
        }

        let well_known = [
            PathBuf::from("/usr/local/lib/libkiwi.dylib"),
            PathBuf::from("/opt/homebrew/lib/libkiwi.dylib"),
        ];
        for path in well_known {
            if path.exists() {
                return Some(path);
            }
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(home) = env::var_os("HOME") {
            let path = PathBuf::from(home)
                .join(".local")
                .join("kiwi")
                .join("lib")
                .join("libkiwi.so");
            if path.exists() {
                return Some(path);
            }
        }

        let well_known = [
            PathBuf::from("/usr/local/lib/libkiwi.so"),
            PathBuf::from("/usr/lib/libkiwi.so"),
        ];
        for path in well_known {
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

pub(crate) fn discover_default_model_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("KIWI_MODEL_PATH") {
        return Some(PathBuf::from(path));
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
            let path = PathBuf::from(local_app_data)
                .join("kiwi")
                .join("models")
                .join("cong")
                .join("base");
            if path.exists() {
                return Some(path);
            }
        }
        if let Some(user_profile) = env::var_os("USERPROFILE") {
            let path = PathBuf::from(user_profile)
                .join("AppData")
                .join("Local")
                .join("kiwi")
                .join("models")
                .join("cong")
                .join("base");
            if path.exists() {
                return Some(path);
            }
        }
    }

    #[cfg(target_os = "windows")]
    let candidates: &[&str] = &[
        "C:\\kiwi\\models\\cong\\base",
        "C:\\Program Files\\Kiwi\\models\\cong\\base",
    ];

    #[cfg(target_os = "macos")]
    let candidates: &[&str] = &[
        "~/.local/kiwi/models/cong/base",
        "/usr/local/models/cong/base",
        "/opt/homebrew/models/cong/base",
        "/usr/local/share/kiwi/models/cong/base",
    ];

    #[cfg(all(unix, not(target_os = "macos")))]
    let candidates: &[&str] = &[
        "~/.local/kiwi/models/cong/base",
        "/usr/local/models/cong/base",
        "/usr/local/share/kiwi/models/cong/base",
        "/usr/share/kiwi/models/cong/base",
    ];

    for candidate in candidates {
        let path = if let Some(stripped) = candidate.strip_prefix("~/") {
            match env::var_os("HOME") {
                Some(home) => PathBuf::from(home).join(stripped),
                None => continue,
            }
        } else {
            PathBuf::from(candidate)
        };
        if path.exists() {
            return Some(path);
        }
    }

    None
}

#[cfg(test)]
mod discovery_tests {
    use super::{
        default_library_candidates, discover_default_library_path, discover_default_model_path,
    };
    use crate::test_support::with_env_vars;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("kiwi-rs-{name}-{suffix}"));
        fs::create_dir_all(&path).expect("failed to create temp dir");
        path
    }

    fn remove_tree(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn default_library_candidates_match_platform() {
        let candidates = default_library_candidates();
        assert!(!candidates.is_empty());

        #[cfg(target_os = "windows")]
        assert!(candidates
            .iter()
            .all(|candidate| candidate.ends_with(".dll")));
        #[cfg(target_os = "macos")]
        assert!(candidates
            .iter()
            .any(|candidate| candidate.ends_with(".dylib")));
        #[cfg(all(unix, not(target_os = "macos")))]
        assert!(candidates
            .iter()
            .any(|candidate| candidate.ends_with(".so")));
    }

    #[test]
    fn discover_default_model_path_prefers_env_var() {
        with_env_vars(
            &[
                ("KIWI_MODEL_PATH", Some("/tmp/kiwi-rs-model-from-env")),
                ("HOME", None),
                ("XDG_CACHE_HOME", None),
                ("LOCALAPPDATA", None),
                ("USERPROFILE", None),
            ],
            || {
                let path = discover_default_model_path();
                assert_eq!(path, Some(PathBuf::from("/tmp/kiwi-rs-model-from-env")));
            },
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn discover_default_model_path_expands_home_candidate() {
        let home = make_temp_dir("discover-model-home");
        let model = home
            .join(".local")
            .join("kiwi")
            .join("models")
            .join("cong")
            .join("base");
        fs::create_dir_all(&model).expect("failed to prepare model path");

        with_env_vars(
            &[
                ("KIWI_MODEL_PATH", None),
                ("HOME", Some(home.to_str().expect("utf-8 temp path"))),
            ],
            || {
                let path = discover_default_model_path();
                assert_eq!(path, Some(model.clone()));
            },
        );

        remove_tree(&home);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn discover_default_model_path_finds_localappdata_candidate() {
        let root = make_temp_dir("discover-model-win");
        let model = root.join("kiwi").join("models").join("cong").join("base");
        fs::create_dir_all(&model).expect("failed to prepare model path");

        with_env_vars(
            &[
                ("KIWI_MODEL_PATH", None),
                (
                    "LOCALAPPDATA",
                    Some(root.to_str().expect("utf-8 temp path")),
                ),
                ("USERPROFILE", None),
            ],
            || {
                let path = discover_default_model_path();
                assert_eq!(path, Some(model.clone()));
            },
        );

        remove_tree(&root);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn discover_default_library_path_finds_home_local_library() {
        let home = make_temp_dir("discover-lib-home");
        let library = {
            #[cfg(target_os = "macos")]
            let file_name = "libkiwi.dylib";
            #[cfg(all(unix, not(target_os = "macos")))]
            let file_name = "libkiwi.so";

            home.join(".local").join("kiwi").join("lib").join(file_name)
        };

        fs::create_dir_all(
            library
                .parent()
                .expect("library path must always include a parent"),
        )
        .expect("failed to create library parent dir");
        fs::write(&library, b"").expect("failed to create fake library");

        with_env_vars(
            &[("HOME", Some(home.to_str().expect("utf-8 temp path")))],
            || {
                let path = discover_default_library_path();
                assert_eq!(path, Some(library.clone()));
            },
        );

        remove_tree(&home);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn discover_default_library_path_returns_none_when_candidates_absent() {
        let home = make_temp_dir("discover-lib-none");
        with_env_vars(
            &[("HOME", Some(home.to_str().expect("utf-8 temp path")))],
            || {
                let path = discover_default_library_path();
                assert!(path.is_none());
            },
        );
        remove_tree(&home);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn discover_default_model_path_returns_none_without_env_or_candidates() {
        let home = make_temp_dir("discover-model-none");
        with_env_vars(
            &[
                ("KIWI_MODEL_PATH", None),
                ("HOME", Some(home.to_str().expect("utf-8 temp path"))),
            ],
            || {
                let path = discover_default_model_path();
                assert!(path.is_none());
            },
        );
        remove_tree(&home);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn discover_default_library_path_finds_localappdata_library() {
        let root = make_temp_dir("discover-lib-win");
        let library = root.join("kiwi").join("lib").join("kiwi.dll");
        fs::create_dir_all(
            library
                .parent()
                .expect("library path must always include a parent"),
        )
        .expect("failed to create library parent dir");
        fs::write(&library, b"").expect("failed to create fake library");

        with_env_vars(
            &[
                (
                    "LOCALAPPDATA",
                    Some(root.to_str().expect("utf-8 temp path")),
                ),
                ("USERPROFILE", None),
            ],
            || {
                let path = discover_default_library_path();
                assert_eq!(path, Some(library.clone()));
            },
        );

        remove_tree(&root);
    }
}
