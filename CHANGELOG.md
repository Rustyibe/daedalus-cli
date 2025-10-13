# Changelog

## 0.1.2

- Added custom SQL query execution feature

## 0.1.1

- Added cross-platform home directory support using dirs crate
- Fixed issue where config directory was created in current working directory on Windows
- Replaced tempdir with tempfile dependency for better cross-platform compatibility
- Added cargo audit pre-commit hook

## 0.1.0

- Initial release of Daedalus CLI
- Connection management with encrypted storage
- Terminal user interface for browsing PostgreSQL databases
- Support for adding, listing, and removing saved database connections
- TUI with arrow key navigation, pagination support, and intuitive controls
