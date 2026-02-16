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
