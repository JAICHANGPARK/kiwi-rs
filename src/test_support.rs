use std::env;
use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn set_env_var(key: &str, value: &str) {
    #[allow(unused_unsafe)]
    unsafe {
        env::set_var(key, value);
    }
}

fn remove_env_var(key: &str) {
    #[allow(unused_unsafe)]
    unsafe {
        env::remove_var(key);
    }
}

/// Runs a closure with one overridden environment variable.
pub(crate) fn with_env_var<T>(key: &str, value: &str, f: impl FnOnce() -> T) -> T {
    with_env_vars(&[(key, Some(value))], f)
}

/// Runs a closure while holding a global environment lock and applying overrides.
pub(crate) fn with_env_vars<T>(overrides: &[(&str, Option<&str>)], f: impl FnOnce() -> T) -> T {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let backups: Vec<(&str, Option<OsString>)> = overrides
        .iter()
        .map(|(key, _)| (*key, env::var_os(key)))
        .collect();

    for (key, value) in overrides {
        match value {
            Some(value) => set_env_var(key, value),
            None => remove_env_var(key),
        }
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    for (key, value) in backups.into_iter().rev() {
        match value {
            Some(value) => {
                #[allow(unused_unsafe)]
                unsafe {
                    env::set_var(key, value);
                }
            }
            None => remove_env_var(key),
        }
    }

    match result {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}
