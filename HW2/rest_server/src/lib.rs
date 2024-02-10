#![allow(clippy::type_complexity)]
use anyhow::{Context, Result};
use apriori::Rule;
use read_rules::RuleServer;
use serde::{Deserialize, Serialize};
use shared::*;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    future::Future,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    main, select, spawn,
    sync::{
        mpsc::{channel, error::SendError, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
    time::sleep,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

use actor::{Actor, Ref};
// use serve::RecommendationResponse;

pub mod actor;
mod read_rules;
mod serve;
mod watch_file;

const FIVE_SECONDS: Duration = Duration::from_secs(5);

#[main]
#[instrument(skip(data_dir), fields(data_dir = ?data_dir.as_ref()))]
pub async fn run(data_dir: impl AsRef<Path>, port: &str) -> Result<()> {
    let rule_query_server = RuleServer::new(data_dir.as_ref().into());
    let (query_server_handle, mut query_server_ref) = rule_query_server.spawn();

    // _testing(query_sender.clone()).await?;
    serve::serve(port, query_server_ref.clone()).await?;

    query_server_ref.cancel();
    query_server_handle.await??;

    Ok(())
}

/*
async fn _testing(query_sender: Sender<QueryServerMsg>) -> Result<()> {
    let (response_sender, mut response_receiver) = channel(1);
    let mock_query = vec![
        "Ride Wit Me".into(),
        "Bottle It Up - Acoustic Mixtape".into(),
        "DNA.".into(),
    ];
    let query = QueryServerMsg::Query(mock_query.clone(), response_sender.clone());
    for _ in 0..20 {
        query_sender.send(query.clone()).await?;
        let (playlist_ids, model_date) = response_receiver.recv().await.unwrap();
        let response = RecommendationResponse::new(playlist_ids, model_date.to_string());
        let response = serde_json::to_string(&response)?;
        warn!(response);
        sleep(FIVE_SECONDS).await;
    }

    Ok(())
}
*/
