use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use itertools::Itertools;

use self::read_rules::RulesMap;

use super::*;

mod error;

use error::AppError;

#[instrument(skip(query_server_ref))]
pub async fn serve(port: &str, query_server_ref: Ref<RuleServer>) -> Result<()> {
    info!("Starting server.");
    let app = Router::new().route("/", get(home_handler)).route(
        "/api/recommend",
        post(|request| async move { query_handler(request, query_server_ref.clone()).await }),
    );
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn home_handler() -> &'static str {
    info!("Requested /.");
    "/"
}

async fn query_handler(
    Json(request): Json<RecommendationRequest>,
    mut query_server_ref: Ref<RuleServer>,
) -> Result<Json<RecommendationResponse>, AppError> {
    info!(?request);
    let rules_map = query_server_ref.call(()).await?;

    let songs = recommend_songs(request.songs, &rules_map);
    let response = RecommendationResponse::new(songs, rules_map.2.to_string());
    Ok(Json(response))
}

#[derive(Clone, Debug, Deserialize)]
pub struct RecommendationRequest {
    pub songs: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecommendationResponse {
    pub playlist_ids: Vec<String>,
    pub version: &'static str,
    pub model_date: String,
}

impl RecommendationResponse {
    pub fn new(playlist_ids: Vec<String>, model_date: String) -> Self {
        Self {
            playlist_ids,
            version: crate_version!(),
            model_date,
        }
    }
}

#[instrument(skip(rules_map))]
fn recommend_songs(mut query: Vec<String>, rules_map: &RulesMap) -> Vec<String> {
    query.sort_unstable();
    query.dedup();
    let mut response = HashSet::with_capacity(MAX_LENGTH * 2);

    'combinations: for length in (1..(query.len().min(MAX_LENGTH) + 1)).rev() {
        for mut combination in query.iter().cloned().combinations(length) {
            combination.sort_unstable();
            if let Some(predictions) = rules_map.1.get(&combination) {
                response.extend(predictions);
                if response.len() >= MAX_LENGTH {
                    debug!(length, "Got enough predictions.");
                    break 'combinations;
                }
            }
        }
    }

    debug!("Sending response.");
    response.into_iter().cloned().collect()
}
