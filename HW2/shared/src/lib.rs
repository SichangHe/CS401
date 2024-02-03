use anyhow::Result;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

pub const MAX_LENGTH: usize = 8;

/// Copied from <https://docs.rs/clap/latest/clap/macro.crate_version.html>.
#[macro_export]
macro_rules! crate_version {
    () => {
        env!("CARGO_PKG_VERSION")
    };
}

pub fn rules_path(data_dir: impl AsRef<Path>) -> PathBuf {
    data_dir.as_ref().join("rules.bincode")
}

pub fn checkpoint_path(data_dir: impl AsRef<Path>) -> PathBuf {
    data_dir.as_ref().join("ml_processor_checkpoint.txt")
}

pub fn read_file(path: impl AsRef<Path>) -> Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}
