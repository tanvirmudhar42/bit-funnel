use bitfunnel::api::{create_router, AppState};
use bitfunnel::BitFunnelIndex;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the BitFunnel index
    let mut index = BitFunnelIndex::with_defaults();

    // Index some files if fixtures exist
    let fixtures_path = Path::new("fixtures/files");
    if fixtures_path.exists() {
        println!("Indexing fixtures from fixtures/files...");
        let mut count = 0;
        for entry in std::fs::read_dir(fixtures_path)? {
            let entry = entry?;
            if entry.path().is_file() {
                index.index_file(entry.path())?;
                count += 1;
                if count >= 100 {
                    break;
                } // Index first 100 for speed
            }
        }
        println!("Indexed {} files.", count);
    } else {
        println!("Fixtures not found. Please run 'cargo run --bin generate-fixtures' first.");
        println!("Indexing current directory as fallback...");
        index.index_file("README.md")?;
        index.index_file("Cargo.toml")?;
    }

    let state: AppState = Arc::new(RwLock::new(index));

    // Create the router with CORS enabled for UI integration
    let app = create_router(state).layer(CorsLayer::permissive());

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    println!("Axum BitFunnel server running on http://localhost:3001");
    println!("Open this URL in your browser to use the search UI.");

    axum::serve(listener, app).await?;

    Ok(())
}
