use std::{
    io::Write,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use log::debug;

use super::*;

pub fn checkpoint_path(data_dir: impl AsRef<Path>) -> PathBuf {
    data_dir.as_ref().join("ml_processor_checkpoint.txt")
}

/// Copied from <https://docs.rs/clap/latest/clap/macro.crate_version.html>.
macro_rules! crate_version {
    () => {
        env!("CARGO_PKG_VERSION")
    };
}

/// Check if the checkpoint uses the same configuration as we do.
pub fn check_checkpoint(dataset_url: &str, checkpoint_path: impl AsRef<Path>) -> Result<bool> {
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

pub fn write_checkpoint(dataset_url: &str, checkpoint_path: impl AsRef<Path>) -> Result<()> {
    let mut checkpoint_file =
        File::create(checkpoint_path).context("Failed to create checkpoint file")?;
    writeln!(
        checkpoint_file,
        "{} {} {}",
        crate_version!(),
        dataset_url,
        unix_time().as_nanos()
    )
    .context("Failed to write to the checkpoint file.")?;
    debug!("Wrote checkpoint file.");

    Ok(())
}

fn unix_time() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Current time is later than UNIX epoch")
}
