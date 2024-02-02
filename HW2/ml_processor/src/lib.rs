use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use apriori::Rule;
use bincode::serialize_into;
use log::{debug, warn};

use checkpoint::{check_checkpoint, checkpoint_path, write_checkpoint};
use url_file::process_data;

mod checkpoint;
#[cfg(test)]
mod tests;
mod url_file;

pub fn run(dataset_url: &str, data_dir: impl AsRef<Path>) -> Result<()> {
    let checkpoint_path = checkpoint_path(&data_dir);
    match check_checkpoint(dataset_url, &checkpoint_path) {
        Ok(true) => {
            debug!("Checkpoint is up to date, the ML processor is skipping processing.");
            return Ok(());
        }
        Ok(false) => {}
        Err(why) => warn!("Failed to check the checkpoint: {:?}", why),
    }

    debug!("Processing dataset `{}`.", dataset_url);
    let rules = process_data(dataset_url, &data_dir)?;

    let rules_path = rules_path(&data_dir);
    debug!(
        "Writing {} rules to `{}`.",
        rules.len(),
        rules_path.display()
    );
    write_rules(&rules, rules_path)?;

    debug!("Writing new checkpoint to `{}`.", checkpoint_path.display());
    write_checkpoint(dataset_url, &checkpoint_path)?;
    Ok(())
}

fn read_file(path: impl AsRef<Path>) -> Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

pub fn rules_path(data_dir: impl AsRef<Path>) -> PathBuf {
    data_dir.as_ref().join("rules.bincode")
}

pub fn write_rules(rules: &[Rule], path: impl AsRef<Path>) -> Result<()> {
    let file = File::create(&path).context("Failed to open rules file")?;
    serialize_into(file, rules).context("Failed to write rules")?;
    Ok(())
}
