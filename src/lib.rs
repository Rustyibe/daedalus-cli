//! # Daedalus CLI
//!
//! Daedalus CLI is a Rust-based command-line interface tool for PostgreSQL database management and exploration.
//! It provides an intuitive terminal user interface (TUI) that allows users to connect to PostgreSQL databases,
//! browse tables, and view data with pagination support.
//!
//! ## Features
//!
//! - **Database Connection Management**: Add, list, and remove saved database connections with encrypted
//!   storage of connection information in `~/.daedalus-cli/config.json`
//! - **Terminal User Interface**: Intuitive TUI for browsing database tables, using arrow keys to navigate
//!   between items in the current view
//! - **Data Exploration**: Browse database tables with column headers, view table data with row highlighting
//! - **Pagination Support**: Navigate large datasets using PageUp/PageDown keys
//! - **Secure Storage**: AES-256-GCM encryption for storing sensitive connection password information
//!
//! ## Usage
//!
//! This library provides modules for configuration management, database operations, and terminal UI.
//! For command-line usage, see the main binary in `src/main.rs`.
//!
//! ## Modules
//!
//! - `config`: Handles connection storage and retrieval
//! - `db`: PostgreSQL connection and query functions
//! - `tui`: TUI rendering and interaction logic

pub mod config;
pub mod db;
pub mod tui;

pub use config::Config;
pub use db::DatabaseConnection;
