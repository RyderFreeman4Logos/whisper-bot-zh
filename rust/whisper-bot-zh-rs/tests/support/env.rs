use std::sync::{Mutex, MutexGuard};

pub struct EnvGuard {
    original: Vec<(String, Option<String>)>,
    _lock: MutexGuard<'static, ()>,
}

impl EnvGuard {
    pub fn set(vars: &[(&str, Option<&str>)]) -> Self {
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let lock = ENV_LOCK.lock().expect("env lock");
        let original = vars
            .iter()
            .map(|(key, value)| {
                let previous = std::env::var(key).ok();
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
                ((*key).to_owned(), previous)
            })
            .collect();
        Self {
            original,
            _lock: lock,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.original.iter().rev() {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}
