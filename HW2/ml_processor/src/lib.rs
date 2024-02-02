use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use apriori::Rule;
use bincode::serialize_into;
use log::{debug, warn};

#[cfg(test)]
mod tests;

pub fn run(dataset_url: &str, songs_url: &str, data_dir: impl AsRef<Path>) -> Result<()> {
    let checkpoint_path = checkpoint_path(&data_dir);
    match check_checkpoint(dataset_url, &checkpoint_path) {
        Ok(true) => {
            debug!("Checkpoint is up to date, the ML processor is skipping processing.");
            return Ok(());
        }
        Ok(false) => {}
        Err(why) => warn!("Failed to check the checkpoint: {:?}", why),
    }

    write_checkpoint(dataset_url, &checkpoint_path)?;
    Ok(())
}

fn checkpoint_path(data_dir: impl AsRef<Path>) -> PathBuf {
    data_dir.as_ref().join("ml_processor_checkpoint.txt")
}

/// Copied from <https://docs.rs/clap/latest/clap/macro.crate_version.html>.
macro_rules! crate_version {
    () => {
        env!("CARGO_PKG_VERSION")
    };
}

/// Check if the checkpoint uses the same configuration as we do.
fn check_checkpoint(dataset_url: &str, checkpoint_path: impl AsRef<Path>) -> Result<bool> {
    let checkpoint_file_content =
        read_file(checkpoint_path).context("Failed to read checkpoint file")?;
    let mut splits = checkpoint_file_content.split_whitespace();

    let previous_version = splits
        .next()
        .context("No previous ML processor version in checkpoint file")?;
    if previous_version != crate_version!() {
        debug!(
            "Previous checkpoint has a different ML processor version `{}`.",
            previous_version
        );
        return Ok(false);
    }

    let previous_url = splits
        .next()
        .context("No previous dataset url in checkpoint file")?;
    if previous_url != dataset_url {
        debug!(
            "Previous checkpoint has a different dataset URL `{}`.",
            previous_url
        );
        return Ok(false);
    }

    Ok(true)
}

fn write_checkpoint(dataset_url: &str, checkpoint_path: impl AsRef<Path>) -> Result<()> {
    let mut checkpoint_file = File::create(checkpoint_path)?;
    writeln!(
        checkpoint_file,
        "{} {} {}",
        crate_version!(),
        dataset_url,
        unix_time().as_nanos()
    )?;
    debug!("Wrote checkpoint file.");

    Ok(())
}

fn unix_time() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Current time is later than UNIX epoch")
}

fn read_file(path: impl AsRef<Path>) -> Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

pub fn write_rules(rules: &[Rule], path: impl AsRef<Path>) -> Result<()> {
    let file = File::open(path)?;
    serialize_into(file, &rules[0])?;
    Ok(())
}
