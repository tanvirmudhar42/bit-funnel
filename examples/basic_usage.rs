use bitfunnel::BitFunnelIndex;
use std::io;

fn main() -> anyhow::Result<()> {
    // Create a new BitFunnel index
    let mut index = BitFunnelIndex::with_defaults();

    println!("BitFunnel Index Example");
    println!("======================");
    println!();

    // Index some example documents
    println!("Indexing documents...");

    index.index_document(
        "doc1.txt".into(),
        "Rust is a systems programming language that runs blazingly fast.".to_string(),
    )?;

    index.index_document(
        "doc2.txt".into(),
        "Python is a high-level programming language known for its simplicity.".to_string(),
    )?;

    index.index_document(
        "doc3.txt".into(),
        "JavaScript is the language of the web, used for both frontend and backend.".to_string(),
    )?;

    println!("Indexed {} documents", index.document_count());
    println!();

    // Interactive search loop
    println!("Enter search queries (type 'quit' to exit):");
    loop {
        print!("Query: ");
        io::Write::flush(&mut io::stdout())?;

        let mut query = String::new();
        io::stdin().read_line(&mut query)?;
        let query = query.trim();

        if query == "quit" || query.is_empty() {
            break;
        }

        // Perform search
        let results = index.search(query);

        if results.is_empty() {
            println!("No results found for '{}'", query);
        } else {
            println!("\nFound {} result(s):", results.len());
            for (i, result) in results.iter().take(5).enumerate() {
                println!(
                    "{}. {} (score: {:.2})",
                    i + 1,
                    result.document.path,
                    result.score
                );
                // Show preview
                let preview = if result.document.content.len() > 60 {
                    &result.document.content[..60]
                } else {
                    &result.document.content
                };
                println!("   Preview: {}...", preview);
            }
        }
        println!();
    }

    Ok(())
}
