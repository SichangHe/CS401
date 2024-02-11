#![allow(clippy::type_complexity)]
use anyhow::{Context, Result};
use apriori::Rule;
use read_rules::RuleServer;
use serde::{Deserialize, Serialize};
use shared::*;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{main, spawn, sync::oneshot, task::JoinHandle, time::sleep};
use tracing::{debug, error, info, instrument, warn};

use tokio_gen_server::actor::*;

mod read_rules;
mod serve;
mod watch_file;

const ONE_SECOND: Duration = Duration::from_secs(1);

#[main]
#[instrument(skip(data_dir), fields(data_dir = ?data_dir.as_ref()))]
pub async fn run(data_dir: impl AsRef<Path>, port: &str) -> Result<()> {
    let rule_server = RuleServer::new(data_dir.as_ref().into());
    let (rule_server_handle, mut rule_server_ref) = rule_server.spawn();

    serve::serve(port, rule_server_ref.clone()).await?;

    rule_server_ref.cancel();
    rule_server_handle.await??;

    Ok(())
}
