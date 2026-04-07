use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use crate::BitFunnelIndex;
pub use crate::integrations::common::*;

/// Shared state for the API server
pub type AppState = Arc<RwLock<BitFunnelIndex>>;

/// Create the API router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/api/search", post(search))
        .route("/api/index/file", post(index_file))
        .route("/api/index/document", post(index_document))
        .route("/api/stats", get(get_stats))
        .route("/api/health", get(health_check))
        .route("/ws/search", get(ws_handler))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}

/// Serve the search UI
async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../../static/index.html"))
}

/// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "bitfunnel"
    }))
}

/// Get statistics about the index
async fn get_stats(State(state): State<AppState>) -> Json<serde_json::Value> {
    let index = state.read().await;
    Json(serde_json::json!({
        "document_count": index.document_count(),
        "status": "ok"
    }))
}

/// Search endpoint
async fn search(
    State(state): State<AppState>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let index_lock = Arc::clone(&state);

    let results = tokio::task::spawn_blocking(move || {
        let index = index_lock.blocking_read();
        index.search(&request.query)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result_dtos: Vec<SearchResultDto> = results.iter().map(SearchResultDto::from).collect();

    Ok(Json(SearchResponse {
        count: result_dtos.len(),
        results: result_dtos,
    }))
}

/// WebSocket handler for real-time search
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    if let Ok(request) = serde_json::from_str::<SearchRequest>(&text) {
                        let index_lock = Arc::clone(&state);
                        let results = tokio::task::spawn_blocking(move || {
                            let index = index_lock.blocking_read();
                            index.search(&request.query)
                        })
                        .await;

                        if let Ok(results) = results {
                            let result_dtos: Vec<SearchResultDto> =
                                results.iter().map(SearchResultDto::from).collect();
                            let response = SearchResponse {
                                count: result_dtos.len(),
                                results: result_dtos,
                            };
                            if let Ok(json) = serde_json::to_string(&response) {
                                if socket.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        } else {
            break;
        }
    }
}

/// Index a file from the filesystem
async fn index_file(
    State(state): State<AppState>,
    Json(request): Json<IndexFileRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut index = state.write().await;

    match index.index_file(&request.path) {
        Ok(doc_id) => Ok(Json(serde_json::json!({
            "success": true,
            "document_id": doc_id,
            "path": request.path
        }))),
        Err(e) => {
            eprintln!("Error indexing file: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Index document content directly
async fn index_document(
    State(state): State<AppState>,
    Json(request): Json<IndexDocumentRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut index = state.write().await;

    match index.index_document(std::path::PathBuf::from(&request.path), request.content) {
        Ok(doc_id) => Ok(Json(serde_json::json!({
            "success": true,
            "document_id": doc_id,
            "path": request.path
        }))),
        Err(e) => {
            eprintln!("Error indexing document: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
