use std::time::Duration;

use notify::{recommended_watcher, Event, RecommendedWatcher, Watcher};

use super::*;

const TEN_SECONDS: Duration = Duration::from_secs(10);

pub async fn keep_watching_file(path: PathBuf, sender: Sender<Event>) {
    loop {
        debug!("Starting watcher for `{}`.", path.display());
        let (mut watcher, mut raw_receiver) = match watch_file(&path) {
            Ok(r) => r,
            Err(why) => {
                error!("Failed to watch `{}`: {}.", path.display(), why);
                sleep(TEN_SECONDS).await;
                continue;
            }
        };
        while let Some(maybe_event) = raw_receiver.recv().await {
            match maybe_event {
                Ok(event) => {
                    if sender.send(event).await.is_err() {
                        warn!("File watcher exiting because the channel is closed.");
                        return;
                    }
                }
                Err(why) => {
                    error!("Received file watcher error: {}. Restarting watcher", why);
                    break;
                }
            }
        }

        _ = watcher.unwatch(&path);
        sleep(TEN_SECONDS).await;
    }
}

pub fn watch_file(
    path: impl AsRef<Path>,
) -> Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (sender, receiver) = channel(1);
    let mut watcher = recommended_watcher(move |event| {
        sender
            .blocking_send(event)
            .expect("Failed to send watcher event")
    })?;
    watcher.watch(path.as_ref(), notify::RecursiveMode::NonRecursive)?;

    Ok((watcher, receiver))
}
