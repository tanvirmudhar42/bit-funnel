use bitfunnel::BitFunnelIndex;

fn main() -> anyhow::Result<()> {
    println!("Testing BitFunnel substring/contains search...\n");

    // Create index
    let mut index = BitFunnelIndex::with_defaults();

    // Index some test documents
    println!("Indexing test documents...");
    index.index_document(
        "doc1.txt".into(),
        "Rust is a systems programming language.".to_string(),
    )?;

    index.index_document(
        "doc2.txt".into(),
        "Python programming is simple and powerful.".to_string(),
    )?;

    index.index_document(
        "doc3.txt".into(),
        "JavaScript is used for web development.".to_string(),
    )?;

    println!("Indexed {} documents\n", index.document_count());

    // Test cases
    let test_cases = vec![
        ("prog", "Should match 'programming'"),
        ("gram", "Should match 'programming'"),
        ("rust", "Should match 'Rust'"),
        ("java", "Should match 'JavaScript'"),
        ("script", "Should match 'JavaScript'"),
        ("web", "Should match 'web'"),
    ];

    for (query, description) in test_cases {
        println!("Query: '{}' - {}", query, description);
        let results = index.search(query);

        if results.is_empty() {
            println!("  ❌ No results found");
        } else {
            println!("  ✅ Found {} result(s):", results.len());
            for result in &results {
                println!(
                    "     - {} (score: {:.1}%)",
                    result.document.path, result.score
                );
            }
        }
        println!();
    }

    Ok(())
}
