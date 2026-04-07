use actix_web::{get, post, web, HttpResponse, Responder, Error};
use actix_ws::Message;
use std::sync::Arc;
use tokio::sync::RwLock;
use futures::StreamExt;

use crate::BitFunnelIndex;
use crate::integrations::common::*;

/// Shared state for the API server
pub type AppState = Arc<RwLock<BitFunnelIndex>>;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/")
            .route(web::get().to(serve_index))
    );

    let mut scope = web::scope("/api")
        .service(search)
        .service(index_file)
        .service(index_document);

    #[cfg(feature = "postgres")]
    {
        scope = scope.service(index_postgres);
    }

    #[cfg(feature = "s3")]
    {
        scope = scope.service(index_s3);
    }

    cfg.service(
        scope
            .service(get_stats)
            .service(health_check)
    )
    .service(web::resource("/ws/search").route(web::get().to(ws_handler)));
}

/// Serve the search UI
async fn serve_index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(include_str!("../../static/index.html"))
}

/// Health check endpoint
#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "bitfunnel"
    }))
}

/// Get statistics about the index
#[get("/stats")]
async fn get_stats(state: web::Data<AppState>) -> impl Responder {
    let index = state.read().await;
    HttpResponse::Ok().json(serde_json::json!({
        "document_count": index.document_count(),
        "status": "ok"
    }))
}

/// Search endpoint
#[post("/search")]
async fn search(
    state: web::Data<AppState>,
    request: web::Json<SearchRequest>,
) -> impl Responder {
    let index_lock = Arc::clone(&state);

    let results = tokio::task::spawn_blocking(move || {
        let index = index_lock.blocking_read();
        index.search(&request.query)
    })
    .await;

    match results {
        Ok(results) => {
            let result_dtos: Vec<SearchResultDto> = results.iter().map(SearchResultDto::from).collect();
            HttpResponse::Ok().json(SearchResponse {
                count: result_dtos.len(),
                results: result_dtos,
            })
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

/// WebSocket handler for real-time search
async fn ws_handler(
    req: actix_web::HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let state = Arc::clone(&state);

    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.next().await {
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
                                if session.text(json).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    Ok(res)
}

/// Index a file from the filesystem
#[post("/index/file")]
async fn index_file(
    state: web::Data<AppState>,
    request: web::Json<IndexFileRequest>,
) -> impl Responder {
    let mut index = state.write().await;

    match index.index_file(&request.path) {
        Ok(doc_id) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "document_id": doc_id,
            "path": request.path
        })),
        Err(e) => {
            eprintln!("Error indexing file: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Index document content directly
#[post("/index/document")]
async fn index_document(
    state: web::Data<AppState>,
    request: web::Json<IndexDocumentRequest>,
) -> impl Responder {
    let mut index = state.write().await;

    match index.index_document(request.path.clone(), request.content.clone()) {
        Ok(doc_id) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "document_id": doc_id,
            "path": request.path
        })),
        Err(e) => {
            eprintln!("Error indexing document: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Index a Postgres table
#[cfg(feature = "postgres")]
#[post("/index/postgres")]
async fn index_postgres(
    state: web::Data<AppState>,
    request: web::Json<IndexPostgresRequest>,
) -> impl Responder {
    use crate::datasource::postgres::PostgresCrawler;
    let crawler_res = PostgresCrawler::new(&request.connection_string).await;

    match crawler_res {
        Ok(crawler) => {
            let mut index = state.write().await;
            let text_columns: Vec<&str> = request.text_columns.iter().map(|s| s.as_str()).collect();

            match crawler.crawl_table(&mut index, &request.table, &request.id_column, &text_columns).await {
                Ok(count) => HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "indexed_count": count,
                    "table": request.table
                })),
                Err(e) => {
                    eprintln!("Error indexing postgres: {}", e);
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        Err(e) => {
            eprintln!("Error connecting to postgres: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Index an S3 bucket
#[cfg(feature = "s3")]
#[post("/index/s3")]
async fn index_s3(
    state: web::Data<AppState>,
    request: web::Json<IndexS3Request>,
) -> impl Responder {
    use crate::datasource::s3::S3Crawler;
    let crawler_res = S3Crawler::new(request.region.clone()).await;

    match crawler_res {
        Ok(crawler) => {
            let mut index = state.write().await;

            match crawler.crawl_bucket(&mut index, &request.bucket, request.prefix.as_deref()).await {
                Ok(count) => HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "indexed_count": count,
                    "bucket": request.bucket
                })),
                Err(e) => {
                    eprintln!("Error indexing S3: {}", e);
                    HttpResponse::InternalServerError().finish()
                }
            }
        }
        Err(e) => {
            eprintln!("Error creating S3 crawler: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}
