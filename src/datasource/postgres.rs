use sqlx::{Postgres, Pool, Row};
use anyhow::{Result, Context, anyhow};
use futures::StreamExt;
use crate::BitFunnelIndex;

pub struct PostgresCrawler {
    pool: Pool<Postgres>,
}

impl PostgresCrawler {
    pub async fn new(connection_string: &str) -> Result<Self> {
        let pool = Pool::<Postgres>::connect(connection_string).await
            .with_context(|| format!("Failed to connect to Postgres: {}", connection_string))?;
        Ok(Self { pool })
    }

    fn validate_identifier(id: &str) -> Result<()> {
        if id.chars().all(|c| c.is_alphanumeric() || c == '_') && !id.is_empty() {
            Ok(())
        } else {
            Err(anyhow!("Invalid identifier: {}", id))
        }
    }

    pub async fn crawl_table(
        &self,
        index: &mut BitFunnelIndex,
        table: &str,
        id_column: &str,
        text_columns: &[&str],
    ) -> Result<usize> {
        Self::validate_identifier(table)?;
        Self::validate_identifier(id_column)?;
        for col in text_columns {
            Self::validate_identifier(col)?;
        }

        let columns = text_columns.join(", ");
        let query = format!("SELECT {}, {} FROM {}", id_column, columns, table);

        let mut rows = sqlx::query(&query).fetch(&self.pool);

        let mut count = 0;
        while let Some(row_res) = rows.next().await {
            let row = row_res.with_context(|| format!("Error fetching row from table: {}", table))?;

            let id: String = match row.try_get::<i32, _>(0) {
                Ok(val) => val.to_string(),
                Err(_) => row.try_get::<String, _>(0).unwrap_or_else(|_| "unknown".to_string()),
            };

            let mut content = String::new();
            for i in 1..=text_columns.len() {
                let val: String = row.try_get::<String, _>(i).unwrap_or_default();
                if !content.is_empty() {
                    content.push(' ');
                }
                content.push_str(&val);
            }

            let source_uri = format!("postgres://{}/{}#{}", table, id, text_columns.join(","));
            index.index_document(source_uri, content)?;
            count += 1;
        }

        Ok(count)
    }
}
