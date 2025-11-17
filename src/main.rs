use bitfunnel::api::{create_router, AppState};
use bitfunnel::BitFunnelIndex;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the BitFunnel index
    let index = BitFunnelIndex::with_defaults();
    let state: AppState = Arc::new(RwLock::new(index));

    // Create the router with CORS enabled for UI integration
    let app = create_router(state)
        .layer(CorsLayer::permissive()); // Allow all origins for development

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("BitFunnel server running on http://0.0.0.0:3000");
    println!("API endpoints:");
    println!("  POST /api/search - Search for documents");
    println!("  POST /api/index/file - Index a file from filesystem");
    println!("  POST /api/index/document - Index document content");
    println!("  GET  /api/stats - Get index statistics");
    println!("  GET  /api/health - Health check");

    axum::serve(listener, app).await?;

    Ok(())
}

