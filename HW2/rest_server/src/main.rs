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

    let data_dir = "../ml_processor/ml-data";
    run(data_dir)
}
