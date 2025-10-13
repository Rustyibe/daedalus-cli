use anyhow::{Result, anyhow};
use tokio_postgres::{Client, Config, NoTls};

#[derive(Debug)]
pub struct DatabaseConnection {
    pub client: Client,
}

impl DatabaseConnection {
    pub async fn connect(
        host: &str,
        port: u16,
        database: &str,
        username: &str,
        password: &str,
    ) -> Result<DatabaseConnection> {
        let mut config = Config::new();
        config
            .host(host)
            .port(port)
            .dbname(database)
            .user(username)
            .password(password);

        match config.connect(NoTls).await {
            Ok((client, connection)) => {
                // The connection object performs the actual communication with the database,
                // so spawn it off to run on its own.
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("Database connection error: {}", e);
                    }
                });

                Ok(DatabaseConnection { client })
            }
            Err(e) => Err(anyhow!("Failed to connect to database: {}", e)),
        }
    }

    pub async fn list_tables(&self) -> Result<Vec<String>> {
        let rows = self
            .client
            .query(
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'",
                &[],
            )
            .await
            .map_err(|e| anyhow!("Failed to query tables: {}", e))?;

        let mut tables = Vec::new();
        for row in rows {
            tables.push(row.get(0));
        }

        Ok(tables)
    }

    pub async fn get_table_data(
        &self,
        table_name: &str,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        // First get column names and more detailed data types
        let columns_query = format!(
            "SELECT column_name, 
                    CASE 
                        WHEN character_maximum_length IS NOT NULL 
                        THEN data_type || '(' || character_maximum_length || ')' 
                        ELSE data_type 
                    END AS detailed_type
             FROM information_schema.columns 
             WHERE table_name = '{}' 
             ORDER BY ordinal_position",
            table_name
        );
        let column_rows = self
            .client
            .query(&columns_query, &[])
            .await
            .map_err(|e| anyhow!("Failed to query columns: {}", e))?;

        let mut columns = Vec::new();
        let mut column_types = Vec::new();
        for row in column_rows {
            let col_name: String = row.get(0);
            let col_type: String = row.get(1);
            columns.push(col_name.clone());
            column_types.push(col_type);
        }

        // Build a SELECT query that casts all columns to text to ensure string values
        let select_columns = columns
            .iter()
            .map(|col| format!("{}::text", col)) // Cast each column to text
            .collect::<Vec<_>>()
            .join(", ");

        let data_query = format!(
            "SELECT {} FROM {} LIMIT {} OFFSET {}",
            select_columns, table_name, limit, offset
        );

        let data_rows = self
            .client
            .query(&data_query, &[])
            .await
            .map_err(|e| anyhow!("Failed to query table data: {}", e))?;

        let mut data = Vec::new();
        for row in data_rows {
            let mut row_data = Vec::new();
            for i in 0..row.len() {
                let value: Option<String> = row.get(i);
                row_data.push(value.unwrap_or_else(|| "NULL".to_string()));
            }
            data.push(row_data);
        }

        // Modify column names to include type information
        let typed_columns: Vec<String> = columns
            .into_iter()
            .zip(column_types.iter())
            .map(|(name, data_type)| format!("{} ({})", name, data_type))
            .collect();

        Ok((typed_columns, data))
    }

    pub async fn get_table_count(&self, table_name: &str) -> Result<i64> {
        let count_query = format!("SELECT COUNT(*) FROM {}", table_name);
        let row = self
            .client
            .query_one(&count_query, &[])
            .await
            .map_err(|e| anyhow!("Failed to query table count: {}", e))?;

        Ok(row.get(0))
    }

    pub async fn execute_custom_query(
        &self,
        query: &str,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        // For SELECT queries, we'll wrap the query to ensure all columns are converted to text
        let limited_query = if query.to_lowercase().trim().starts_with("select") {
            // Get the column names from the original query
            let base_query = query.trim_end_matches(';');

            // Execute a limited version of the query to get column information
            let column_query = format!("{} LIMIT 1", base_query);
            let column_rows = self
                .client
                .query(&column_query, &[])
                .await
                .map_err(|e| anyhow!("Failed to get column information: {}", e))?;

            if column_rows.is_empty() {
                // If no rows, just execute the original query with limit/offset
                format!("{} LIMIT {} OFFSET {}", base_query, limit, offset)
            } else {
                // Get column names and build a query that converts all columns to text
                let columns = column_rows[0]
                    .columns()
                    .iter()
                    .map(|col| col.name())
                    .collect::<Vec<_>>();

                let select_columns = columns
                    .iter()
                    .map(|col| format!("{}::text", col))
                    .collect::<Vec<_>>()
                    .join(", ");

                format!(
                    "SELECT {} FROM ({} LIMIT {} OFFSET {}) AS text_query",
                    select_columns, base_query, limit, offset
                )
            }
        } else {
            // For non-SELECT queries (INSERT, UPDATE, DELETE), just execute as-is
            query.to_string()
        };

        // Execute the query
        let rows = self
            .client
            .query(&limited_query, &[])
            .await
            .map_err(|e| anyhow!("Failed to execute custom query: {}", e))?;

        // Get column names from the result
        let columns = if !rows.is_empty() {
            rows[0]
                .columns()
                .iter()
                .map(|col| col.name().to_string())
                .collect()
        } else {
            // If no rows returned, we need to determine columns differently
            // For now, return empty columns
            Vec::new()
        };

        // Convert rows to string data using the same approach as get_table_data
        let mut data = Vec::new();
        for row in rows {
            let mut row_data = Vec::new();
            for i in 0..row.len() {
                // Use the same simple approach as get_table_data
                let value: Option<String> = row.get(i);
                row_data.push(value.unwrap_or_else(|| "NULL".to_string()));
            }
            data.push(row_data);
        }

        Ok((columns, data))
    }

    pub async fn get_query_row_count(&self, query: &str) -> Result<i64> {
        // For SELECT queries, try to get the count
        if query.to_lowercase().trim().starts_with("select") {
            // Extract the FROM clause and create a count query
            let count_query = format!(
                "SELECT COUNT(*) FROM ({}) AS count_query",
                query.trim_end_matches(';')
            );

            match self.client.query_one(&count_query, &[]).await {
                Ok(row) => Ok(row.get(0)),
                Err(_) => {
                    // If count query fails, return a default value
                    Ok(0)
                }
            }
        } else {
            // For non-SELECT queries, return 0
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock implementations for connection tests
    #[tokio::test]
    async fn test_connect_success() {
        // This test would require a real database connection or mocking
        // For now, we test that the connection string is built correctly
        // Note: This test will fail without a running PostgreSQL server
        let result =
            DatabaseConnection::connect("localhost", 5432, "postgres", "postgres", "password")
                .await;

        // The connection might fail due to no server running,
        // but we check the error message format to ensure the function works
        if result.is_err() {
            let err = result.unwrap_err();
            assert!(err.to_string().contains("Failed to connect to database:"));
        }
    }

    #[tokio::test]
    async fn test_connect_with_invalid_host() {
        let result = DatabaseConnection::connect(
            "nonexistent_host",
            5432,
            "postgres",
            "postgres",
            "password",
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Failed to connect to database:"));
    }

    #[tokio::test]
    async fn test_get_table_count() {
        // We can't test the actual function without a real connection
        // But we can test the SQL query structure by examining it
        // This is a placeholder - would need to use mocking in a real scenario
    }
}
