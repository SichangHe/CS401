#![allow(clippy::type_complexity)]
use anyhow::{Context, Result};
use apriori::Rule;
use read_rules::rule_query_server;
use serde::Serialize;
use shared::*;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    main, select, spawn,
    sync::mpsc::{channel, Receiver, Sender},
    time::{sleep, timeout},
};
use tracing::{debug, error, info, instrument, warn};

use read_rules::QueryServerMsg;

mod read_rules;
mod watch_file;

const FIVE_SECONDS: Duration = Duration::from_secs(5);

#[main]
#[instrument(skip(data_dir), fields(data_dir = ?data_dir.as_ref()))]
pub async fn run(data_dir: impl AsRef<Path>) -> Result<()> {
    let (query_sender, query_receiver) = channel(16);
    let rule_query_thread = spawn(rule_query_server(
        data_dir.as_ref().into(),
        query_sender.clone(),
        query_receiver,
    ));

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
        let response = RecommendationResponse::new(playlist_ids, &model_date);
        let response = serde_json::to_string(&response)?;
        warn!(response);
        sleep(FIVE_SECONDS).await;
    }

    query_sender.send(QueryServerMsg::Exit).await?;
    drop(query_sender);
    rule_query_thread.await?;

    Ok(())
}

#[derive(Clone, Debug, Serialize)]
pub struct RecommendationResponse<'a> {
    pub playlist_ids: Vec<String>,
    pub version: &'a str,
    pub model_date: &'a str,
}

impl<'a> RecommendationResponse<'a> {
    pub fn new(playlist_ids: Vec<String>, model_date: &'a str) -> Self {
        Self {
            playlist_ids,
            version: crate_version!(),
            model_date,
        }
    }
}
