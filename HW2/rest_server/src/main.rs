use anyhow::Result;
use rest_server::run;

fn main() -> Result<()> {
    let data_dir = "../ml_processor/ml-data";
    run(data_dir)
}
