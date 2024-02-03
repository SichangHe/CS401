use std::time::Duration;

use notify::{recommended_watcher, Event, RecommendedWatcher, Watcher};

use super::*;

const TEN_SECONDS: Duration = Duration::from_secs(10);

#[instrument(skip(sender))]
pub async fn keep_watching_file(path: PathBuf, sender: Sender<Event>) {
    let file_name = path.file_name().expect("`path` is not a file.");
    let parent = path.parent().unwrap_or_else(|| &path);
    loop {
        debug!("Starting watcher.");
        let (mut watcher, mut raw_receiver) = match watch_file(parent) {
            Ok(r) => r,
            Err(why) => {
                error!(?why, "Failed to watch.");
                sleep(TEN_SECONDS).await;
                continue;
            }
        };
        while let Some(maybe_event) = raw_receiver.recv().await {
            match maybe_event {
                Ok(event) => {
                    if event.paths.iter().any(|p| match p.file_name() {
                        Some(changed_file_name) => *file_name == *changed_file_name,
                        None => false,
                    }) {
                        if sender.send(event).await.is_err() {
                            warn!("File watcher exiting because the channel is closed.");
                            return;
                        }
                    } else {
                        debug!(?event, "Filtering out irrelevent file watcher event.")
                    }
                }
                Err(why) => {
                    error!(?why, "Received file watcher error. Restarting watcher");
                    break;
                }
            }
        }

        _ = watcher.unwatch(parent);
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
