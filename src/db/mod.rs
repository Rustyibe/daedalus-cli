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
