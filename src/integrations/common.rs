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
        const PREVIEW_CHARS: usize = 200;
        let char_count = result.document.content.chars().count();
        let preview = if char_count > PREVIEW_CHARS {
            let truncated: String = result.document.content.chars().take(PREVIEW_CHARS).collect();
            format!("{truncated}...")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn preview_truncates_without_breaking_utf8_boundaries() {
        let content = "é".repeat(201);
        let result = SearchResult {
            document_id: 1,
            score: 100.0,
            document: Arc::new(Document {
                id: 1,
                path: PathBuf::from("unicode.txt"),
                content,
                words: Vec::new(),
            }),
        };

        let dto = SearchResultDto::from(&result);
        assert!(dto.preview.ends_with("..."));
        assert_eq!(dto.preview.chars().count(), 203);
    }
}
