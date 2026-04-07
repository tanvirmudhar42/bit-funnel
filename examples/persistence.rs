use bitfunnel::BitFunnelIndex;

fn main() -> anyhow::Result<()> {
    let mut index = BitFunnelIndex::with_defaults();

    // Index some content
    index.index_document("doc1.txt".to_string(), "This is some test content about Rust.".to_string())?;
    index.index_document("doc2.txt".to_string(), "BitFunnel is a fast search algorithm.".to_string())?;

    let index_file = "test_index.json";

    // Save the index
    println!("Saving index to {}...", index_file);
    index.save_to_file(index_file)?;

    // Load the index into a new instance
    println!("Loading index from {}...", index_file);
    let loaded_index = BitFunnelIndex::load_from_file(index_file)?;

    println!("Document count in loaded index: {}", loaded_index.document_count());

    // Search the loaded index
    let results = loaded_index.search("Rust");
    for res in results {
        println!("Found: {} (Score: {:.1}%)", res.document.path, res.score);
    }

    // Cleanup
    std::fs::remove_file(index_file)?;

    Ok(())
}
