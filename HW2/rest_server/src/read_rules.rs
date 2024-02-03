use std::sync::Arc;

use itertools::Itertools;

use super::*;

use watch_file::keep_watching_file;

#[instrument(skip(query_receiver))]
pub async fn rule_query_server(
    data_dir: PathBuf,
    mut query_receiver: Receiver<(Vec<String>, Sender<Vec<String>>)>,
) {
    let checkpoint_path = checkpoint_path(&data_dir);
    let (mut fs_event_receiver, fs_exit_sender, fs_watch_thread) = {
        let (sender, receiver) = channel(1);
        let (exit_sender, exit_receiver) = channel(1);
        let thread = spawn(keep_watching_file(checkpoint_path, sender, exit_receiver));
        (receiver, exit_sender, thread)
    };

    let rules_path = rules_path(&data_dir);

    'make_rules: loop {
        let rules_map = match make_rules_map(&rules_path) {
            Ok(r) => r,
            Err(why) => {
                error!(?why, "Failed to read rules from file.");
                sleep(TEN_SECONDS).await;
                continue;
            }
        };
        let rules_map = Arc::new(rules_map);
        loop {
            select! {
                maybe_query = query_receiver.recv() => {
                    match maybe_query {
                        Some((query, response_sender)) => drop(spawn(answer_query(
                                query, Arc::clone(&rules_map), response_sender
                            ))),
                        None => {
                            warn!("Query sender is closed. Exiting.");
                            break 'make_rules;
                        },
                    }
                }
                maybe_fs_event = fs_event_receiver.recv() => {
                    let event = maybe_fs_event
                        .expect("File watcher should not exit before rule query server.");
                    warn!(?event, "Got file watcher event. Reloading rules.");
                    break;
                }
            }
        }
    }

    drop(fs_event_receiver);
    _ = fs_exit_sender.send(()).await;
    let abort_handle = fs_watch_thread.abort_handle();
    match timeout(TEN_SECONDS, fs_watch_thread).await {
        Ok(Ok(_)) => {}
        Ok(Err(why)) => error!(?why, "File watcher thread exited with error."),
        Err(_) => {
            abort_handle.abort();
            error!("File watcher thread took too long to exit. Aborted.");
        }
    }
}

#[instrument(skip(rules_map, response_sender))]
async fn answer_query(
    mut query: Vec<String>,
    rules_map: Arc<HashMap<Vec<String>, HashSet<String>>>,
    response_sender: Sender<Vec<String>>,
) {
    query.sort_unstable();
    query.dedup();
    let mut response = HashSet::with_capacity(MAX_LENGTH * 2);

    'combinations: for length in (1..(query.len().min(MAX_LENGTH) + 1)).rev() {
        for mut combination in query.iter().cloned().combinations(length) {
            combination.sort_unstable();
            if let Some(predictions) = rules_map.get(&combination) {
                response.extend(predictions);
                if response.len() >= MAX_LENGTH {
                    debug!(length, "Got enough predictions.");
                    break 'combinations;
                }
            }
        }
    }

    match response_sender
        .send(response.into_iter().cloned().collect())
        .await
    {
        Ok(_) => debug!("Sent response."),
        Err(_) => debug!("Sender stoped listening."),
    }
}

#[instrument]
fn make_rules_map(rules_path: &Path) -> Result<HashMap<Vec<String>, HashSet<String>>> {
    let file = File::open(rules_path)?;
    let rules: Vec<Rule> = bincode::deserialize_from(file)?;
    debug!(n_rules = rules.len(), "Read rules from file.");

    Ok(rules
        .into_iter()
        .map(
            |Rule {
                 antecedent,
                 consequent,
                 confidence: _,
                 lift: _,
             }| {
                let mut antecedent_vec: Vec<_> = antecedent.into_iter().collect();
                antecedent_vec.sort_unstable();
                (antecedent_vec, consequent)
            },
        )
        .collect())
}
