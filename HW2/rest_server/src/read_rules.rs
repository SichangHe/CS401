use chrono::NaiveDateTime;

use super::*;

use watch_file::FileWatcher;

pub struct RuleServer {
    data_dir: PathBuf,
    checkpoint_path: PathBuf,
    rules_path: PathBuf,
    file_watcher: Option<(JoinHandle<Result<()>>, Ref<FileWatcher>)>,
    last_check: Instant,
    rules_map: Option<Arc<RulesMap>>,
    timestamp_checked: i64,
}

impl RuleServer {
    #[instrument]
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            checkpoint_path: checkpoint_path(&data_dir),
            rules_path: rules_path(&data_dir),
            data_dir,
            file_watcher: None,
            last_check: Instant::now(),
            rules_map: None,
            timestamp_checked: i64::MIN,
        }
    }

    pub fn try_spawn_file_watcher(&mut self, env: Ref<Self>) -> Result<()> {
        let cancellation_token = env.cancellation_token.child_token();
        let file_watcher = FileWatcher::new(self.data_dir.clone(), env);
        let file_watcher = file_watcher.spawn_with_token(cancellation_token);
        self.file_watcher = Some(file_watcher);
        Ok(())
    }
}

impl Actor for RuleServer {
    type CallMsg = ();
    type CastMsg = RuleServerMsg;
    type Reply = Arc<RulesMap>;

    async fn init(&mut self, env: &mut Ref<Self>) -> Result<()> {
        env.cast(RuleServerMsg::InitFileWatcher).await?;
        env.cast(RuleServerMsg::ReadRules(Instant::now())).await?;
        Ok(())
    }

    #[instrument(skip(self, msg, env))]
    async fn handle_cast(&mut self, msg: Self::CastMsg, env: &mut Ref<Self>) -> Result<()> {
        match msg {
            RuleServerMsg::InitFileWatcher => {
                if let Err(why) = self.try_spawn_file_watcher(env.clone()) {
                    error!(?why, "Failed to spawn file watcher, retrying after sleep.");

                    let mut env = env.clone();
                    drop(spawn(async move {
                        sleep(FIVE_SECONDS).await;
                        env.cast(RuleServerMsg::InitFileWatcher).await
                    }))
                }
            }

            RuleServerMsg::WatchedFileChanged(when) if when > self.last_check => {
                info!(?when, "File changed.");
                self.last_check = when;
                if let Some(rules_map) = &self.rules_map {
                    drop(spawn(check_checkpoint_or_retry(
                        self.checkpoint_path.clone(),
                        rules_map.0,
                        env.clone(),
                    )));
                }
            }
            RuleServerMsg::WatchedFileChanged(_) => {}

            RuleServerMsg::NewCheckpoint(timestamp) if timestamp > self.timestamp_checked => {
                info!(?timestamp, "New checkpoint.");
                self.timestamp_checked = timestamp;
                let read_event = RuleServerMsg::ReadRules(Instant::now());
                _ = env.cast(read_event).await;
            }
            RuleServerMsg::NewCheckpoint(_) => {}

            RuleServerMsg::ReadRules(when) if when > self.last_check => {
                info!(?when, "Reading rules.");
                self.last_check = when;

                drop(spawn(update_rules_or_retry(
                    self.checkpoint_path.clone(),
                    self.rules_path.clone(),
                    self.rules_map.as_ref().map_or(i64::MIN, |r| r.0),
                    env.clone(),
                )));
            }
            RuleServerMsg::ReadRules(_) => {}

            RuleServerMsg::NewRules { rules_map, when } => match self.rules_map.as_ref() {
                Some(current_map) if rules_map.0 <= current_map.0 => {}
                _ => {
                    let new_datetime = &rules_map.2;
                    info!(?new_datetime, "New rules.");

                    self.timestamp_checked = rules_map.0;
                    self.rules_map = Some(Arc::new(rules_map));
                    self.last_check = when;
                }
            },
        }

        Ok(())
    }

    #[instrument(skip(self, msg, env, response_sender))]
    async fn handle_call(
        &mut self,
        msg: Self::CallMsg,
        env: &mut Ref<Self>,
        response_sender: oneshot::Sender<Self::Reply>,
    ) -> Result<()> {
        match &self.rules_map {
            Some(rules_map) => _ = response_sender.send(Arc::clone(rules_map)),
            None => {
                warn!("Deferring query reply.");

                let mut env = env.clone();
                drop(spawn(async move {
                    sleep(FIVE_SECONDS).await;
                    _ = env.relay_call(msg, response_sender).await;
                }));
            }
        }

        Ok(())
    }
}

pub struct RulesMap(
    pub i64,
    pub HashMap<Vec<String>, HashSet<String>>,
    pub String,
);

impl RulesMap {
    pub fn new(timestamp: i64, rules_map: HashMap<Vec<String>, HashSet<String>>) -> Self {
        let data_datetime = NaiveDateTime::from_timestamp_nanos(timestamp)
            .unwrap()
            .to_string();
        Self(timestamp, rules_map, data_datetime)
    }
}

pub enum RuleServerMsg {
    InitFileWatcher,
    WatchedFileChanged(Instant),
    NewCheckpoint(i64),
    ReadRules(Instant),
    NewRules { rules_map: RulesMap, when: Instant },
}

async fn check_checkpoint_or_retry(
    checkpoint_path: PathBuf,
    old_timestamp: i64,
    mut server_ref: Ref<RuleServer>,
) {
    if let Err(why) = try_check_checkpoint(&checkpoint_path, old_timestamp, &mut server_ref).await {
        error!(?why, "Failed to check checkpoint.");

        let when_failed = Instant::now();
        sleep(FIVE_SECONDS).await;
        let retry_event = RuleServerMsg::WatchedFileChanged(when_failed);
        _ = server_ref.cast(retry_event).await
    }
}

async fn try_check_checkpoint(
    checkpoint_path: &Path,
    old_timestamp: i64,
    server_ref: &mut Ref<RuleServer>,
) -> Result<()> {
    let timestamp = checkpoint_timestamp(checkpoint_path).context("Checkpoint timestamp")?;
    if timestamp > old_timestamp {
        let checkpoint_event = RuleServerMsg::NewCheckpoint(timestamp);
        _ = server_ref.cast(checkpoint_event).await
    }

    Ok(())
}

async fn update_rules_or_retry(
    checkpoint_path: PathBuf,
    rules_path: PathBuf,
    old_timestamp: i64,
    mut server_ref: Ref<RuleServer>,
) {
    if let Err(why) = try_update_rules(
        &checkpoint_path,
        &rules_path,
        old_timestamp,
        &mut server_ref,
    )
    .await
    {
        error!(?why, "Failed to update rules.");

        let when_fail = Instant::now();
        sleep(FIVE_SECONDS).await;
        let retry_event = RuleServerMsg::ReadRules(when_fail);
        _ = server_ref.cast(retry_event).await
    }
}

async fn try_update_rules(
    checkpoint_path: &Path,
    rules_path: &Path,
    old_timestamp: i64,
    server_ref: &mut Ref<RuleServer>,
) -> Result<()> {
    let timestamp = checkpoint_timestamp(checkpoint_path).context("Checkpoint timestamp")?;
    if timestamp > old_timestamp {
        let when = Instant::now();
        let rules_map = make_rules_map(rules_path).context("Read rules from file")?;
        let new_rules_event = RuleServerMsg::NewRules {
            rules_map: RulesMap::new(timestamp, rules_map),
            when,
        };
        _ = server_ref.cast(new_rules_event).await;
    }
    Ok(())
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
