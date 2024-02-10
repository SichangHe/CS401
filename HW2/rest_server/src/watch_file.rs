use notify::{recommended_watcher, Event, RecommendedWatcher, Watcher};

use super::*;

#[derive(Debug)]
pub struct FileWatcher {
    path: PathBuf,
    watcher: Option<RecommendedWatcher>,
    query_sender: Sender<QueryServerMsg>,
}

impl FileWatcher {
    pub fn new(path: PathBuf, query_sender: Sender<QueryServerMsg>) -> Self {
        Self {
            path,
            watcher: None,
            query_sender,
        }
    }

    pub async fn try_start_watcher(&mut self, env: &mut ActorRef<Self>) -> Result<()> {
        let mut event_sender = env.clone();

        let mut watcher = recommended_watcher(move |event| {
            event_sender
                .blocking_cast(FileWatchEvent::Event(event, Instant::now()))
                .expect("Failed to send watcher event")
        })?;
        watcher.watch(&self.path, notify::RecursiveMode::NonRecursive)?;

        self.watcher = Some(watcher);
        Ok(())
    }
}

impl Actor for FileWatcher {
    type CallMsg = ();
    type CastMsg = FileWatchEvent;
    type Reply = ();

    async fn init(&mut self, env: &mut ActorRef<Self>) -> Result<()> {
        env.cast(FileWatchEvent::Init).await?;
        Ok(())
    }

    #[instrument(skip(self, msg, env))]
    async fn handle_cast(&mut self, msg: Self::CastMsg, env: &mut ActorRef<Self>) -> Result<()> {
        match msg {
            FileWatchEvent::Event(Ok(event), when) => {
                let (kind, paths) = (event.kind, event.paths);
                debug!(?kind, ?paths, "File watcher event.");

                if self
                    .query_sender
                    .send(QueryServerMsg::WatchedFileChanged(when))
                    .await
                    .is_err()
                {
                    warn!("File watcher exiting because the query receiver is closed.");
                    env.cancel();
                }
            }
            FileWatchEvent::Event(Err(why), _) => {
                error!(?why, "Received file watcher error. Restarting watcher");

                let mut env = env.clone();
                drop(spawn(
                    async move { _ = env.cast(FileWatchEvent::Init).await },
                ));
            }

            FileWatchEvent::Init => {
                info!("Initializing file watcher.");

                if let Err(why) = self.try_start_watcher(env).await {
                    error!(
                        ?why,
                        "Failed to initialize watcher, restarting after sleep."
                    );

                    let mut env = env.clone();
                    drop(spawn(async move {
                        sleep(FIVE_SECONDS).await;
                        _ = env.cast(FileWatchEvent::Init).await
                    }));
                }
            }
        }

        Ok(())
    }
}

pub enum FileWatchEvent {
    Event(notify::Result<Event>, Instant),
    Init,
}
