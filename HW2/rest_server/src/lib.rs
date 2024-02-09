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
    sync::mpsc::{channel, error::SendError, Receiver, Sender},
    task::JoinHandle,
    time::{sleep, timeout},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

use read_rules::QueryServerMsg;
use serve::RecommendationResponse;

mod read_rules;
mod serve;
mod watch_file;

const FIVE_SECONDS: Duration = Duration::from_secs(5);

pub struct ActorRef<A: Actor> {
    actor: JoinHandle<Result<()>>,
    env: ActorEnv<A::Msg>,
}

impl<A: Actor> ActorRef<A> {
    pub fn send(
        &mut self,
        msg: A::Msg,
    ) -> impl Future<Output = Result<(), SendError<<A as Actor>::Msg>>> + '_ {
        self.env.self_sender.send(msg)
    }

    pub fn cancel(&mut self) {
        self.env.cancellation_token.cancel()
    }

    pub fn abort(self) {
        self.actor.abort()
    }

    pub fn to_parts(self) -> (JoinHandle<Result<()>>, ActorEnv<A::Msg>) {
        (self.actor, self.env)
    }
}

#[derive(Debug)]
pub struct ActorEnv<M> {
    pub self_sender: Sender<M>,
    pub cancellation_token: CancellationToken,
}

impl<M> Clone for ActorEnv<M> {
    fn clone(&self) -> Self {
        Self {
            self_sender: self.self_sender.clone(),
            cancellation_token: self.cancellation_token.clone(),
        }
    }
}

pub trait Actor: Sized + Send + 'static {
    type Msg: Send;

    fn handle(
        &mut self,
        msg: Self::Msg,
        env: &ActorEnv<Self::Msg>,
    ) -> impl Future<Output = Result<()>> + Send;

    fn handle_continuously(
        &mut self,
        mut receiver: Receiver<Self::Msg>,
        env: ActorEnv<Self::Msg>,
    ) -> impl Future<Output = Result<()>> + Send {
        async move {
            loop {
                let maybe_msg = select! {
                    m = receiver.recv() => m,
                    () = env.cancellation_token.cancelled() => return Ok(()),
                };

                let msg = match maybe_msg {
                    Some(m) => m,
                    None => return Ok(()),
                };

                select! {
                    maybe_ok = self.handle(msg, &env) => maybe_ok,
                    () = env.cancellation_token.cancelled() => return Ok(()),
                }?;
            }
        }
    }

    fn spawn(mut self) -> ActorRef<Self> {
        let (self_sender, receiver) = channel(8);
        let env = ActorEnv {
            self_sender,
            cancellation_token: CancellationToken::new(),
        };
        let actor = {
            let env = env.clone();
            spawn(async move { self.handle_continuously(receiver, env).await })
        };

        ActorRef { actor, env }
    }
}

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
