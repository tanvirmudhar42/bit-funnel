> ⚠️ **WARNING: RESEARCH PURPOSES ONLY** ⚠️
> 
> This project is intended for **research and educational purposes only**. It should **NOT be used in production environments**. The implementation may contain bugs, security vulnerabilities, or performance issues that make it unsuitable for production use. Use at your own risk.

# BitFunnel - Fast Incremental Keyword Search Backend

A Rust implementation of the BitFunnel algorithm for efficient, real-time keyword search in text files. This backend is designed to support incremental search as users type, making it ideal for UI applications that need fast search capabilities.

## Overview

BitFunnel uses bit-sliced signatures (similar to Bloom filters) to represent documents, enabling fast search operations through bitwise comparisons. The algorithm is optimized for:

- **Incremental Search**: Results update as the user types
- **Fast Queries**: Bitwise operations provide O(1) signature matching
- **Memory Efficiency**: Compact bit vector representations
- **Scalability**: Efficient handling of large document collections

## Features

- Bit-sliced signature indexing for fast document matching
- Incremental search support (updates as query changes)
- HTTP REST API for easy UI integration
- Relevance scoring based on bit matching and exact term matches
- Support for indexing files from filesystem or direct content

## Prerequisites

- **Rust**: Install Rust from [rustup.rs](https://rustup.rs/)
- **Cargo**: Comes with Rust installation

## Installation

Clone the repository and build the project:

```bash
git clone <repository-url>
cd bitfunnel
cargo build --release
```

This will create the following binaries in `target/release/`:
- `bitfunnel` - HTTP API server
- `bitfunnel-cli` - Interactive CLI search tool
- `generate-fixtures` - Test data generator

## Usage

### Quick Start

1. **Generate test data** (optional):
   ```bash
   cargo run --release --bin generate-fixtures
   ```
   This creates 500 test files in `fixtures/files/` for testing.

2. **Run the CLI**:
   ```bash
   cargo run --release --bin bitfunnel-cli -- --path fixtures/files --recursive
   ```

3. **Or start the API server**:
   ```bash
   cargo run --release
   ```

### Interactive CLI Tool

The CLI tool provides an interactive, real-time search interface where results update as you type.

#### Basic Usage

```bash
# Search files in current directory
cargo run --release --bin bitfunnel-cli

# Search files in a specific directory
cargo run --release --bin bitfunnel-cli -- --path /path/to/files

# Recursively search subdirectories
cargo run --release --bin bitfunnel-cli -- --path /path/to/files --recursive

# Only search specific file types
cargo run --release --bin bitfunnel-cli -- --path . --extensions txt,md,rs

# Load an existing index instead of re-indexing
cargo run --release --bin bitfunnel-cli -- --load my_index.json

# Save the index after scanning files
cargo run --release --bin bitfunnel-cli -- --path . --save my_index.json
```

#### Command Line Options

- `-p, --path <PATH>`: Directory or file to index (default: current directory)
- `-r, --recursive`: Recursively index subdirectories
- `-e, --extensions <EXTENSIONS>`: Comma-separated list of file extensions to include (e.g., `txt,rs,md`)
- `--save <FILE>`: Save the index to a file after indexing
- `--load <FILE>`: Load the index from a file instead of indexing
- `-h, --help`: Show help message

#### CLI Controls

- **Type**: Start typing to search - results update in real-time
- **↑/↓ Arrow Keys**: Navigate through search results
- **Enter**: View full content of the selected file
- **ESC**: Exit the application or go back from file view
- **q**: Quit the application
- **Backspace**: Delete characters from search query

#### CLI Features

- **Real-time search**: Results update instantly as you type
- **Interactive interface**: Full-screen terminal UI with color-coded results using ratatui
- **File preview**: See snippets of matching content with highlighted matches
- **Relevance scoring**: Results sorted by relevance score (0-100%)
- **Navigation**: Smooth scrolling and selection highlighting
- **File viewing**: Full-screen file content viewer

**Example:**
```
╔═══════════════════════════════════════════════════════════════╗
║                    BitFunnel Search CLI                      ║
╚═══════════════════════════════════════════════════════════════╝

Search: rust programming_
─────────────────────────────────────────────────────────────────

Found 3 result(s):

1. src/main.rs
   Score: 92.5% | ...This is a Rust programming example...

2. docs/tutorial.md
   Score: 87.3% | ...Learn Rust programming with this guide...

3. examples/demo.rs
   Score: 75.1% | ...Rust programming patterns...
```

### HTTP API Server

Start the API server to use BitFunnel as a backend for web applications:

```bash
cargo run --release
```

The server will start on `http://0.0.0.0:3000` with CORS enabled for easy frontend integration.

#### API Endpoints

The server provides the following REST endpoints:

- `POST /api/search` - Search for documents
- `POST /api/index/file` - Index a file from filesystem
- `POST /api/index/document` - Index document content directly
- `POST /api/index/postgres` - Index a Postgres table
- `POST /api/index/s3` - Index an S3 bucket
- `POST /api/index/save` - Save index to disk
- `POST /api/index/load` - Load index from disk
- `GET /api/stats` - Get index statistics
- `GET /api/health` - Health check

### API Examples

#### Search for Documents

```bash
curl -X POST http://localhost:3000/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "rust programming"}'
```

Response:
```json
{
  "results": [
    {
      "document_id": 0,
      "score": 85.5,
      "path": "/path/to/document.txt",
      "preview": "This document discusses Rust programming..."
    }
  ],
  "count": 1
}
```

#### Index a File

```bash
curl -X POST http://localhost:3000/api/index/file \
  -H "Content-Type: application/json" \
  -d '{"path": "/path/to/file.txt"}'
```

#### Index Document Content

```bash
curl -X POST http://localhost:3000/api/index/document \
  -H "Content-Type: application/json" \
  -d '{
    "path": "document.txt",
    "content": "This is the document content..."
  }'
```

#### Index a Postgres Table

```bash
curl -X POST http://localhost:3000/api/index/postgres \
  -H "Content-Type: application/json" \
  -d '{
    "connection_string": "postgres://user:pass@localhost/db",
    "table": "articles",
    "id_column": "id",
    "text_columns": ["title", "content"]
  }'
```

#### Index an S3 Bucket

```bash
curl -X POST http://localhost:3000/api/index/s3 \
  -H "Content-Type: application/json" \
  -d '{
    "bucket": "my-text-data-bucket",
    "region": "us-east-1",
    "prefix": "optional/folder/"
  }'
```

#### Persistence (Save/Load Index)

```bash
# Save index
curl -X POST http://localhost:3000/api/index/save \
  -H "Content-Type: application/json" \
  -d '{"path": "my_index.json"}'

# Load index
curl -X POST http://localhost:3000/api/index/load \
  -H "Content-Type: application/json" \
  -d '{"path": "my_index.json"}'
```

#### Get Statistics

```bash
curl http://localhost:3000/api/stats
```

Response:
```json
{
  "document_count": 42,
  "status": "ok"
}
```

#### Health Check

```bash
curl http://localhost:3000/api/health
```

Response:
```json
{
  "status": "ok",
  "service": "bitfunnel"
}
```

### Search Behavior

BitFunnel implements a sophisticated search algorithm with the following features:

1. **AND Operation**: All words in the query must be present in matching documents
   - Query: `"rust programming"` → Matches documents containing both "rust" AND "programming"

2. **Order Preservation**: Words must appear in the same order as in the query
   - Query: `"rust programming"` → Matches "Rust is a programming language" ✅
   - Query: `"programming rust"` → Does NOT match "Rust is a programming language" ❌

3. **Substring Matching**: Partial words are searchable
   - Query: `"prog"` → Matches documents containing "programming", "program", etc.
   - Query: `"rust lang"` → Matches documents containing "rust language", "rust lang", etc.

4. **Case Insensitive**: All searches are case-insensitive
   - Query: `"RUST"` matches "rust", "Rust", "RUST", etc.

5. **Relevance Scoring**: Results are ranked by relevance (0-100%)
   - Higher scores indicate better matches
   - Scoring considers both bit signature matches and exact term matches

## Library Usage

You can use BitFunnel as a library in your Rust projects by adding it to your `Cargo.toml`:

```toml
[dependencies]
bitfunnel = { path = "../bitfunnel" }
```

### Basic Example

```rust
use bitfunnel::BitFunnelIndex;

fn main() -> anyhow::Result<()> {
    // Create an index with default settings
    let mut index = BitFunnelIndex::with_defaults();

    // Index a file from the filesystem
    index.index_file("path/to/file.txt")?;

    // Or index content directly
    index.index_document(
        "doc.txt".into(),
        "This is the document content...".to_string(),
    )?;

    // Search for documents
    let results = index.search("rust programming");
    
    // Process results
    for result in results {
        println!("Document: {:?}", result.document.path);
        println!("Score: {:.1}%", result.score);
        println!("Content preview: {}", 
            &result.document.content[..result.document.content.len().min(100)]);
    }

    Ok(())
}
```

### Advanced Configuration

```rust
use bitfunnel::BitFunnelIndex;

// Create index with custom signature size and hash count
let mut index = BitFunnelIndex::new(2048, 5); // 2048 bits, 5 hash functions

// Index multiple files
for file_path in file_paths {
    index.index_file(file_path)?;
}
```

## Web Integrations

BitFunnel provides easy-to-use plugins for popular Rust web frameworks.

### Axum Integration

Add `axum` with `ws` feature to your `Cargo.toml`:

```toml
[dependencies]
axum = { version = "0.7", features = ["ws"] }
bitfunnel = { path = "../bitfunnel" }
```

In your `main.rs`:

```rust
use bitfunnel::api::{create_router, AppState};
use bitfunnel::BitFunnelIndex;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    let index = BitFunnelIndex::with_defaults();
    let state: AppState = Arc::new(RwLock::new(index));

    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

This provides:
- `/` - Search UI
- `POST /api/search` - JSON Search
- `GET /ws/search` - WebSocket Real-time Search

### Actix-web Integration

Add `actix-web` and `actix-ws` to your `Cargo.toml`:

```toml
[dependencies]
actix-web = "4"
actix-ws = "0.2"
bitfunnel = { path = "../bitfunnel" }
```

In your `main.rs`:

```rust
use actix_web::{web, App, HttpServer};
use bitfunnel::integrations::actix::{configure, AppState};
use bitfunnel::BitFunnelIndex;
use std::sync::Arc;
use tokio::sync::RwLock;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let index = BitFunnelIndex::with_defaults();
    let state: AppState = Arc::new(RwLock::new(index));
    let data = web::Data::new(state);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .configure(configure) // Register BitFunnel routes
    })
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}
```

### Examples

The project includes several examples:

```bash
# Basic usage example
cargo run --example basic_usage

