use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

use super::*;

mod error;

use error::AppError;

#[instrument(skip(query_sender))]
pub async fn serve(port: &str, query_sender: Sender<QueryServerMsg>) -> Result<()> {
    info!("Starting server.");
    let app = Router::new().route("/", get(home_handler)).route(
        "/api/recommend",
        post(|request| async move { query_handler(request, query_sender.clone()).await }),
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
    query_sender: Sender<QueryServerMsg>,
) -> Result<Json<RecommendationResponse>, AppError> {
    info!(?request);
    let (response_sender, mut response_receiver) = channel(1);
    let query = QueryServerMsg::Query(request.songs, response_sender);
    query_sender.send(query).await?;
    let (playlist_ids, datetime) = response_receiver
        .recv()
        .await
        .context("No response from query server")?;
    let response = RecommendationResponse::new(playlist_ids, datetime.to_string());
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
