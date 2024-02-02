use anyhow::Result;
use log::LevelFilter;
use ml_processor::run;

fn main() -> Result<()> {
    env_logger::builder()
        .filter_module("ml_processor", LevelFilter::Debug)
        .parse_default_env()
        .init();

    let dataset_url = "https://homepages.dcc.ufmg.br/~cunha/hosted/cloudcomp-2023s2-datasets/2023_spotify_ds1.csv";
    let data_dir = "ml-data";
    run(dataset_url, data_dir)?;

    Ok(())
}
