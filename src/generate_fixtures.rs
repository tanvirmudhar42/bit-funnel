use std::fs;
use std::io::Write;
use std::path::PathBuf;

// Common words for generating realistic text
const WORDS: &[&str] = &[
    "the", "be", "to", "of", "and", "a", "in", "that", "have", "i",
    "it", "for", "not", "on", "with", "he", "as", "you", "do", "at",
    "this", "but", "his", "by", "from", "they", "we", "say", "her", "she",
    "or", "an", "will", "my", "one", "all", "would", "there", "their", "what",
    "so", "up", "out", "if", "about", "who", "get", "which", "go", "me",
    "when", "make", "can", "like", "time", "no", "just", "him", "know", "take",
    "people", "into", "year", "your", "good", "some", "could", "them", "see", "other",
    "than", "then", "now", "look", "only", "come", "its", "over", "think", "also",
    "back", "after", "use", "two", "how", "our", "work", "first", "well", "way",
    "even", "new", "want", "because", "any", "these", "give", "day", "most", "us",
    "rust", "programming", "language", "system", "memory", "performance", "code", "function",
    "variable", "struct", "trait", "implementation", "error", "result", "option", "string",
    "vector", "hash", "map", "iterator", "closure", "async", "await", "thread",
    "concurrent", "parallel", "algorithm", "data", "structure", "binary", "search",
    "index", "query", "document", "text", "file", "directory", "path", "content",
];

// Topic-specific words for variety
const TOPICS: &[&[&str]] = &[
    &["computer", "software", "hardware", "network", "internet", "server", "client", "database"],
    &["science", "research", "experiment", "theory", "hypothesis", "analysis", "method", "result"],
    &["business", "company", "market", "product", "service", "customer", "revenue", "profit"],
    &["education", "student", "teacher", "school", "university", "course", "study", "learn"],
    &["technology", "innovation", "development", "design", "engineering", "solution", "project"],
];

fn main() -> std::io::Result<()> {
    let fixtures_dir = PathBuf::from("fixtures/files");
    
    // Create directory if it doesn't exist
    fs::create_dir_all(&fixtures_dir)?;
    
    println!("Generating 500 text files with 10,000+ words each...");
    println!("Output directory: {}", fixtures_dir.display());
    
    let mut rng = SimpleRng::new(12345); // Seed for reproducibility
    
    for i in 1..=500 {
        let filename = format!("file_{:04}.txt", i);
        let filepath = fixtures_dir.join(&filename);
        
        // Generate content with at least 10,000 words
        let content = generate_text(&mut rng, 10000 + (i % 1000)); // Vary between 10k-11k words
        
        // Write file
        let mut file = fs::File::create(&filepath)?;
        file.write_all(content.as_bytes())?;
        
        if i % 50 == 0 {
            println!("Generated {} files...", i);
        }
    }
    
    println!("Done! Generated 500 files in {}", fixtures_dir.display());
    Ok(())
}

fn generate_text(rng: &mut SimpleRng, target_words: usize) -> String {
    let mut text = String::new();
    let mut word_count = 0;
    
    // Choose a topic for this file
    let topic_words = TOPICS[rng.next() as usize % TOPICS.len()];
    
    // Generate paragraphs
    while word_count < target_words {
        // Paragraph length: 50-150 words
        let para_length = 50 + (rng.next() as usize % 100);
        
        // Add some topic-specific words
        if rng.next() % 3 == 0 {
            text.push_str(&capitalize(topic_words[rng.next() as usize % topic_words.len()]));
            text.push(' ');
            word_count += 1;
        }
        
        // Generate paragraph
        for _ in 0..para_length {
            if word_count >= target_words {
                break;
            }
            
            let word = WORDS[rng.next() as usize % WORDS.len()];
            text.push_str(word);
            text.push(' ');
            word_count += 1;
            
            // Add punctuation occasionally
            if rng.next() % 15 == 0 && word_count < target_words {
                match rng.next() % 3 {
                    0 => text.push_str(". "),
                    1 => text.push_str(", "),
                    _ => text.push(' '),
                }
            }
        }
        
        // End paragraph
        text.push_str(".\n\n");
    }
    
    text
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

// Simple PRNG for generating text
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    
    fn next(&mut self) -> u64 {
        // Linear congruential generator
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        self.state
    }
}

