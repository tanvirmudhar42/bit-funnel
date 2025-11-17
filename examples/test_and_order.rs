use bitfunnel::BitFunnelIndex;

fn main() -> anyhow::Result<()> {
    println!("Testing BitFunnel AND + Order search...\n");

    // Create index
    let mut index = BitFunnelIndex::with_defaults();

    // Index test documents
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
        "Rust programming language is fast.".to_string(),
    )?;
    
    index.index_document(
        "doc4.txt".into(),
        "Programming in Rust is fun.".to_string(),
    )?;

    println!("Indexed {} documents\n", index.document_count());

    // Test cases
    let test_cases = vec![
        ("rust programming", vec!["doc1.txt", "doc3.txt"], "Should match docs with 'rust' before 'programming'"),
        ("programming rust", vec!["doc4.txt"], "Should match doc4 (programming before rust)"),
        ("python programming", vec!["doc2.txt"], "Should match doc with both words in order"),
        ("rust language", vec!["doc1.txt", "doc3.txt"], "Should match docs with 'rust' before 'language'"),
        ("programming language", vec!["doc1.txt", "doc3.txt"], "Should match docs with both words in order"),
        ("language rust", vec![], "Should NOT match (wrong order - language comes after rust)"),
    ];

    for (query, expected_files, description) in test_cases {
        println!("Query: '{}' - {}", query, description);
        let results = index.search(query);
        
        let result_files: Vec<String> = results.iter()
            .map(|r| r.document.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        
        if result_files.is_empty() && expected_files.is_empty() {
            println!("  ✅ Correctly found no results");
        } else if result_files.len() == expected_files.len() 
            && expected_files.iter().all(|f| result_files.contains(&f.to_string())) {
            println!("  ✅ Found {} result(s): {:?}", results.len(), result_files);
        } else {
            println!("  ❌ Expected: {:?}, Got: {:?}", expected_files, result_files);
        }
        println!();
    }

    Ok(())
}

