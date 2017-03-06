
use std::path::{Path, PathBuf};
use std::ffi::OsStr;

extern crate git2;


pub struct Castle {
    repo: git2::Repository,
}

impl Castle {
    pub fn new_for_path<P: AsRef<Path>>(path: P) -> Result<Castle, String> {
        let repo = git2::Repository::open(path).map_err(|e| format!("could not open castle: {}", e))?;
        Ok(Castle{repo: repo})
    }

    pub fn name(&self) -> Option<&OsStr> {
        self.repo.workdir().and_then(|p| p.file_name())
    }

}
