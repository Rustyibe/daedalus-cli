# Changelog

## 0.1.3

- Fixed field-level navigation and detailed field view in custom SQL queries
- Added support for navigating between fields in custom query results using left/right arrow keys
- Added support for viewing detailed field values in custom query results with Enter key
- Added scroll support for long field values in custom queries using up/down arrow keys
- Fixed state management to properly return to the correct view (table or custom query) when exiting field detail view
- Added Clone trait to AppState enum to support proper state tracking

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
