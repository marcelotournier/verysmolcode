//! Per-file mutation serialization (Rust port of pi's file-mutation-queue.ts).
//!
//! Operations targeting the same canonical path run one at a time. Operations
//! against *different* files still run in parallel.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

fn registry() -> &'static Mutex<HashMap<PathBuf, Arc<Mutex<()>>>> {
    static R: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

fn key_for(path: &Path) -> PathBuf {
    // Try canonicalize so symlinks/relative paths line up. If the path doesn't
    // exist yet (write-create), fall back to the absolute form.
    std::fs::canonicalize(path).unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    })
}

fn lock_for(path: &Path) -> Arc<Mutex<()>> {
    let key = key_for(path);
    let mut map = registry().lock().expect("file-mutation registry poisoned");
    map.entry(key).or_insert_with(|| Arc::new(Mutex::new(()))).clone()
}

/// Run `f` while holding the mutation lock for the given file path.
pub fn with_file_mutation_queue<F, R>(path: &Path, f: F) -> R
where
    F: FnOnce() -> R,
{
    let lock = lock_for(path);
    let _guard = lock.lock().expect("file-mutation lock poisoned");
    f()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_serializes_same_path() {
        let counter = Arc::new(AtomicUsize::new(0));
        let max_concurrent = Arc::new(AtomicUsize::new(0));
        let path = std::env::temp_dir().join("vsc_mutqueue_serialize.txt");
        std::fs::write(&path, "x").unwrap();

        let mut handles = Vec::new();
        for _ in 0..8 {
            let p = path.clone();
            let c = counter.clone();
            let m = max_concurrent.clone();
            handles.push(thread::spawn(move || {
                with_file_mutation_queue(&p, || {
                    let n = c.fetch_add(1, Ordering::SeqCst) + 1;
                    let cur_max = m.load(Ordering::SeqCst);
                    if n > cur_max {
                        m.store(n, Ordering::SeqCst);
                    }
                    thread::sleep(Duration::from_millis(10));
                    c.fetch_sub(1, Ordering::SeqCst);
                });
            }));
        }
        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(max_concurrent.load(Ordering::SeqCst), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_parallel_for_different_paths() {
        // We can't reliably assert parallelism without timing tricks, but at
        // least confirm distinct paths use distinct locks (no deadlock when
        // both are acquired).
        let p1 = std::env::temp_dir().join("vsc_mutqueue_a.txt");
        let p2 = std::env::temp_dir().join("vsc_mutqueue_b.txt");
        std::fs::write(&p1, "a").unwrap();
        std::fs::write(&p2, "b").unwrap();

        let l1 = lock_for(&p1);
        let l2 = lock_for(&p2);
        assert!(!Arc::ptr_eq(&l1, &l2));
        let _ = std::fs::remove_file(&p1);
        let _ = std::fs::remove_file(&p2);
    }
}
