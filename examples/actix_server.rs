use actix_web::{web, App, HttpServer};
use bitfunnel::integrations::actix::{configure, AppState};
use bitfunnel::BitFunnelIndex;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize the BitFunnel index
    let mut index = BitFunnelIndex::with_defaults();

    // Index some files if fixtures exist
    let fixtures_path = Path::new("fixtures/files");
    if fixtures_path.exists() {
        println!("Indexing fixtures from fixtures/files...");
        let mut count = 0;
        for entry in std::fs::read_dir(fixtures_path)? {
            let entry = entry.unwrap();
            if entry.path().is_file() {
                index.index_file(entry.path()).unwrap();
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
        index.index_file("README.md").unwrap();
        index.index_file("Cargo.toml").unwrap();
    }

    let state: AppState = Arc::new(RwLock::new(index));
    let data = web::Data::new(state);

    println!("Actix BitFunnel server running on http://localhost:3002");
    println!("Open this URL in your browser to use the search UI.");

    HttpServer::new(move || App::new().app_data(data.clone()).configure(configure))
        .bind(("0.0.0.0", 3002))?
        .run()
        .await
}
