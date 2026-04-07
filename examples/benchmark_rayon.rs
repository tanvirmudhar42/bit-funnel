use bitfunnel::BitFunnelIndex;
use std::path::Path;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let mut index = BitFunnelIndex::with_defaults();

    let fixtures_dir = Path::new("fixtures/files");
    if !fixtures_dir.exists() {
        println!("Fixtures directory not found. Please run generate-fixtures first.");
        return Ok(());
    }

    println!("--- Rayon Implementation ---");
    println!("Indexing documents from fixtures/files...");
    let start = Instant::now();
    let mut count = 0;
    for entry in std::fs::read_dir(fixtures_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            index.index_file(&path)?;
            count += 1;
        }
    }
    let duration = start.elapsed();
    println!("Indexed {} documents in {:?}", count, duration);

    let queries = [
        "rust",
        "programming",
        "performance",
        "search engine",
        "bitfunnel algorithm",
        "distributed systems",
        "memory safety",
        "concurrency model",
        "zero cost abstractions",
        "package manager",
    ];

    println!("\nBenchmarking searches...");
    for query in &queries {
        let start = Instant::now();
        let results = index.search(query);
        let duration = start.elapsed();
        println!(
            "Search for '{}' found {} results in {:?}",
            query,
            results.len(),
            duration
        );
    }

    // Warm up
    for _ in 0..10 {
        for query in &queries {
            index.search(query);
        }
    }

    println!("\nTimed searches (average of 100 runs):");
    for query in &queries {
        let start = Instant::now();
        for _ in 0..100 {
            index.search(query);
        }
        let average = start.elapsed() / 100;
        println!("Search for '{}': {:?}", query, average);
    }

    Ok(())
}
