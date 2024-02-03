use anyhow::{anyhow, Context, Result};
use apriori::Rule;
use read_rules::rule_query_server;
use shared::*;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{
    main, select, spawn,
    sync::mpsc::{channel, Receiver, Sender},
    time::{sleep, timeout},
};
use tracing::Level;
use tracing::{debug, error, instrument, warn};

mod read_rules;
mod watch_file;

const TEN_SECONDS: Duration = Duration::from_secs(10);

#[main]
#[instrument(skip(data_dir), fields(data_dir = ?data_dir.as_ref()))]
pub async fn run(data_dir: impl AsRef<Path>) -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let (query_sender, query_receiver) = channel(8);
    let rule_query_thread = spawn(rule_query_server(data_dir.as_ref().into(), query_receiver));

    let (response_sender, mut response_receiver) = channel(1);
    let mock_query = vec![
        "Ride Wit Me".into(),
        "Bottle It Up - Acoustic Mixtape".into(),
        "DNA.".into(),
    ];
    for _ in 0..20 {
        query_sender
            .send((mock_query.clone(), response_sender.clone()))
            .await?;
        let response = response_receiver.recv().await;
        warn!(?response, "Got response from rule query server.");
        sleep(TEN_SECONDS).await;
    }

    drop(query_sender);
    rule_query_thread.await?;

    Ok(())
}
