use anyhow::{Context, Result};
use bitvec::prelude::*;
#[cfg(all(feature = "rayon", not(feature = "tokio-parallel")))]
use rayon::prelude::*;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use twox_hash::XxHash64;

pub mod api;
pub mod integrations;

/// Represents a document in the index
#[derive(Debug, Clone)]
pub struct Document {
    pub id: usize,
    pub path: PathBuf,
    pub content: String,
    pub words: Vec<String>,
}

/// BitFunnel index that uses bit-sliced signatures for efficient search
pub struct BitFunnelIndex {
    /// Documents indexed by ID
    documents: Vec<Arc<Document>>,
    /// Bit signatures for each document (Bloom filter representation)
    signatures: Vec<BitVec<u64, Lsb0>>,
    /// Signature size in bits
    signature_size: usize,
    /// Number of hash functions (bits per term)
    hash_count: usize,
}

impl BitFunnelIndex {
    /// Create a new BitFunnel index with specified signature size
    pub fn new(signature_size: usize, hash_count: usize) -> Self {
        // Prevent configuration values that would otherwise panic at runtime
        // (e.g. modulo by zero when hashing terms).
        let signature_size = signature_size.max(1);
        let hash_count = hash_count.max(1);

        Self {
            documents: Vec::new(),
            signatures: Vec::new(),
            signature_size,
            hash_count,
        }
    }

    /// Create a new BitFunnel index with default parameters
    pub fn with_defaults() -> Self {
        Self::new(1024, 3) // 1024 bits, 3 hash functions per term
    }

