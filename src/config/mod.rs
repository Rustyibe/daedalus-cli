use aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use dirs::home_dir;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionInfo {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StoredConnectionInfo {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: Option<String>,
    pub password_cipher: Option<String>,
    pub password_nonce: Option<String>,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    connections: HashMap<String, StoredConnectionInfo>,
}

impl Config {
    pub fn new() -> Result<Self> {
        Ok(Config {
            connections: HashMap::new(),
        })
    }

    pub fn load() -> Result<Self> {
        let config_path = Config::get_config_file_path();

        if !config_path.exists() {
            let config = Config::new()?;
            config.save()?;
            return Ok(config);
        }

        let config_str = fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_str)?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Config::get_config_file_path();

        // Ensure the config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let config_str = serde_json::to_string_pretty(self)?;
        fs::write(config_path, config_str)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn add_connection(&mut self, info: ConnectionInfo) -> Result<()> {
        let (cipher, nonce) = Self::encrypt_password(&info.password)?;
        let stored_info = StoredConnectionInfo {
            host: info.host,
            port: info.port,
            database: info.database,
            username: info.username,
            password: None,
            password_cipher: Some(cipher),
            password_nonce: Some(nonce),
            name: info.name,
        };
        self.connections
            .insert(stored_info.name.clone(), stored_info);
        Ok(())
    }

    pub fn get_connection(&self, name: &str) -> Option<ConnectionInfo> {
        if let Some(stored) = self.connections.get(name).cloned() {
            let password = if let (Some(c), Some(n)) = (
                stored.password_cipher.clone(),
                stored.password_nonce.clone(),
            ) {
                match Self::decrypt_password(&c, &n) {
                    Ok(p) => p,
                    Err(_) => return None,
                }
            } else if let Some(p) = stored.password.clone() {
                p
            } else {
                return None;
            };
            return Some(ConnectionInfo {
                host: stored.host,
                port: stored.port,
                database: stored.database,
                username: stored.username,
                password,
                name: stored.name,
            });
        }
        None
    }

    pub fn list_connections(&self) -> Vec<String> {
        self.connections.keys().cloned().collect()
    }

    #[allow(dead_code)]
    pub fn remove_connection(&mut self, name: &str) -> bool {
        self.connections.remove(name).is_some()
    }

    pub fn decrypt_connection_password(&self, info: &ConnectionInfo) -> Result<String> {
        Ok(info.password.clone())
    }

    fn get_config_file_path() -> std::path::PathBuf {
        let home_dir = Self::get_home_dir();
        let mut config_dir = std::path::PathBuf::from(home_dir);
        config_dir.push(".daedalus-cli");
        config_dir.push("config.json");
        config_dir
    }

    fn get_key_file_path() -> std::path::PathBuf {
        let home_dir = Self::get_home_dir();
        let mut p = std::path::PathBuf::from(home_dir);
        p.push(".daedalus-cli");
        p.push("key.bin");
        p
    }

    fn get_home_dir() -> String {
        // Use the dirs crate for reliable cross-platform home directory detection
        home_dir()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string()) // Fallback to current directory
    }

    fn get_or_create_key() -> Result<[u8; 32]> {
        let path = Self::get_key_file_path();
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut key = [0u8; 32];
            rand::rng().fill(&mut key);
            fs::write(&path, key)?;
            return Ok(key);
        }
        let data = fs::read(path)?;
        let mut key = [0u8; 32];
        key.copy_from_slice(&data[..32]);
        Ok(key)
    }

    fn encrypt_password(plain: &str) -> Result<(String, String)> {
        let key = Self::get_or_create_key()?;
        let cipher = Aes256Gcm::new(&key.into());
        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = cipher
            .encrypt(nonce, plain.as_bytes())
            .map_err(|_| anyhow::anyhow!("encryption failed"))?;
        Ok((STANDARD.encode(ct), STANDARD.encode(nonce_bytes)))
    }

    fn decrypt_password(cipher_b64: &str, nonce_b64: &str) -> Result<String> {
        let key = Self::get_or_create_key()?;
        let cipher = Aes256Gcm::new(&key.into());
        let nonce_bytes = STANDARD.decode(nonce_b64)?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = STANDARD.decode(cipher_b64)?;
        let pt = cipher
            .decrypt(nonce, ct.as_ref())
            .map_err(|_| anyhow::anyhow!("decryption failed"))?;
        Ok(String::from_utf8(pt)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_env() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
        }
        temp_dir
    }

    #[test]
    fn test_new_config() {
        let _temp_dir = setup_test_env(); // Ensure isolated test environment
        let config = Config::new().unwrap();
        assert!(config.connections.is_empty());
    }

    #[test]
    fn test_config_save_and_load() {
        let _temp_dir = setup_test_env();
        let mut config = Config::new().unwrap();

        // Create and save a connection
        let conn_info = ConnectionInfo {
            host: "localhost".to_string(),
            port: 5432,
            database: "test_db".to_string(),
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
            name: "test_conn".to_string(),
        };

        config.add_connection(conn_info.clone()).unwrap();
        config.save().unwrap();

        // Load the config and verify it has the connection
        let loaded_config = Config::load().unwrap();
        assert_eq!(
            loaded_config.list_connections(),
            vec!["test_conn".to_string()]
        );

        let loaded_conn = loaded_config.get_connection("test_conn").unwrap();
        assert_eq!(loaded_conn.host, "localhost");
        assert_eq!(loaded_conn.port, 5432);
        assert_eq!(loaded_conn.database, "test_db");
        assert_eq!(loaded_conn.username, "test_user");
        assert_eq!(loaded_conn.password, "test_pass");
        assert_eq!(loaded_conn.name, "test_conn");
    }

    #[test]
    fn test_add_connection() {
        let _temp_dir = setup_test_env(); // Ensure isolated test environment
        let mut config = Config::new().unwrap();

        let conn_info = ConnectionInfo {
            host: "localhost".to_string(),
            port: 5432,
            database: "test_db".to_string(),
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
            name: "test_conn".to_string(),
        };

        config.add_connection(conn_info).unwrap();
        assert_eq!(config.list_connections(), vec!["test_conn".to_string()]);
    }

    #[test]
    fn test_get_connection() {
        let _temp_dir = setup_test_env();

        let mut config = Config::new().unwrap();

        let conn_info = ConnectionInfo {
            host: "localhost".to_string(),
            port: 5432,
            database: "test_db".to_string(),
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
            name: "test_conn".to_string(),
        };

        config.add_connection(conn_info.clone()).unwrap();

        let retrieved_conn = config.get_connection("test_conn").unwrap();
        assert_eq!(retrieved_conn.host, conn_info.host);
        assert_eq!(retrieved_conn.port, conn_info.port);
        assert_eq!(retrieved_conn.database, conn_info.database);
        assert_eq!(retrieved_conn.username, conn_info.username);
        assert_eq!(retrieved_conn.password, conn_info.password);
        assert_eq!(retrieved_conn.name, conn_info.name);
    }

    #[test]
    fn test_get_nonexistent_connection() {
        let config = Config::new().unwrap();
        assert!(config.get_connection("nonexistent").is_none());
    }

    #[test]
    fn test_list_connections() {
        let mut config = Config::new().unwrap();

        let conn1 = ConnectionInfo {
            host: "localhost".to_string(),
            port: 5432,
            database: "test_db1".to_string(),
            username: "user1".to_string(),
            password: "pass1".to_string(),
            name: "conn1".to_string(),
        };

        let conn2 = ConnectionInfo {
            host: "remote".to_string(),
            port: 5433,
            database: "test_db2".to_string(),
            username: "user2".to_string(),
            password: "pass2".to_string(),
            name: "conn2".to_string(),
        };

        config.add_connection(conn1).unwrap();
        config.add_connection(conn2).unwrap();

        let connections = config.list_connections();
        assert_eq!(connections.len(), 2);
        assert!(connections.contains(&"conn1".to_string()));
        assert!(connections.contains(&"conn2".to_string()));
    }

    #[test]
    fn test_remove_connection() {
        let mut config = Config::new().unwrap();

        let conn_info = ConnectionInfo {
            host: "localhost".to_string(),
            port: 5432,
            database: "test_db".to_string(),
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
            name: "test_conn".to_string(),
        };

        config.add_connection(conn_info).unwrap();
        assert_eq!(config.list_connections(), vec!["test_conn".to_string()]);

        let removed = config.remove_connection("test_conn");
        assert!(removed);
        assert!(config.list_connections().is_empty());

        // Try to remove a non-existent connection
        let removed = config.remove_connection("nonexistent");
        assert!(!removed);
    }

    #[test]
    fn test_password_encryption_decryption() {
        let _temp_dir = setup_test_env();
        let plaintext = "my_secret_password";
        let (cipher, nonce) = Config::encrypt_password(plaintext).unwrap();

        let decrypted = Config::decrypt_password(&cipher, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_config_default_creation() {
        let _temp_dir = setup_test_env();
        let path = Config::get_config_file_path();

        // Config::load should create a default config file if one doesn't exist
        let config = Config::load().unwrap();
        assert!(path.exists());
        assert!(config.connections.is_empty());
    }
}
