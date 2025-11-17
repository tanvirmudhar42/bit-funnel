use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{BitFunnelIndex, SearchResult};

/// Shared state for the API server
pub type AppState = Arc<RwLock<BitFunnelIndex>>;

/// Request to index a file
#[derive(Deserialize)]
pub struct IndexFileRequest {
    pub path: String,
}

/// Request to index document content directly
#[derive(Deserialize)]
pub struct IndexDocumentRequest {
    pub path: String,
    pub content: String,
}

/// Search request
#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
}

/// Search response
#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResultDto>,
    pub count: usize,
}

/// Document DTO for API responses
#[derive(Serialize, Clone)]
pub struct SearchResultDto {
    pub document_id: usize,
    pub score: f64,
    pub path: String,
    pub preview: String,
}

impl From<&SearchResult> for SearchResultDto {
    fn from(result: &SearchResult) -> Self {
        // Create a preview of the document content (first 200 chars)
        let preview = if result.document.content.len() > 200 {
            format!("{}...", &result.document.content[..200])
        } else {
            result.document.content.clone()
        };

        Self {
            document_id: result.document_id,
            score: result.score,
            path: result.document.path.to_string_lossy().to_string(),
            preview,
        }
    }
}

/// Create the API router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/search", post(search))
        .route("/api/index/file", post(index_file))
        .route("/api/index/document", post(index_document))
        .route("/api/stats", get(get_stats))
        .route("/api/health", get(health_check))
        .with_state(state)
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

/// Search endpoint - supports incremental search as user types
async fn search(
    State(state): State<AppState>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let index = state.read().await;
    let results = index.search(&request.query);
    
    let result_dtos: Vec<SearchResultDto> = results.iter().map(SearchResultDto::from).collect();
    
    Ok(Json(SearchResponse {
        count: result_dtos.len(),
        results: result_dtos,
    }))
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
    
    match index.index_document(
        std::path::PathBuf::from(&request.path),
        request.content,
    ) {
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