    /// Index a document from a file path
    pub fn index_file(&mut self, path: impl AsRef<Path>) -> Result<usize> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        self.index_document(path.to_path_buf(), content)
    }

    /// Index a document with given path and content
    pub fn index_document(&mut self, path: PathBuf, content: String) -> Result<usize> {
        let doc_id = self.documents.len();

        // Create signature for this document
        let mut signature = bitvec![u64, Lsb0; 0; self.signature_size];

        // Extract terms and set bits in signature
        let (words, terms) = Self::extract_terms_and_words(&content);
        for term in &terms {
            let bit_positions = self.get_term_bit_positions(term);
            for pos in bit_positions {
                if pos < self.signature_size {
                    signature.set(pos, true);
                }
            }
        }

        self.documents.push(Arc::new(Document {
            id: doc_id,
            path,
            content,
            words,
        }));
        self.signatures.push(signature);

        Ok(doc_id)
    }

    /// Search for documents matching the query (supports incremental search)
    /// All words in the query must be present (AND) and appear in order
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        // Extract whole words from query (for order checking)
        let query_words = Self::extract_query_words(query);
        if query_words.is_empty() {
            return Vec::new();
        }

        // Create query signature from all terms (including n-grams for substring matching)
        let query_terms = Self::extract_terms(query);
        let mut query_signature = bitvec![u64, Lsb0; 0; self.signature_size];
        for term in &query_terms {
            let bit_positions = self.get_term_bit_positions(term);
            for pos in bit_positions {
                if pos < self.signature_size {
                    query_signature.set(pos, true);
                }
            }
        }

        let mut results = self.perform_search_parallel(&query_signature, &query_words);

        // Sort by relevance score (higher is better)
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    #[cfg(all(feature = "rayon", not(feature = "tokio-parallel")))]
    fn perform_search_parallel(
        &self,
        query_signature: &BitVec<u64, Lsb0>,
        query_words: &[String],
    ) -> Vec<SearchResult> {
        let query_raw = query_signature.as_raw_slice();
        self.signatures
            .par_iter()
            .enumerate()
            .filter_map(|(doc_id, doc_signature)| {
                self.match_document(
                    doc_id,
                    doc_signature,
                    query_signature,
                    query_raw,
                    query_words,
                )
            })
            .collect()
    }

    #[cfg(feature = "tokio-parallel")]
    fn perform_search_parallel(
        &self,
        query_signature: &BitVec<u64, Lsb0>,
        query_words: &[String],
    ) -> Vec<SearchResult> {
        use std::sync::Mutex;

        if self.signatures.is_empty() {
            return Vec::new();
        }

        let results = Arc::new(Mutex::new(Vec::new()));

        let n_threads = num_cpus::get().max(1);
        let chunk_size = self.signatures.len().div_ceil(n_threads).max(1);

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(n_threads)
            .enable_all()
            .build()
            .expect("Failed to build runtime");

        rt.block_on(async {
            let mut set = tokio::task::JoinSet::new();

            // Extend lifetime of self to 'static for tokio::spawn
            // SAFETY: We block on the JoinSet, so self will not be dropped while tasks are running.
            let self_static: &'static Self = unsafe { std::mem::transmute(self) };

            for (chunk_idx, chunk_sigs) in self.signatures.chunks(chunk_size).enumerate() {
                let results = Arc::clone(&results);
                let query_signature = query_signature.clone();
                let query_words = query_words.to_vec();
                let start_idx = chunk_idx * chunk_size;
                let chunk_sigs_static: &'static [BitVec<u64, Lsb0>] =
                    unsafe { std::mem::transmute(chunk_sigs) };

                set.spawn(async move {
                    let mut local_results = Vec::new();
                    let query_raw = query_signature.as_raw_slice();
                    for (i, doc_signature) in chunk_sigs_static.iter().enumerate() {
                        if let Some(res) = self_static.match_document(
                            start_idx + i,
                            doc_signature,
                            &query_signature,
                            query_raw,
                            &query_words,
                        ) {
                            local_results.push(res);
                        }
                    }
                    let mut r = results.lock().unwrap();
                    r.extend(local_results);
                });
            }

            while set.join_next().await.is_some() {}
        });

        let mut final_results = results.lock().unwrap();
        std::mem::take(&mut *final_results)
    }

    #[cfg(not(any(feature = "rayon", feature = "tokio-parallel")))]
    fn perform_search_parallel(
        &self,
        query_signature: &BitVec<u64, Lsb0>,
        query_words: &[String],
    ) -> Vec<SearchResult> {
        let query_raw = query_signature.as_raw_slice();
        self.signatures
            .iter()
            .enumerate()
            .filter_map(|(doc_id, doc_signature)| {
                self.match_document(
                    doc_id,
                    doc_signature,
                    query_signature,
                    query_raw,
                    query_words,
                )
            })
            .collect()
    }

    fn match_document(
        &self,
        doc_id: usize,
        doc_signature: &BitVec<u64, Lsb0>,
        query_signature: &BitVec<u64, Lsb0>,
        query_raw: &[u64],
        query_words: &[String],
    ) -> Option<SearchResult> {
        // First check: document signature must contain all query bits (fast filter)
        let doc_raw = doc_signature.as_raw_slice();
        for (q, d) in query_raw.iter().zip(doc_raw.iter()) {
            if (q & d) != *q {
                return None;
            }
        }

        let doc = &self.documents[doc_id];
        // Second check: all query words must be present AND in order
        if self.matches_query_words_in_order(doc, query_words) {
            // Calculate relevance score
            let score = self.calculate_relevance(query_signature, doc_signature, query_words, doc);
            return Some(SearchResult {
                document_id: doc_id,
                score,
                document: Arc::clone(doc),
            });
        }
        None
    }

    /// Extract whole words from query (for order checking)
    fn extract_query_words(query: &str) -> Vec<String> {
        query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|s| !s.is_empty() && s.len() > 1)
            .map(|s| s.to_string())
            .collect()
    }

    /// Check if all query words appear in the document in order
    fn matches_query_words_in_order(&self, doc: &Document, query_words: &[String]) -> bool {
        if query_words.is_empty() {
            return true;
        }

        let doc_words = &doc.words;

        // Find positions where each query word matches (as substring), ensuring order
        let mut last_pos = 0;
        for query_word in query_words {
            // Find the first occurrence of this query word after the last matched position
            let mut found = false;
            for (pos, doc_word) in doc_words.iter().enumerate().skip(last_pos) {
                // Check if query word is a substring of doc word (for substring matching)
                // or if they match exactly
                if doc_word.contains(query_word) {
                    last_pos = pos + 1; // Next search starts after this position
                    found = true;
                    break;
                }
            }
            if !found {
                return false; // Query word not found in order
            }
        }

        true
    }

    /// Calculate relevance score for a document
    fn calculate_relevance(
        &self,
        query_sig: &BitVec<u64, Lsb0>,
        doc_sig: &BitVec<u64, Lsb0>,
        query_words: &[String],
        doc: &Document,
    ) -> f64 {
        // Count matching bits
        let mut matching_bits = 0;
        let mut query_bits = 0;

        let query_raw = query_sig.as_raw_slice();
        let doc_raw = doc_sig.as_raw_slice();

        for (q, d) in query_raw.iter().zip(doc_raw.iter()) {
            query_bits += q.count_ones();
            matching_bits += (q & d).count_ones();
        }

        // Also count exact term matches for better relevance
        let mut exact_matches = 0;
        for word in query_words {
            // Check if any word in the document contains the query word
            if doc.words.iter().any(|dw| dw.contains(word)) {
                exact_matches += 1;
            }
        }

        // Combine bit matching and exact term matching
        let bit_score = if query_bits > 0 {
            matching_bits as f64 / query_bits as f64
        } else {
            0.0
        };
        let term_score = if !query_words.is_empty() {
            exact_matches as f64 / query_words.len() as f64
        } else {
            0.0
        };

        (bit_score * 0.3 + term_score * 0.7) * 100.0
    }

    /// Get bit positions for a term using multiple hash functions
    fn get_term_bit_positions(&self, term: &str) -> Vec<usize> {
        if self.signature_size == 0 {
            return Vec::new();
        }

        let mut positions = Vec::new();

        // Generate multiple hash values for this term using different seeds
        for i in 0..self.hash_count {
            let mut hasher = XxHash64::with_seed(i as u64);
            hasher.write(term.as_bytes());
            let hash = hasher.finish();
            let pos = (hash as usize) % self.signature_size;
            positions.push(pos);
        }

        positions
    }

    /// Extract terms and words from text
    fn extract_terms_and_words(text: &str) -> (Vec<String>, Vec<String>) {
        let text_lower = text.to_lowercase();

        // Extract whole words
        let words: Vec<String> = text_lower
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|s| !s.is_empty() && s.len() > 1)
            .map(|s| s.to_string())
            .collect();

        let mut terms = Vec::with_capacity(words.len() * 5);

        // Add whole words
        terms.extend(words.clone());

        // Generate n-grams (substrings) from words for substring matching
        const MIN_NGRAM_LEN: usize = 3;
        const MAX_NGRAM_LEN: usize = 8;

        for word in &words {
            let chars: Vec<char> = word.chars().collect();
            let len = chars.len();
            if len >= MIN_NGRAM_LEN {
                let max_ngram = len.min(MAX_NGRAM_LEN);
                for ngram_len in MIN_NGRAM_LEN..=max_ngram {
                    for start in 0..=(len - ngram_len) {
                        let ngram: String = chars[start..start + ngram_len].iter().collect();
                        terms.push(ngram);
                    }
                }
            }
        }

        (words, terms)
    }

    /// Extract terms from text (simple tokenization)
    /// Now includes both whole words and n-grams for substring matching
    fn extract_terms(text: &str) -> Vec<String> {
        Self::extract_terms_and_words(text).1
    }

    /// Get number of indexed documents
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// Get a document by ID
    pub fn get_document(&self, id: usize) -> Option<&Document> {
        self.documents.get(id).map(|d| d.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::BitFunnelIndex;
    use std::path::PathBuf;

    #[test]
    fn search_with_zero_signature_size_does_not_panic() {
        let mut index = BitFunnelIndex::new(0, 3);
        index
            .index_document(PathBuf::from("doc.txt"), "hello world".to_string())
            .expect("indexing should succeed");
        let results = index.search("hello");
        assert_eq!(results.len(), 1);
    }

    #[cfg(feature = "tokio-parallel")]
    #[test]
    fn tokio_parallel_search_on_empty_index_returns_empty_results() {
        let index = BitFunnelIndex::with_defaults();
        let results = index.search("query");
        assert!(results.is_empty());
    }
}

/// Search result containing document and relevance score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub document_id: usize,
    pub score: f64,
    pub document: Arc<Document>,
}
