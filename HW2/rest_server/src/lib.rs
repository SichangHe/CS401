use anyhow::{anyhow, Context, Result};
use tracing::{debug, error, warn};
use shared::*;
use std::path::{Path, PathBuf};
use tokio::{
    main, spawn,
    sync::mpsc::{channel, Receiver, Sender},
    time::sleep,
};
use tracing::Level;

use watch_file::keep_watching_file;

mod watch_file;

#[main]
pub async fn run(data_dir: impl AsRef<Path>) -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let checkpoint_path = checkpoint_path(&data_dir);
    let (mut fs_event_receiver, fs_watch_thread) = {
        let (sender, receiver) = channel(1);
        let thread = spawn(keep_watching_file(checkpoint_path, sender));
        (receiver, thread)
    };

    while let Some(event) = fs_event_receiver.recv().await {
        warn!("Got event: {:?}.", event);
    }

    fs_watch_thread.await?;

    Ok(())
}
