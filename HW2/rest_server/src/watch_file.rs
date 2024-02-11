use notify::{recommended_watcher, Event, RecommendedWatcher, Watcher};

use self::read_rules::{RuleServer, RuleServerMsg};

use super::*;

pub struct FileWatcher {
    path: PathBuf,
    watcher: Option<RecommendedWatcher>,
    server_ref: Ref<RuleServer>,
}

impl FileWatcher {
    pub fn new(path: PathBuf, server_ref: Ref<RuleServer>) -> Self {
        Self {
            path,
            watcher: None,
            server_ref,
        }
    }

    pub async fn try_start_watcher(&mut self, env: &mut Ref<Self>) -> Result<()> {
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

    async fn init(&mut self, env: &mut Ref<Self>) -> Result<()> {
        env.cast(FileWatchEvent::Init).await?;
        Ok(())
    }

    #[instrument(skip(self, msg, env))]
    async fn handle_cast(&mut self, msg: Self::CastMsg, env: &mut Ref<Self>) -> Result<()> {
        match msg {
            FileWatchEvent::Event(Ok(event), when) => {
                let (kind, paths) = (event.kind, event.paths);
                debug!(?kind, ?paths, "File watcher event.");

                let file_event = RuleServerMsg::WatchedFileChanged(when);
                if self.server_ref.cast(file_event).await.is_err() {
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
                        sleep(ONE_SECOND).await;
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
