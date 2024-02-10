use chrono::NaiveDateTime;
use itertools::Itertools;

use super::*;

use watch_file::FileWatcher;

#[instrument(skip(data_dir, query_sender, query_receiver))]
pub async fn rule_query_server(
    data_dir: PathBuf,
    mut query_sender: Sender<QueryServerMsg>,
    mut query_receiver: Receiver<QueryServerMsg>,
) {
    let checkpoint_path = checkpoint_path(&data_dir);
    info!(?data_dir, ?checkpoint_path);

    let (file_watcher_handle, mut file_watcher_ref) =
        FileWatcher::new(data_dir.clone(), query_sender.clone()).spawn();

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

    file_watcher_ref.cancel();

    let abort_handle = file_watcher_handle.abort_handle();
    match timeout(FIVE_SECONDS, file_watcher_handle).await {
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
    Query(Vec<String>, Sender<(Vec<String>, Arc<str>)>),
    WatchedFileChanged(Instant),
    NewCheckpoint(i64),
    NewRules {
        timestamp: i64,
        rules_map: Arc<HashMap<Vec<String>, HashSet<String>>>,
        when: Instant,
    },
    ReadRules(Instant),
    Exit,
}

async fn try_serve_queries(
    checkpoint_path: &Path,
    rules_path: &Path,
    query_receiver: &mut Receiver<QueryServerMsg>,
    query_sender: &mut Sender<QueryServerMsg>,
) -> Result<()> {
    let mut last_check = Instant::now();
    let (mut current_timestamp, mut current_rules_map) = read_rules(checkpoint_path, rules_path)?;
    let mut timestamp_checked = current_timestamp;
    let mut data_datetime = NaiveDateTime::from_timestamp_nanos(current_timestamp)
        .unwrap()
        .to_string()
        .into();

    while let Some(message) = query_receiver.recv().await {
        match message {
            QueryServerMsg::Query(query, response_sender) => {
                drop(spawn(answer_query(
                    query,
                    Arc::clone(&current_rules_map),
                    Arc::clone(&data_datetime),
                    response_sender,
                )));
                continue;
            }
            QueryServerMsg::WatchedFileChanged(when_changed) if when_changed > last_check => {
                info!(?when_changed, "File changed.");
                let checkpoint_path = checkpoint_path.to_owned();
                let query_sender = query_sender.clone();
                spawn(async move {
                    if let Err(why) =
                        check_checkpoint(&checkpoint_path, current_timestamp, &query_sender).await
                    {
                        error!(?why, "Failed to check checkpoint.");
                        sleep(FIVE_SECONDS).await;
                        query_sender
                            .send(QueryServerMsg::WatchedFileChanged(when_changed))
                            .await
                            .unwrap();
                    }
                });
            }
            QueryServerMsg::WatchedFileChanged(_) => {}
            QueryServerMsg::NewCheckpoint(timestamp) if timestamp > timestamp_checked => {
                info!(?timestamp, "New checkpoint.");
                timestamp_checked = timestamp;
                query_sender
                    .send(QueryServerMsg::ReadRules(Instant::now()))
                    .await
                    .expect("I own an open receiver.");
            }
            QueryServerMsg::NewCheckpoint(_) => {}
            QueryServerMsg::NewRules {
                timestamp,
                rules_map,
                when,
            } => {
                if timestamp > current_timestamp {
                    info!(?timestamp, "New rules.");
                    current_timestamp = timestamp;
                    current_rules_map = rules_map;
                    last_check = when;

                    timestamp_checked = timestamp;
                    data_datetime = NaiveDateTime::from_timestamp_nanos(current_timestamp)
                        .unwrap()
                        .to_string()
                        .into();
                    info!(?data_datetime);
                }
            }
            QueryServerMsg::ReadRules(when) if when > last_check => {
                info!(?when, "Reading rules.");
                last_check = when;

                let checkpoint_path = checkpoint_path.to_owned();
                let rules_path = rules_path.to_owned();
                let query_sender = query_sender.clone();
                drop(spawn(async move {
                    if let Err(why) = update_rules(
                        &checkpoint_path,
                        &rules_path,
                        current_timestamp,
                        &query_sender,
                    )
                    .await
                    {
                        error!(?why, "Failed to update rules.");
                        let when_fail = Instant::now();
                        sleep(FIVE_SECONDS).await;
                        let retry_event = QueryServerMsg::ReadRules(when_fail);
                        query_sender.send(retry_event).await.unwrap()
                    }
                }));
            }
            QueryServerMsg::ReadRules(_) => {}
            QueryServerMsg::Exit => {
                info!("Got exit message.");
                break;
            }
        }
    }

    warn!("Exiting.");
    Ok(())
}

async fn check_checkpoint(
    checkpoint_path: &Path,
    current_timestamp: i64,
    query_sender: &Sender<QueryServerMsg>,
) -> Result<()> {
    let timestamp = checkpoint_timestamp(checkpoint_path).context("Checkpoint timestamp")?;
    if timestamp > current_timestamp {
        _ = query_sender
            .clone()
            .send(QueryServerMsg::NewCheckpoint(timestamp))
            .await
    }

    Ok(())
}

async fn update_rules(
    checkpoint_path: &Path,
    rules_path: &Path,
    old_timestamp: i64,
    query_sender: &Sender<QueryServerMsg>,
) -> Result<()> {
    let timestamp = checkpoint_timestamp(checkpoint_path).context("Checkpoint timestamp")?;
    if timestamp != old_timestamp {
        let (timestamp, rules_map) =
            read_rules(checkpoint_path, rules_path).context("Failed to read rules from file.")?;
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
) -> Result<(i64, Arc<HashMap<Vec<String>, HashSet<String>>>)> {
    let timestamp = checkpoint_timestamp(checkpoint_path).context("Checkpoint timestamp")?;
    let rules_map = make_rules_map(rules_path).context("Read rules from file")?;
    let rules_map = Arc::new(rules_map);
    Ok((timestamp, rules_map))
}

#[instrument(skip(rules_map, response_sender))]
async fn answer_query(
    mut query: Vec<String>,
    rules_map: Arc<HashMap<Vec<String>, HashSet<String>>>,
    datetime: Arc<str>,
    response_sender: Sender<(Vec<String>, Arc<str>)>,
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

    debug!("Sending response.");
    _ = response_sender
        .send((response.into_iter().cloned().collect(), datetime))
        .await;
}

fn checkpoint_timestamp(checkpoint_path: impl AsRef<Path>) -> Result<i64> {
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
    info!(n_rules = rules.len(), "Read rules from file.");

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
