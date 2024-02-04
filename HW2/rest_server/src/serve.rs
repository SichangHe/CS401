use axum::{routing::get, Router};

use super::*;

#[instrument]
pub async fn serve() -> Result<()> {
    info!("Starting server.");
    let app = Router::new().route(
        "/",
        get(|| async {
            info!("Requested /.");
            "/"
        }),
    );
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
