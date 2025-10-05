# Daedalus CLI

[![Crates.io Version](https://img.shields.io/crates/v/daedalus-cli)](https://crates.io/crates/daedalus-cli)
[![Documentation](https://docs.rs/daedalus-cli/badge.svg)](https://docs.rs/daedalus-cli)
[![License](https://img.shields.io/crates/l/daedalus-cli)](https://github.com/Zephyruston/daedalus-cli/blob/main/LICENSE)

[中文文档](README_zh.md) | [Chinese Documentation](README_zh.md)

Craft Your Path in the Data Labyrinth.

Daedalus CLI is a Rust-based command-line interface tool for PostgreSQL database management and exploration. It provides an intuitive terminal user interface (TUI) that allows users to connect to PostgreSQL databases, browse tables, and view data with pagination support.

## Features

- **Database Connection Management**: Add, list, and remove saved database connections with encrypted storage of connection information in `~/.daedalus-cli/config.json`
- **Terminal User Interface**: Intuitive TUI for browsing database tables, using arrow keys to navigate between items in the current view
- **Data Exploration**: Browse database tables with column headers, view table data with row highlighting
- **Pagination Support**: Navigate large datasets using PageUp/PageDown keys
- **Secure Storage**: AES-256-GCM encryption for storing sensitive connection password information

## Installation

Install Daedalus CLI from crates.io:

```bash
cargo install daedalus-cli
```

Or build from source:

```bash
git clone https://github.com/Zephyruston/daedalus-cli.git
cd daedalus-cli
cargo install --path .
```

## Usage

### Adding a Database Connection

Add a new database connection with a custom name:

```bash
daedalus-cli add-conn postgresql://username:password@host:port/database --name mydb
```

Or let the system generate a name based on host and database:

```bash
daedalus-cli add-conn postgresql://username:password@host:port/database
```

### Listing Saved Connections

List all saved database connections:

```bash
daedalus-cli list-conns
```

### Removing a Connection

Remove a saved connection:

```bash
daedalus-cli remove-conn mydb
```

### Connecting to a Database

Connect to a saved database using the TUI:

```bash
daedalus-cli connect mydb
```

### Testing a Connection

Test a connection without opening the TUI:

```bash
daedalus-cli ping mydb
```

### Generating Shell Completions

Generate command-line completion scripts for bash, zsh, and fish:

```bash
# Generate completion script for bash
daedalus-cli completions bash

# Generate completion script for zsh
daedalus-cli completions zsh

# Generate completion script for fish
daedalus-cli completions fish
```

To make completions effective in the current session, you need to execute one of the following operations based on your shell type:

**For bash:**

```bash
# Generate and save the completion script
daedalus-cli completions bash > ~/.bash_completion.d/daedalus-cli
# Source the file in your .bashrc
echo "source ~/.bash_completion.d/daedalus-cli" >> ~/.bashrc
# Reload configuration
source ~/.bashrc
```

**For zsh:**

```bash
# Generate completion script to standard location
daedalus-cli completions zsh > /usr/local/share/zsh/site-functions/_daedalus-cli
# Or for user-specific installation
mkdir -p ~/.zsh/completion
daedalus-cli completions zsh > ~/.zsh/completion/_daedalus-cli
echo "fpath+=~/.zsh/completion" >> ~/.zshrc
# Reload configuration
source ~/.zshrc
```

**For fish:**

```bash
# Generate completion script to fish's completions directory
mkdir -p ~/.config/fish/completions
daedalus-cli completions fish > ~/.config/fish/completions/daedalus-cli.fish
# Restart fish or reload configuration
```

## TUI Navigation

After connecting to a database, the TUI provides the following navigation controls:

- **Arrow keys (↑/↓)**: Navigate between items in the current view
- **Enter**: Select highlighted item
- **PageUp/PageDown**: Navigate in large datasets
- **'t'**: Return to table list
- **'c'**: Return to connection selection
- **'q' or Esc**: Exit the application

## Security

Daedalus CLI implements security measures to protect your database credentials:

- Connection passwords are encrypted using AES-256-GCM before being stored to the config file
- Randomly generated encryption key is stored in `~/.daedalus-cli/key.bin`
- All connections are established using the secure tokio-postgres library

## Development

### Prerequisites

- Rust edition 2024
- PostgreSQL server connection (or use Docker container provided)

### Testing

```bash
# Run tests
cargo test

# Check for compilation errors
cargo check

# Run linter
cargo clippy
```

### Docker Support

The project includes a `docker-compose.yml` file for setting up a PostgreSQL container with sample data for testing:

```bash
# Start PostgreSQL container with sample data
docker-compose up -d

# Stop containers
docker-compose down
```

#### Using the Docker Test Database

1. **Start the test database**:

   ```bash
   docker-compose up -d
   ```

2. **Wait for database initialization**:

   - After the container starts, the database needs some time to initialize and be running
   - You can use `docker-compose logs -f` to view startup progress
   - By default, the database will run on port 5432

3. **Connect to the test database**:

   ```bash
   daedalus-cli add-conn postgresql://test:123456@localhost:5432/test_db --name test_db
   ```

4. **Test the connection**:

   ```bash
   daedalus-cli ping test_db
   ```

5. **Browse data**:
   ```bash
   daedalus-cli connect test_db
   ```

#### Test Database Structure

The Docker test database includes the following sample tables for demonstration purposes:

- **users**: Stores user information (id, username, email, created_at)
- **projects**: Stores project information (id, name, description, owner_id)
- **tasks**: Stores task information (id, title, description, project_id, assigned_to, status, priority)
- **api_keys**: Stores API keys (id, key_value, user_id, name, permissions)

These tables contain sample data that allows you to test Daedalus CLI functionality in a real database environment.

#### Stopping the Test Database

After you are done, you can stop the database container:

```bash
docker-compose down
```

This will stop all running containers but preserve the data volumes. If you need to remove the data volumes (clearing all data), use:

```bash
docker-compose down -v
```

## Configuration

Connection information is stored in `~/.daedalus-cli/config.json`. This includes:

- Host, port, database name, username
- Encrypted password data
- Connection name for identification

The encryption key is stored in `~/.daedalus-cli/key.bin` and should be kept secure.

## License

MIT License - see the [LICENSE](LICENSE) file for details.
