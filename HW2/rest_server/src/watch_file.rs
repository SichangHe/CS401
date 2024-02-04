use notify::{recommended_watcher, Event, RecommendedWatcher, Watcher};

use super::*;

#[instrument(skip(sender, exit))]
pub async fn keep_watching_file(
    path: PathBuf,
    sender: Sender<QueryServerMsg>,
    mut exit: Receiver<()>,
) {
    let parent = path.parent().unwrap_or_else(|| &path);
    loop {
        debug!("Starting watcher.");
        let (mut watcher, mut raw_receiver) = match watch_file(parent) {
            Ok(r) => r,
            Err(why) => {
                error!(?why, "Failed to watch.");
                sleep(FIVE_SECONDS).await;
                continue;
            }
        };

        loop {
            let (maybe_event, when) = select! {
                e = raw_receiver.recv() => if let Some(m) = e { m } else { break; },
                _ = exit.recv() => {
                    debug!("Received exit request.");
                    return;
                },
            };
            match maybe_event {
                Ok(event) => {
                    let (kind, paths) = (event.kind, event.paths);
                    debug!(?kind, ?paths, "File watcher received event.");
                    if sender
                        .send(QueryServerMsg::WatchedFileChanged(when))
                        .await
                        .is_err()
                    {
                        warn!("File watcher exiting because the channel is closed.");
                        return;
                    }
                }
                Err(why) => {
                    error!(?why, "Received file watcher error. Restarting watcher");
                    break;
                }
            }
        }

        _ = watcher.unwatch(parent);
        drop(watcher);
        sleep(FIVE_SECONDS).await;
    }
}

pub fn watch_file(
    path: impl AsRef<Path>,
) -> Result<(
    RecommendedWatcher,
    Receiver<(notify::Result<Event>, Instant)>,
)> {
    let (sender, receiver) = channel(1);
    let mut watcher = recommended_watcher(move |event| {
        sender
            .blocking_send((event, Instant::now()))
            .expect("Failed to send watcher event")
    })?;
    watcher.watch(path.as_ref(), notify::RecursiveMode::NonRecursive)?;

    Ok((watcher, receiver))
}
