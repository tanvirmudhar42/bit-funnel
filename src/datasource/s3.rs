use aws_sdk_s3::Client;
use aws_config::meta::region::RegionProviderChain;
use anyhow::{Result, Context};
use crate::BitFunnelIndex;

pub struct S3Crawler {
    client: Client,
}

impl S3Crawler {
    pub async fn new(region: Option<String>) -> Result<Self> {
        let region_provider = RegionProviderChain::first_try(region.map(aws_sdk_s3::config::Region::new))
            .or_default_provider();
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = Client::new(&config);
        Ok(Self { client })
    }

    pub async fn crawl_bucket(
        &self,
        index: &mut BitFunnelIndex,
        bucket: &str,
        prefix: Option<&str>,
    ) -> Result<usize> {
        let mut count = 0;
        let mut continuation_token = None;
        const MAX_SIZE: u64 = 10 * 1024 * 1024; // 10MB limit

        loop {
            let mut list_objects = self.client.list_objects_v2().bucket(bucket);
            if let Some(prefix) = prefix {
                list_objects = list_objects.prefix(prefix);
            }
            if let Some(token) = continuation_token {
                list_objects = list_objects.continuation_token(token);
            }

            let resp = list_objects.send().await
                .with_context(|| format!("Failed to list objects in bucket: {}", bucket))?;

            for object in resp.contents() {
                if let Some(key) = object.key() {
                    // Check size before downloading
                    if let Some(size) = object.size() {
                        if size as u64 > MAX_SIZE {
                            eprintln!("Skipping large object: {} ({} bytes)", key, size);
                            continue;
                        }
                    }

                    let data = self.client.get_object().bucket(bucket).key(key).send().await?;
                    let bytes = data.body.collect().await?.into_bytes();
                    let content = String::from_utf8_lossy(&bytes).to_string();

                    let source_uri = format!("s3://{}/{}", bucket, key);
                    index.index_document(source_uri, content)?;
                    count += 1;
                }
            }

            if resp.is_truncated().unwrap_or(false) {
                continuation_token = resp.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(count)
    }
}
