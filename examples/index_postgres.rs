#[cfg(feature = "postgres")]
use bitfunnel::datasource::postgres::PostgresCrawler;
#[cfg(feature = "postgres")]
use bitfunnel::BitFunnelIndex;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "postgres")]
    {
        // This example requires a running Postgres instance.
        // If you don't have one, this will fail.
        let conn_str = "postgres://postgres:password@localhost/mydb";

        let crawler = match PostgresCrawler::new(conn_str).await {
            Ok(c) => c,
            Err(e) => {
                println!(
                    "Postgres connection failed (expected if no DB is running): {}",
                    e
                );
                return Ok(());
            }
        };

        let mut index = BitFunnelIndex::with_defaults();

        // Index rows from a hypothetical 'articles' table
        let count = crawler
            .crawl_table(&mut index, "articles", "id", &["title", "content"])
            .await?;

        println!("Indexed {} rows from Postgres", count);

        // Search the indexed DB content
        let results = index.search("database");
        for res in results {
            println!(
                "Found in DB: {} (Score: {:.1}%)",
                res.document.path, res.score
            );
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        println!("This example requires the 'postgres' feature.");
    }

    Ok(())
}
