use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;

pub struct Castle {
    location: PathBuf
}

impl Castle {
    pub fn new_for_path<P: AsRef<Path>>(path: P) -> Castle {
        Castle{location: path.as_ref().to_path_buf()}
    }

    pub fn name(&self) -> Option<&OsStr> {
        self.location.file_name()
    }
       
}
