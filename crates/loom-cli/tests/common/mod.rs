use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

pub fn loom_bin() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target");
    path.push("debug");
    path.push("loom");
    path
}

pub struct TestInstall {
    pub _tempdir: TempDir,
    pub executable: PathBuf,
}

impl TestInstall {
    pub fn new() -> Self {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let bin_dir = tempdir.path().join(".local/bin");
        fs::create_dir_all(&bin_dir).expect("create bin dir");
        let executable = bin_dir.join("loom");
        fs::copy(loom_bin(), &executable).expect("copy loom binary");
        Self {
            _tempdir: tempdir,
            executable,
        }
    }
}
