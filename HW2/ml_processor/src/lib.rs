use std::{fs::File, path::Path};

use anyhow::Result;
use apriori::Rule;
use bincode::serialize_into;

#[cfg(test)]
mod tests;

pub fn write_rules(rules: &[Rule], path: impl AsRef<Path>) -> Result<()> {
    let file = File::open(path)?;
    serialize_into(file, &rules[0])?;
    Ok(())
}
