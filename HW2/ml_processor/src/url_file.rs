use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process::Command,
};

use apriori::{apriori, Rule};

use super::*;

const MIN_SUPPORT: f32 = 0.025;
const MIN_CONFIDENCE: f32 = 0.7;

pub fn process_data(dataset_url: &str, data_dir: impl AsRef<Path>) -> Result<Vec<Rule>> {
    let dataset_file_content = read_url_file(dataset_url, data_dir)?;
    let mut data_set_lines = dataset_file_content.lines();

    let (playlist_id_index, track_name_index) = get_playlist_id_and_track_name_index_in_header(
        data_set_lines
            .next()
            .context("The dataset file is empty.")?,
    )?;

    let mut raw_transactions = HashMap::<&str, HashSet<&str>>::new();
    for line in data_set_lines {
        let mut playlist_id = None;
        let mut track_name = None;
        for (index, attribute) in line.split(',').enumerate() {
            if index == playlist_id_index {
                playlist_id = Some(attribute);
            } else if index == track_name_index {
                track_name = Some(attribute);
            }
        }
        raw_transactions
            .entry(playlist_id.context("Line does not contain `pid` column")?)
            .or_default()
            .insert(track_name.context("Line does not contain `track_name` column")?);
    }
    debug!("Got {} playlists.", raw_transactions.len());

    let (rules, _frequent_itemsets) = apriori(
        raw_transactions.into_values().collect(),
        MIN_SUPPORT,
        MIN_CONFIDENCE,
        MAX_LENGTH,
    );

    Ok(rules)
}

fn get_playlist_id_and_track_name_index_in_header(header: &str) -> Result<(usize, usize)> {
    let mut playlist_id_index = None;
    let mut track_name_index = None;
    for (index, attribute) in header.split(',').enumerate() {
        match attribute {
            "pid" => playlist_id_index = Some(index),
            "track_name" => track_name_index = Some(index),
            _ => {}
        }
    }
    Ok((
        playlist_id_index.context("Dataset file has no `pid` column.")?,
        track_name_index.context("Dataset file has no `track_name` column.")?,
    ))
}

fn read_url_file(url: &str, data_dir: impl AsRef<Path>) -> Result<String> {
    let file_path = download(url, data_dir)?;
    let content = read_file(file_path)?;
    Ok(content)
}

fn download(url: &str, data_dir: impl AsRef<Path>) -> Result<PathBuf> {
    let file_name = url
        .split('/')
        .last()
        .expect("There should be at least one split.");
    let file_path = data_dir.as_ref().join(file_name);
    let file_path_str = file_path
        .to_str()
        .with_context(|| format!("File directory `{:?}` contains invalid UTF-8", file_path))?;

    debug!("Downloading `{}` to `{}`.", url, file_name);
    let args = [
        "-o",
        file_path_str,
        "--check-certificate",
        "false",
        "-c",
        url,
    ];
    let mut aria = Command::new("aria2c")
        .args(args)
        .spawn()
        .with_context(|| format!("Failed to spawn aria2c with {:?}.", args))?;
    let exit_status = aria.wait()?;

    if exit_status.success() {
        Ok(file_path)
    } else {
        Err(anyhow!("aria2c failed to download file `{}`", file_name))
    }
}
