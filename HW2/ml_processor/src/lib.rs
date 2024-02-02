use std::{fs::File, io::Read, path::Path};

use anyhow::{Result, Context, anyhow};
use apriori::Rule;
use bincode::serialize_into;
use log::{debug, warn};

use checkpoint::{check_checkpoint, checkpoint_path, write_checkpoint};

mod checkpoint;
mod url_file;
#[cfg(test)]
mod tests;

pub fn run(dataset_url: &str, _songs_url: &str, data_dir: impl AsRef<Path>) -> Result<()> {
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
