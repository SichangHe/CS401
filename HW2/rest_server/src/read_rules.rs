use std::sync::Arc;

use itertools::Itertools;

use super::*;

use watch_file::keep_watching_file;

#[instrument(skip(query_sender, query_receiver))]
pub async fn rule_query_server(
    data_dir: PathBuf,
    mut query_sender: Sender<QueryServerMsg>,
    mut query_receiver: Receiver<QueryServerMsg>,
) {
    let checkpoint_path = checkpoint_path(&data_dir);
    let (fs_exit_sender, fs_watch_thread) = {
        let (exit_sender, exit_receiver) = channel(1);
        let thread = spawn(keep_watching_file(
            checkpoint_path.clone(),
            query_sender.clone(),
            exit_receiver,
        ));
        (exit_sender, thread)
    };

    let rules_path = rules_path(&data_dir);

    loop {
        match try_serve_queries(
            &checkpoint_path,
            &rules_path,
            &mut query_receiver,
            &mut query_sender,
        )
        .await
        {
            Ok(_) => break,
            Err(why) => error!(?why, "Failed to serve queries."),
        }
        sleep(FIVE_SECONDS).await;
    }

    _ = fs_exit_sender.send(()).await;
    let abort_handle = fs_watch_thread.abort_handle();
    match timeout(FIVE_SECONDS, fs_watch_thread).await {
        Ok(Ok(_)) => {}
        Ok(Err(why)) => error!(?why, "File watcher thread exited with error."),
        Err(_) => {
            abort_handle.abort();
            error!("File watcher thread took too long to exit. Aborted.");
        }
    }
}

#[derive(Clone, Debug)]
pub enum QueryServerMsg {
    Query(Vec<String>, Sender<Vec<String>>),
    WatchedFileChanged(Instant),
    NewRules {
        timestamp: u128,
        rules_map: Arc<HashMap<Vec<String>, HashSet<String>>>,
        when: Instant,
    },
    Exit,
}

async fn try_serve_queries(
    checkpoint_path: &Path,
    rules_path: &Path,
    query_receiver: &mut Receiver<QueryServerMsg>,
    query_sender: &mut Sender<QueryServerMsg>,
) -> Result<()> {
    let mut last_read = Instant::now();
    let (mut current_timestamp, mut current_rules_map) = read_rules(checkpoint_path, rules_path)?;

    while let Some(message) = query_receiver.recv().await {
        match message {
            QueryServerMsg::Query(query, response_sender) => {
                drop(spawn(answer_query(
                    // TODO: Send timestamp as well.
                    query,
                    Arc::clone(&current_rules_map),
                    response_sender,
                )));
                continue;
            }
            QueryServerMsg::WatchedFileChanged(when_changed) if when_changed > last_read => {
                let checkpoint_path = checkpoint_path.into();
                let rules_path = rules_path.into();
                let query_sender = query_sender.clone();
                drop(spawn(async move {
                    if let Err(why) =
                        update_rules(checkpoint_path, rules_path, current_timestamp, query_sender)
                            .await
                    {
                        error!(?why, "Failed to update rules.");
                    }
                }));
            }
            QueryServerMsg::WatchedFileChanged(_) => {}
            QueryServerMsg::NewRules {
                timestamp,
                rules_map,
                when,
            } => {
                if timestamp > current_timestamp {
                    current_timestamp = timestamp;
                    current_rules_map = rules_map;
                    last_read = when;
                }
            }
            QueryServerMsg::Exit => {
                debug!("Got exit message.");
                break;
            }
        }
    }

    warn!("Exiting.");
    Ok(())
}

async fn update_rules(
    checkpoint_path: PathBuf,
    rules_path: PathBuf,
    old_timestamp: u128,
    query_sender: Sender<QueryServerMsg>,
) -> Result<()> {
    let timestamp = checkpoint_timestamp(&checkpoint_path).context("Checkpoint timestamp")?;
    if timestamp != old_timestamp {
        let (timestamp, rules_map) =
            read_rules(&checkpoint_path, &rules_path).context("Failed to read rules from file.")?;
        _ = query_sender
            .clone()
            .send(QueryServerMsg::NewRules {
                timestamp,
                rules_map,
                when: Instant::now(),
            })
            .await
    }
    Ok(())
}

fn read_rules(
    checkpoint_path: &Path,
    rules_path: &Path,
) -> Result<(u128, Arc<HashMap<Vec<String>, HashSet<String>>>)> {
    let timestamp = checkpoint_timestamp(checkpoint_path).context("Checkpoint timestamp")?;
    let rules_map = make_rules_map(rules_path).context("Read rules from file")?;
    let rules_map = Arc::new(rules_map);
    Ok((timestamp, rules_map))
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

fn checkpoint_timestamp(checkpoint_path: impl AsRef<Path>) -> Result<u128> {
    read_file(&checkpoint_path)
        .with_context(|| format!("Read {:?}", checkpoint_path.as_ref()))?
        .split_whitespace()
        .nth(2)
        .context("No timestamp found")?
        .parse()
        .context("Failed to parse timestamp number")
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
