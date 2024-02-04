use anyhow::Result;
use rest_server::run;
use tracing::Level;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let data_dir = "../ml_processor/ml-data";
    run(data_dir)
}
