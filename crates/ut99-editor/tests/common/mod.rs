use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct TestDir(PathBuf);

impl TestDir {
    pub fn new() -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let base = std::env::var_os("FORGE99_TEST_TMP")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        loop {
            let id = SEQ.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!("forge99-test-{}-{id}", std::process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("could not create isolated test directory: {e}"),
            }
        }
    }

    pub fn path(&self) -> &Path { &self.0 }
}

impl Drop for TestDir {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}
