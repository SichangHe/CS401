#![allow(clippy::type_complexity)]
use anyhow::{Context, Result};
use apriori::Rule;
use read_rules::rule_query_server;
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
    time::{sleep, timeout},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

use actor::{Actor, Ref as ActorRef};
use read_rules::QueryServerMsg;
use serve::RecommendationResponse;

pub mod actor;
mod read_rules;
mod serve;
mod watch_file;

const FIVE_SECONDS: Duration = Duration::from_secs(5);

#[main]
#[instrument(skip(data_dir), fields(data_dir = ?data_dir.as_ref()))]
pub async fn run(data_dir: impl AsRef<Path>, port: &str) -> Result<()> {
    let (query_sender, query_receiver) = channel(16);
    let rule_query_thread = spawn(rule_query_server(
        data_dir.as_ref().into(),
        query_sender.clone(),
        query_receiver,
    ));

    // _testing(query_sender.clone()).await?;
    serve::serve(port, query_sender.clone()).await?;

    query_sender.send(QueryServerMsg::Exit).await?;
    rule_query_thread.await?;

    Ok(())
}

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
