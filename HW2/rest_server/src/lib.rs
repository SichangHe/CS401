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

use read_rules::QueryServerMsg;
use serve::RecommendationResponse;

mod read_rules;
mod serve;
mod watch_file;

const FIVE_SECONDS: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct ActorRef<A: Actor> {
    pub self_sender: Sender<ActorMsg<A>>,
    pub cancellation_token: CancellationToken,
}

impl<A: Actor> ActorRef<A> {
    pub fn cast(
        &mut self,
        msg: A::CastMsg,
    ) -> impl Future<Output = Result<(), SendError<ActorMsg<A>>>> + '_ {
        self.self_sender.send(ActorMsg::Cast(msg))
    }

    pub async fn call(&mut self, msg: A::CallMsg) -> Result<A::Reply> {
        let (reply_sender, reply_receiver) = oneshot::channel();
        self.self_sender
            .send(ActorMsg::Call(msg, reply_sender))
            .await
            .context("Failed to send call to actor")?;
        reply_receiver
            .await
            .context("Failed to receive actor's reply")
    }

    pub fn cancel(&mut self) {
        self.cancellation_token.cancel()
    }
}

impl<A: Actor> Clone for ActorRef<A> {
    fn clone(&self) -> Self {
        Self {
            self_sender: self.self_sender.clone(),
            cancellation_token: self.cancellation_token.clone(),
        }
    }
}

pub enum ActorMsg<A: Actor> {
    Call(A::CallMsg, oneshot::Sender<A::Reply>),
    Cast(A::CastMsg),
}

pub trait Actor: Sized + Send + 'static {
    type CallMsg: Send + Sync;
    type CastMsg: Send + Sync;
    type Reply: Send;

    fn handle_cast(
        &mut self,
        msg: Self::CastMsg,
        env: &ActorRef<Self>,
    ) -> impl Future<Output = Result<()>> + Send;

    fn handle_call(
        &mut self,
        msg: Self::CallMsg,
        env: &ActorRef<Self>,
        reply_sender: oneshot::Sender<Self::Reply>,
    ) -> impl Future<Output = Result<()>> + Send;

    fn handle_call_or_cast(
        &mut self,
        msg: ActorMsg<Self>,
        env: &ActorRef<Self>,
    ) -> impl Future<Output = Result<()>> + Send {
        async move {
            match msg {
                ActorMsg::Call(msg, reply_sender) => self.handle_call(msg, env, reply_sender).await,
                ActorMsg::Cast(msg) => self.handle_cast(msg, env).await,
            }
        }
    }

    fn handle_continuously(
        &mut self,
        mut receiver: Receiver<ActorMsg<Self>>,
        env: ActorRef<Self>,
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
                    maybe_ok = self.handle_call_or_cast(msg, &env) => maybe_ok,
                    () = env.cancellation_token.cancelled() => return Ok(()),
                }?;
            }
        }
    }

    fn spawn(mut self) -> (JoinHandle<Result<()>>, ActorRef<Self>) {
        let (self_sender, actor_receiver) = channel(8);

        let actor_ref = ActorRef {
            self_sender,
            cancellation_token: CancellationToken::new(),
        };
        let handle = {
            let env = actor_ref.clone();
            spawn(async move { self.handle_continuously(actor_receiver, env).await })
        };

        (handle, actor_ref)
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
