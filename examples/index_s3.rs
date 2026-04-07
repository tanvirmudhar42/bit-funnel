#[cfg(feature = "s3")]
use bitfunnel::BitFunnelIndex;
#[cfg(feature = "s3")]
use bitfunnel::datasource::s3::S3Crawler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "s3")]
    {
        // This example requires valid AWS credentials and access to an S3 bucket.
        let bucket_name = "my-text-data-bucket";
        let region = Some("us-east-1".to_string());

        let crawler = match S3Crawler::new(region).await {
            Ok(c) => c,
            Err(e) => {
                println!("S3 crawler setup failed (check your AWS credentials): {}", e);
                return Ok(());
            }
        };

        let mut index = BitFunnelIndex::with_defaults();

        // Index all .txt objects in the bucket
        let count = crawler.crawl_bucket(
            &mut index,
            bucket_name,
            None // or Some("prefix/") to limit the search
        ).await?;

        println!("Indexed {} objects from S3 bucket: {}", count, bucket_name);

        // Search the indexed S3 content
        let results = index.search("cloud storage");
        for res in results {
            println!("Found in S3: {} (Score: {:.1}%)", res.document.path, res.score);
        }
    }
    #[cfg(not(feature = "s3"))]
    {
        println!("This example requires the 's3' feature.");
    }

    Ok(())
}
