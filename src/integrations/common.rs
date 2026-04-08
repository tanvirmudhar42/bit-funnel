use crate::SearchResult;
use serde::{Deserialize, Serialize};

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

/// Request to index a Postgres table
#[derive(Deserialize)]
pub struct IndexPostgresRequest {
    pub connection_string: String,
    pub table: String,
    pub id_column: String,
    pub text_columns: Vec<String>,
}

/// Request to index an S3 bucket
#[derive(Deserialize)]
pub struct IndexS3Request {
    pub bucket: String,
    pub region: Option<String>,
    pub prefix: Option<String>,
}

/// Request to save or load index
#[derive(Deserialize)]
pub struct PersistenceRequest {
    pub path: String,
}

/// Search request
#[derive(Deserialize, Serialize)]
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
            path: result.document.path.clone(),
            preview,
        }
    }
}
