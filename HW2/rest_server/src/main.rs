use std::env;

use anyhow::Result;
use rest_server::run;
use tracing::Level;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(Level::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let data_dir = match env::var("DATA_DIR") {
        Ok(d) => Box::leak(d.into()),
        Err(_) => "../ml_processor/ml-data",
    };
    let port = match env::var("PORT") {
        Ok(d) => Box::leak(d.into()),
        Err(_) => "3000",
    };
    run(data_dir, port)
}