# Test substring matching
cargo run --example test_substring

# Test AND + order requirements
cargo run --example test_and_order
```

## How It Works

1. **Indexing**: Each document is processed to extract terms, which are hashed to set bits in a bit vector signature (Bloom filter-like structure).

2. **Query Processing**: When a search query is received, it's converted into a query signature using the same hashing process.

3. **Matching**: Documents are matched by checking if their signatures contain all the bits set in the query signature using bitwise operations.

4. **Relevance Scoring**: Results are scored based on:
   - Bit matching ratio (30%)
   - Exact term matches (70%)

5. **Incremental Search**: As the query changes, only the query signature needs to be recomputed, making incremental search very efficient.

## Configuration

The default configuration uses:
- Signature size: 1024 bits
- Hash functions per term: 3

You can customize these when creating an index:

```rust
let mut index = BitFunnelIndex::new(2048, 5); // Larger signature, more hash functions
```

## Performance Considerations

- **Signature Size**: Larger signatures reduce false positives but use more memory
  - Default: 1024 bits (good balance)
  - Recommended: 2048-4096 bits for large document collections
- **Hash Count**: More hash functions improve accuracy but increase computation
  - Default: 3 hash functions (good balance)
  - Recommended: 3-5 hash functions
- **Document Count**: BitFunnel scales well with large document collections due to efficient bitwise operations
- **N-gram Generation**: Substring matching generates additional terms (3-8 character n-grams), which increases index size but enables flexible searching

## Project Structure

```
bitfunnel/
├── src/
│   ├── lib.rs          # Core BitFunnel algorithm implementation
│   ├── api.rs          # HTTP API server endpoints
│   ├── cli.rs          # Interactive CLI tool
│   ├── main.rs         # API server entry point
│   └── generate_fixtures.rs  # Test data generator
├── examples/
│   ├── basic_usage.rs       # Basic library usage example
│   ├── test_substring.rs    # Substring matching test
│   └── test_and_order.rs   # AND + order requirement test
├── fixtures/
│   └── files/          # Generated test files (500 files, 10k+ words each)
└── Cargo.toml          # Project dependencies and configuration
```

## Troubleshooting

### CLI Issues

- **Terminal not displaying correctly**: Ensure your terminal supports ANSI colors and is at least 80x24 characters
- **Navigation not working**: Make sure you're not in file view mode (press ESC to exit file view)
- **No results found**: Check that files are being indexed (look at the indexing progress)

### API Issues

- **Connection refused**: Ensure the server is running on port 3000
- **CORS errors**: The server has CORS enabled by default, but check your frontend configuration
- **Slow indexing**: Large files or many files will take time to index initially

### Performance Tips

- Use `--extensions` flag to limit file types indexed
- For very large document collections, consider increasing signature size
- The first search after indexing may be slower; subsequent searches are fast

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## License

This project is provided as-is for educational and development purposes.
