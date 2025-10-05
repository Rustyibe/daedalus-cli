use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use daedalus_cli::config::ConnectionInfo;
use daedalus_cli::db::DatabaseConnection;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;

mod config;
mod db;
mod tui;

use crate::tui::{App, run_app};

#[derive(Parser)]
#[command(name = "daedalus-cli")]
#[command(about = "A CLI tool for PostgreSQL database management", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new database connection
    #[command(alias = "add")]
    AddConn {
        /// Connection string in the format: postgresql://username:password@host:port/database
        connection_string: String,
        /// Name for the connection (optional, will generate if not provided)
        #[arg(short, long)]
        name: Option<String>,
    },
    /// List all saved connections
    #[command(alias = "ls")]
    ListConns,
    /// Remove a saved connection
    #[command(alias = "rm")]
    RemoveConn {
        /// Name of the connection to remove
        name: String,
    },
    /// Connect to a database with a saved connection
    Connect {
        /// Name of the saved connection to use
        name: String,
    },
    /// Ping a saved connection without TUI
    Ping {
        /// Name of the saved connection to use
        name: String,
    },
    /// Generate shell completions
    #[command(alias = "gen-completions")]
    Completions {
        /// Shell type for completions
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::AddConn {
            connection_string,
            name,
        } => {
            add_connection(connection_string, name).await?;
        }
        Commands::ListConns => {
            list_connections().await?;
        }
        Commands::RemoveConn { name } => {
            remove_connection(name).await?;
        }
        Commands::Connect { name } => {
            run_tui(name).await?;
        }
        Commands::Ping { name } => {
            ping_connection(name).await?;
        }
        Commands::Completions { shell } => {
            generate_completions(*shell);
        }
    }

    Ok(())
}

async fn add_connection(connection_string: &str, name: &Option<String>) -> Result<()> {
    // Parse the connection string
    let parsed = parse_connection_string(connection_string)?;

    // Use provided name or generate a default name
    let connection_name = name.clone().unwrap_or_else(|| {
        // Generate a name based on host and database
        format!("{}@{}", parsed.username, parsed.database)
    });

    // Create connection info
    let conn_info = ConnectionInfo {
        host: parsed.host,
        port: parsed.port,
        database: parsed.database,
        username: parsed.username,
        password: parsed.password,
        name: connection_name.clone(),
    };

    // Load config, add connection, and save
    let mut config = daedalus_cli::config::Config::load()?;
    config.add_connection(conn_info)?;
    config.save()?;

    println!("Connection '{}' added successfully!", connection_name);
    Ok(())
}

async fn list_connections() -> Result<()> {
    let config = daedalus_cli::config::Config::load()?;
    let connections = config.list_connections();

    if connections.is_empty() {
        println!("No saved connections found.");
    } else {
        println!("Saved connections:");
        for conn in connections {
            println!("- {}", conn);
        }
    }

    Ok(())
}

async fn remove_connection(name: &str) -> Result<()> {
    let mut config = daedalus_cli::config::Config::load()?;

    if config.remove_connection(name) {
        config.save()?;
        println!("Connection '{}' removed successfully!", name);
    } else {
        eprintln!("Connection '{}' not found.", name);
        std::process::exit(1);
    }

    Ok(())
}

async fn run_tui(connection_name: &str) -> Result<()> {
    // Check if connection exists
    let config = daedalus_cli::config::Config::load()?;
    if config.get_connection(connection_name).is_none() {
        eprintln!("Connection '{}' not found.", connection_name);
        std::process::exit(1);
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create the app with the specified connection and run it
    let mut app = App::new_with_connection(connection_name.to_string())?;
    app.init();
    let res = run_app(&mut terminal, app, connection_name.to_string()).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:#?}", err);
    }

    Ok(())
}

// Helper function to connect to database with parameters
#[allow(dead_code)]
async fn connect_to_database(
    host: &str,
    port: u16,
    database: &str,
    username: &str,
    password: &str,
) -> Result<DatabaseConnection> {
    let connection = DatabaseConnection::connect(host, port, database, username, password).await?;
    Ok(connection)
}

// Example of how to connect using saved connection
#[allow(dead_code)]
async fn connect_with_saved_info(name: &str) -> Result<DatabaseConnection> {
    let config = crate::config::Config::load()?;
    if let Some(conn_info) = config.get_connection(name) {
        let password = config.decrypt_connection_password(&conn_info)?;
        connect_to_database(
            &conn_info.host,
            conn_info.port,
            &conn_info.database,
            &conn_info.username,
            &password,
        )
        .await
    } else {
        Err(anyhow!("Connection not found"))
    }
}

async fn ping_connection(name: &str) -> Result<()> {
    let conn = connect_with_saved_info(name).await?;
    let tables = conn.list_tables().await?;
    println!("Ping successful. {} tables found.", tables.len());
    Ok(())
}

fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());
}

// Parse a connection string into its components
use anyhow::anyhow;

fn parse_connection_string(connection_string: &str) -> Result<ParsedConnectionString> {
    // Basic parsing for postgresql://username:password@host:port/database
    if !connection_string.starts_with("postgresql://") {
        return Err(anyhow!(
            "Invalid connection string format. Must start with 'postgresql://'"
        ));
    }

    let without_prefix = &connection_string[13..]; // Remove "postgresql://"

    // Split at @ to separate credentials from host
    let parts: Vec<&str> = without_prefix.split('@').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid connection string format. Expected 'postgresql://user:pass@host:port/db'"
        ));
    }

    let (credentials, host_part) = (parts[0], parts[1]);

    // Extract username and password from credentials
    let cred_parts: Vec<&str> = credentials.split(':').collect();
    if cred_parts.len() != 2 {
        return Err(anyhow!(
            "Invalid credentials format. Expected 'username:password'"
        ));
    }

    let username = cred_parts[0];
    let password = cred_parts[1];

    // Split host_part to extract host:port and database
    let host_db_parts: Vec<&str> = host_part.split('/').collect();
    if host_db_parts.len() != 2 {
        return Err(anyhow!(
            "Invalid connection string format. Expected host:port/database"
        ));
    }

    let (host_port, database) = (host_db_parts[0], host_db_parts[1]);

    // Extract host and port
    let host_port_parts: Vec<&str> = host_port.split(':').collect();
    if host_port_parts.len() != 2 {
        return Err(anyhow!("Invalid host:port format. Expected 'host:port'"));
    }

    let host = host_port_parts[0].to_string();
    let port: u16 = host_port_parts[1]
        .parse()
        .map_err(|_| anyhow!("Invalid port number"))?;

    Ok(ParsedConnectionString {
        username: username.to_string(),
        password: password.to_string(),
        host: host.to_string(),
        port,
        database: database.to_string(),
    })
}

#[derive(Debug)]
struct ParsedConnectionString {
    username: String,
    password: String,
    host: String,
    port: u16,
    database: String,
}
