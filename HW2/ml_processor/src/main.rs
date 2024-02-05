use std::env;

use anyhow::Result;
use log::LevelFilter;
use ml_processor::run;

fn main() -> Result<()> {
    env_logger::builder()
        .filter_module("ml_processor", LevelFilter::Debug)
        .parse_default_env()
        .init();

    let dataset_url = match env::var("DATASET_URL") {
        Ok(d) => Box::leak(d.into()),
        Err(_) => "https://homepages.dcc.ufmg.br/~cunha/hosted/cloudcomp-2023s2-datasets/2023_spotify_ds1.csv",
    };
    let data_dir = match env::var("DATA_DIR") {
        Ok(d) => Box::leak(d.into()),
        Err(_) => "ml-data",
    };
    run(dataset_url, data_dir)?;

    Ok(())
}
