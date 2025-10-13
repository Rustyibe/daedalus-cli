use crate::db::DatabaseConnection;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Row, Table, TableState},
};
use std::io;

#[derive(Debug, PartialEq, Clone)]
pub enum AppState {
    ConnectionSelection,
    TableList,
    TableData,
    FieldDetail, // New state for detailed field view
    CustomQuery,
    CustomQueryInput,
    Connecting,
    ConnectionError,
}

pub struct App {
    pub state: AppState,
    pub config: crate::config::Config,
    pub connection: Option<DatabaseConnection>,
    pub connections_list_state: ListState,
    pub tables_list_state: ListState,
    pub table_data_state: TableState,
    pub field_selection_state: Option<usize>, // Track selected field in the current row (None means row-focused mode)
    pub tables: Vec<String>,
    pub current_table: Option<String>,
    pub table_columns: Vec<String>,
    pub table_data: Vec<Vec<String>>,
    pub current_page: u32,
    pub max_page: u32,
    pub items_per_page: u32,
    pub error_message: Option<String>,
    pub connection_status: Option<String>,
    // Custom query fields
    pub custom_query_input: String,
    pub custom_query_cursor_position: usize,
    pub custom_query_result_columns: Vec<String>,
    pub custom_query_result_data: Vec<Vec<String>>,
    pub custom_query_current_page: u32,
    pub custom_query_max_page: u32,
    // Field detail view
    pub selected_field_value: Option<String>, // Store the value for detailed view
    pub field_detail_scroll: u16,             // Track scroll position for long field values
    pub field_detail_origin_state: Option<AppState>, // Track the original state when entering field detail view
}

impl App {
    #[allow(dead_code)]
    pub fn new() -> Result<App> {
        let config = crate::config::Config::load()?;

        Ok(App {
            state: AppState::ConnectionSelection,
            config,
            connection: None,
            connections_list_state: ListState::default(),
            tables_list_state: ListState::default(),
            table_data_state: TableState::default(),
            field_selection_state: None,
            tables: Vec::new(),
            current_table: None,
            table_columns: Vec::new(),
            table_data: Vec::new(),
            current_page: 0,
            max_page: 0,
            items_per_page: 20,
            error_message: None,
            connection_status: None,
            // Custom query fields
            custom_query_input: String::new(),
            custom_query_cursor_position: 0,
            custom_query_result_columns: Vec::new(),
            custom_query_result_data: Vec::new(),
            custom_query_current_page: 0,
            custom_query_max_page: 0,
            selected_field_value: None,
            field_detail_scroll: 0,
            field_detail_origin_state: None,
        })
    }

    pub fn new_with_connection(connection_name: String) -> Result<App> {
        let config = crate::config::Config::load()?;

        let mut app = App {
            state: AppState::Connecting,
            config,
            connection: None,
            connections_list_state: ListState::default(),
            tables_list_state: ListState::default(),
            table_data_state: TableState::default(),
            field_selection_state: None,
            tables: Vec::new(),
            current_table: None,
            table_columns: Vec::new(),
            table_data: Vec::new(),
            current_page: 0,
            max_page: 0,
            items_per_page: 20,
            error_message: None,
            connection_status: Some(format!("Connecting to {}...", connection_name)),
            // Custom query fields
            custom_query_input: String::new(),
            custom_query_cursor_position: 0,
            custom_query_result_columns: Vec::new(),
            custom_query_result_data: Vec::new(),
            custom_query_current_page: 0,
            custom_query_max_page: 0,
            selected_field_value: None,
            field_detail_scroll: 0,
            field_detail_origin_state: None,
        };

        // Pre-select the connection by name if it exists
        let connections = app.config.list_connections();
        if let Some(index) = connections.iter().position(|conn| conn == &connection_name) {
            app.connections_list_state.select(Some(index));
        }

        Ok(app)
    }

    pub fn init(&mut self) {
        let connections = self.config.list_connections();
        if !connections.is_empty() {
            self.connections_list_state.select(Some(0));
        }
    }

    pub async fn connect_to_selected(&mut self) -> Result<()> {
        match self.connections_list_state.selected() {
            Some(index) => {
                let connections = self.config.list_connections();
                if index < connections.len() {
                    let conn_name = &connections[index];
                    self.connect_to_saved_connection(conn_name).await
                } else {
                    Err(anyhow::anyhow!("Invalid connection selection"))
                }
            }
            None => Err(anyhow::anyhow!("No connection selected")),
        }
    }

    pub async fn connect_to_saved_connection(&mut self, name: &str) -> Result<()> {
        self.connection_status = Some(format!("Connecting to {}...", name));
        self.state = AppState::Connecting;

        match self.config.get_connection(name) {
            Some(conn_info) => {
                match self.config.decrypt_connection_password(&conn_info) {
                    Ok(password) => {
                        match DatabaseConnection::connect(
                            &conn_info.host,
                            conn_info.port,
                            &conn_info.database,
                            &conn_info.username,
                            &password,
                        )
                        .await
                        {
                            Ok(connection) => {
                                self.connection = Some(connection);
                                self.connection_status = Some(format!("Connected to {}", name));

                                // Load tables after connecting
                                if let Err(e) = self.load_tables().await {
                                    self.error_message =
                                        Some(format!("Error loading tables: {}", e));
                                    self.state = AppState::ConnectionError;
                                } else {
                                    self.state = AppState::TableList;
                                }
                            }
                            Err(e) => {
                                self.error_message = Some(format!("Connection error: {}", e));
                                self.state = AppState::ConnectionError;
                            }
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Error decrypting password: {}", e));
                        self.state = AppState::ConnectionError;
                    }
                }
            }
            None => {
                self.error_message = Some("Connection not found".to_string());
                self.state = AppState::ConnectionError;
            }
        }

        Ok(())
    }

    pub async fn load_tables(&mut self) -> Result<()> {
        if let Some(conn) = &self.connection {
            self.tables = conn.list_tables().await?;
            if !self.tables.is_empty() {
                self.tables_list_state.select(Some(0));
            }
        }
        Ok(())
    }

    pub async fn load_table_data(&mut self) -> Result<()> {
        if let (Some(table), Some(conn)) = (&self.current_table, &self.connection) {
            let offset = (self.current_page * self.items_per_page) as i64;
            let limit = self.items_per_page as i64;

            let (columns, data) = conn.get_table_data(table, offset, limit).await?;

            self.table_columns = columns;
            self.table_data = data;

            // Calculate max page based on table count
            let total_count = conn.get_table_count(table).await?;
            self.max_page = ((total_count as f64) / (self.items_per_page as f64)).ceil() as u32;

            if !self.table_data.is_empty() {
                self.table_data_state.select(Some(0));
            }
        }
        Ok(())
    }

    pub fn next_connection(&mut self) {
        let i = match self.connections_list_state.selected() {
            Some(i) => {
                if i >= self.config.list_connections().len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.connections_list_state.select(Some(i));
    }

    pub fn previous_connection(&mut self) {
        let i = match self.connections_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.config.list_connections().len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.connections_list_state.select(Some(i));
    }

    pub fn next_table(&mut self) {
        let i = match self.tables_list_state.selected() {
            Some(i) => {
                if i >= self.tables.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.tables_list_state.select(Some(i));
    }

    pub fn previous_table(&mut self) {
        let i = match self.tables_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.tables.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.tables_list_state.select(Some(i));
    }

    pub fn next_row(&mut self) {
        let data_len = if matches!(self.state, AppState::CustomQuery) {
            self.custom_query_result_data.len()
        } else {
            self.table_data.len()
        };

        if data_len == 0 {
            return;
        }

        let i = match self.table_data_state.selected() {
            Some(i) => {
                if i >= data_len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_data_state.select(Some(i));
    }

    pub fn previous_row(&mut self) {
        let data_len = if matches!(self.state, AppState::CustomQuery) {
            self.custom_query_result_data.len()
        } else {
            self.table_data.len()
        };

        if data_len == 0 {
            return;
        }

        let i = match self.table_data_state.selected() {
            Some(i) => {
                if i == 0 {
                    data_len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_data_state.select(Some(i));
    }

    pub fn next_page(&mut self) {
        if self.current_page < self.max_page - 1 {
            self.current_page += 1;
            self.table_data.clear(); // Clear to reload on next render
        }
    }

    pub fn previous_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.table_data.clear(); // Clear to reload on next render
        }
    }

    pub fn next_field(&mut self) {
        // Check if we're in table data view
        if matches!(self.state, AppState::TableData)
            && let Some(selected_row_idx) = self.table_data_state.selected()
            && selected_row_idx < self.table_data.len()
        {
            let num_fields = self.table_data[selected_row_idx].len();
            if num_fields > 0 {
                let next_field_idx = match self.field_selection_state {
                    Some(current_idx) => {
                        if current_idx >= num_fields - 1 {
                            0 // Wrap to first field
                        } else {
                            current_idx + 1
                        }
                    }
                    None => 0, // Start with first field if none selected yet
                };
                self.field_selection_state = Some(next_field_idx);
            }
        }
        // Check if we're in custom query view
        else if matches!(self.state, AppState::CustomQuery)
            && let Some(selected_row_idx) = self.table_data_state.selected()
            && selected_row_idx < self.custom_query_result_data.len()
        {
            let num_fields = self.custom_query_result_data[selected_row_idx].len();
            if num_fields > 0 {
                let next_field_idx = match self.field_selection_state {
                    Some(current_idx) => {
                        if current_idx >= num_fields - 1 {
                            0 // Wrap to first field
                        } else {
                            current_idx + 1
                        }
                    }
                    None => 0, // Start with first field if none selected yet
                };
                self.field_selection_state = Some(next_field_idx);
            }
        }
    }

    pub fn previous_field(&mut self) {
        // Check if we're in table data view
        if matches!(self.state, AppState::TableData)
            && let Some(selected_row_idx) = self.table_data_state.selected()
            && selected_row_idx < self.table_data.len()
        {
            let num_fields = self.table_data[selected_row_idx].len();
            if num_fields > 0 {
                let prev_field_idx = match self.field_selection_state {
                    Some(current_idx) => {
                        if current_idx == 0 {
                            num_fields - 1 // Wrap to last field
                        } else {
                            current_idx - 1
                        }
                    }
                    None => 0, // Start with first field if none selected yet
                };
                self.field_selection_state = Some(prev_field_idx);
            }
        }
        // Check if we're in custom query view
        else if matches!(self.state, AppState::CustomQuery)
            && let Some(selected_row_idx) = self.table_data_state.selected()
            && selected_row_idx < self.custom_query_result_data.len()
        {
            let num_fields = self.custom_query_result_data[selected_row_idx].len();
            if num_fields > 0 {
                let prev_field_idx = match self.field_selection_state {
                    Some(current_idx) => {
                        if current_idx == 0 {
                            num_fields - 1 // Wrap to last field
                        } else {
                            current_idx - 1
                        }
                    }
                    None => 0, // Start with first field if none selected yet
                };
                self.field_selection_state = Some(prev_field_idx);
            }
        }
    }

    pub fn enter_field_detail_view(&mut self) {
        // Check if we're in table data view
        if matches!(self.state, AppState::TableData)
            && let Some(selected_row_idx) = self.table_data_state.selected()
            && selected_row_idx < self.table_data.len()
        {
            if let Some(selected_field_idx) = self.field_selection_state {
                if selected_field_idx < self.table_data[selected_row_idx].len() {
                    // Store the selected field value for detailed view
                    self.selected_field_value =
                        Some(self.table_data[selected_row_idx][selected_field_idx].clone());
                    // Store the original state for returning later
                    self.field_detail_origin_state = Some(AppState::TableData);
                    // Switch to field detail view
                    self.state = AppState::FieldDetail;
                    self.field_detail_scroll = 0; // Reset scroll to top
                }
            } else if !self.table_data[selected_row_idx].is_empty() {
                // If no field is selected yet, select the first field
                self.field_selection_state = Some(0);
                self.selected_field_value = Some(self.table_data[selected_row_idx][0].clone());
                // Store the original state for returning later
                self.field_detail_origin_state = Some(AppState::TableData);
                self.state = AppState::FieldDetail;
                self.field_detail_scroll = 0; // Reset scroll to top
            }
        }
        // Check if we're in custom query view
        else if matches!(self.state, AppState::CustomQuery)
            && let Some(selected_row_idx) = self.table_data_state.selected()
            && selected_row_idx < self.custom_query_result_data.len()
        {
            if let Some(selected_field_idx) = self.field_selection_state {
                if selected_field_idx < self.custom_query_result_data[selected_row_idx].len() {
                    // Store the selected field value for detailed view
                    self.selected_field_value = Some(
                        self.custom_query_result_data[selected_row_idx][selected_field_idx].clone(),
                    );
                    // Store the original state for returning later
                    self.field_detail_origin_state = Some(AppState::CustomQuery);
                    // Switch to field detail view
                    self.state = AppState::FieldDetail;
                    self.field_detail_scroll = 0; // Reset scroll to top
                }
            } else if !self.custom_query_result_data[selected_row_idx].is_empty() {
                // If no field is selected yet, select the first field
                self.field_selection_state = Some(0);
                self.selected_field_value =
                    Some(self.custom_query_result_data[selected_row_idx][0].clone());
                // Store the original state for returning later
                self.field_detail_origin_state = Some(AppState::CustomQuery);
                self.state = AppState::FieldDetail;
                self.field_detail_scroll = 0; // Reset scroll to top
            }
        }
    }

    pub fn scroll_field_detail_up(&mut self) {
        if self.field_detail_scroll > 0 {
            self.field_detail_scroll -= 1;
        }
    }

    pub fn scroll_field_detail_down(&mut self) {
        // We'll update the scroll based on content when rendering, so just increment here
        // We can't determine the total lines without knowing the terminal height here
        self.field_detail_scroll += 1;
    }

    pub async fn execute_custom_query(&mut self) -> Result<()> {
        if let Some(conn) = &self.connection {
            let offset = (self.custom_query_current_page * self.items_per_page) as i64;
            let limit = self.items_per_page as i64;

            let (columns, data) = conn
                .execute_custom_query(&self.custom_query_input, offset, limit)
                .await?;

            self.custom_query_result_columns = columns;
            self.custom_query_result_data = data;

            // Calculate max page based on query count
            let total_count = conn.get_query_row_count(&self.custom_query_input).await?;
            self.custom_query_max_page =
                ((total_count as f64) / (self.items_per_page as f64)).ceil() as u32;

            if !self.custom_query_result_data.is_empty() {
                self.table_data_state.select(Some(0));
            }
        }
        Ok(())
    }

    pub fn next_custom_query_page(&mut self) {
        if self.custom_query_current_page < self.custom_query_max_page - 1 {
            self.custom_query_current_page += 1;
            self.custom_query_result_data.clear(); // Clear to reload on next render
        }
    }

    pub fn previous_custom_query_page(&mut self) {
        if self.custom_query_current_page > 0 {
            self.custom_query_current_page -= 1;
            self.custom_query_result_data.clear(); // Clear to reload on next render
        }
    }
}

pub async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    connection_name: String,
) -> io::Result<()> {
    // Automatically connect to the specified connection if we're in the Connecting state
    if matches!(app.state, AppState::Connecting)
        && let Err(e) = app.connect_to_saved_connection(&connection_name).await
    {
        app.error_message = Some(e.to_string());
        app.state = AppState::ConnectionError;
    }

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match app.state {
                AppState::ConnectionSelection => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Esc => return Ok(()), // Keep ESC to quit from main menu
                    KeyCode::Down => app.next_connection(),
                    KeyCode::Up => app.previous_connection(),
                    KeyCode::Enter => {
                        // Attempt to connect to the selected database
                        if let Err(e) = app.connect_to_selected().await {
                            app.error_message = Some(e.to_string());
                            app.state = AppState::ConnectionError;
                        }
                    }
                    _ => {}
                },
                AppState::Connecting => {
                    // In connecting state, allow quit with 'q' or go back with ESC
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Esc => app.state = AppState::ConnectionSelection,
                        _ => {}
                    }
                }
                AppState::ConnectionError => {
                    // In error state, allow quit or return to connection selection
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Esc => {
                            app.state = AppState::ConnectionSelection;
                            app.error_message = None; // Clear error when going back
                        }
                        KeyCode::Char('c') => {
                            app.state = AppState::ConnectionSelection;
                            app.error_message = None; // Clear error when going back
                        }
                        _ => {}
                    }
                }
                AppState::TableList => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Esc => app.state = AppState::ConnectionSelection,
                    KeyCode::Down => app.next_table(),
                    KeyCode::Up => app.previous_table(),
                    KeyCode::Enter => {
                        // Load the selected table's data
                        if let Some(index) = app.tables_list_state.selected()
                            && index < app.tables.len()
                        {
                            app.current_table = Some(app.tables[index].clone());
                            // Reset pagination when loading a new table
                            app.current_page = 0;
                            app.state = AppState::TableData;

                            // Load data for the selected table
                            if let Err(e) = app.load_table_data().await {
                                app.error_message =
                                    Some(format!("Error loading table data: {}", e));
                                app.state = AppState::ConnectionError;
                            }
                        }
                    }
                    KeyCode::Char('c') => app.state = AppState::ConnectionSelection,
                    KeyCode::Char('s') => {
                        // Enter custom query mode
                        app.state = AppState::CustomQueryInput;
                        app.custom_query_input.clear();
                    }
                    _ => {}
                },
                AppState::TableData => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Esc => {
                        app.state = AppState::TableList;
                        app.current_table = None;
                        app.field_selection_state = None; // Reset field selection
                    }
                    KeyCode::Down => {
                        app.next_row();
                        app.field_selection_state = None; // Reset field selection when changing rows
                    }
                    KeyCode::Up => {
                        app.previous_row();
                        app.field_selection_state = None; // Reset field selection when changing rows
                    }
                    KeyCode::Left => app.previous_field(), // Add left arrow for field navigation
                    KeyCode::Right => app.next_field(),    // Add right arrow for field navigation
                    KeyCode::Enter => app.enter_field_detail_view(), // Add enter to view field detail
                    KeyCode::PageDown => {
                        app.next_page();
                        app.field_selection_state = None; // Reset field selection when changing pages
                        // Reload data for the new page
                        if let Err(e) = app.load_table_data().await {
                            app.error_message = Some(format!("Error loading table data: {}", e));
                            app.state = AppState::ConnectionError;
                        }
                    }
                    KeyCode::PageUp => {
                        app.previous_page();
                        app.field_selection_state = None; // Reset field selection when changing pages
                        // Reload data for the new page
                        if let Err(e) = app.load_table_data().await {
                            app.error_message = Some(format!("Error loading table data: {}", e));
                            app.state = AppState::ConnectionError;
                        }
                    }
                    KeyCode::Char('t') => {
                        app.state = AppState::TableList;
                        app.current_table = None;
                        app.field_selection_state = None; // Reset field selection
                    }
                    KeyCode::Char('c') => {
                        app.state = AppState::ConnectionSelection;
                        app.current_table = None;
                        app.field_selection_state = None; // Reset field selection
                    }
                    KeyCode::Char('s') => {
                        // Enter custom query mode
                        app.state = AppState::CustomQueryInput;
                        app.custom_query_input.clear();
                        app.field_selection_state = None; // Reset field selection
                    }
                    _ => {}
                },
                AppState::CustomQueryInput => match key.code {
                    KeyCode::Esc => app.state = AppState::TableList,
                    KeyCode::Enter => {
                        // Execute the custom query
                        if !app.custom_query_input.trim().is_empty() {
                            // Reset pagination
                            app.custom_query_current_page = 0;
                            app.state = AppState::CustomQuery;

                            // Execute the query
                            if let Err(e) = app.execute_custom_query().await {
                                app.error_message = Some(format!("Error executing query: {}", e));
                                app.state = AppState::ConnectionError;
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        if app.custom_query_cursor_position > 0 {
                            // Find the previous character boundary
                            let mut chars: Vec<char> = app.custom_query_input.chars().collect();
                            if app.custom_query_cursor_position <= chars.len() {
                                chars.remove(app.custom_query_cursor_position - 1);
                                app.custom_query_input = chars.into_iter().collect();
                                app.custom_query_cursor_position -= 1;
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        // Convert to chars, insert at position, then convert back
                        let mut chars: Vec<char> = app.custom_query_input.chars().collect();
                        if app.custom_query_cursor_position <= chars.len() {
                            chars.insert(app.custom_query_cursor_position, c);
                            app.custom_query_input = chars.into_iter().collect();
                            app.custom_query_cursor_position += 1;
                        }
                    }
                    KeyCode::Left => {
                        if app.custom_query_cursor_position > 0 {
                            app.custom_query_cursor_position -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if app.custom_query_cursor_position < app.custom_query_input.len() {
                            app.custom_query_cursor_position += 1;
                        }
                    }
                    KeyCode::Home => {
                        app.custom_query_cursor_position = 0;
                    }
                    KeyCode::End => {
                        app.custom_query_cursor_position = app.custom_query_input.len();
                    }
                    _ => {}
                },
                AppState::FieldDetail => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Esc => {
                        // Return to the original state
                        app.state = app
                            .field_detail_origin_state
                            .clone()
                            .unwrap_or(AppState::TableData);
                    }
                    KeyCode::Up => app.scroll_field_detail_up(),
                    KeyCode::Down => app.scroll_field_detail_down(),
                    _ => {}
                },
                AppState::CustomQuery => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Esc => {
                        app.state = AppState::CustomQueryInput;
                        app.field_selection_state = None; // Reset field selection
                    }
                    KeyCode::Down => {
                        app.next_row();
                        app.field_selection_state = None; // Reset field selection when changing rows
                    }
                    KeyCode::Up => {
                        app.previous_row();
                        app.field_selection_state = None; // Reset field selection when changing rows
                    }
                    KeyCode::Left => app.previous_field(), // Add left arrow for field navigation
                    KeyCode::Right => app.next_field(),    // Add right arrow for field navigation
                    KeyCode::Enter => app.enter_field_detail_view(), // Add enter to view field detail
                    KeyCode::PageDown => {
                        app.next_custom_query_page();
                        app.field_selection_state = None; // Reset field selection when changing pages
                        // Reload data for the new page
                        if let Err(e) = app.execute_custom_query().await {
                            app.error_message = Some(format!("Error loading query data: {}", e));
                            app.state = AppState::ConnectionError;
                        }
                    }
                    KeyCode::PageUp => {
                        app.previous_custom_query_page();
                        app.field_selection_state = None; // Reset field selection when changing pages
                        // Reload data for the new page
                        if let Err(e) = app.execute_custom_query().await {
                            app.error_message = Some(format!("Error loading query data: {}", e));
                            app.state = AppState::ConnectionError;
                        }
                    }
                    KeyCode::Char('t') => {
                        app.state = AppState::TableList;
                        app.field_selection_state = None; // Reset field selection
                    }
                    KeyCode::Char('c') => {
                        app.state = AppState::ConnectionSelection;
                        app.field_selection_state = None; // Reset field selection
                    }
                    KeyCode::Char('s') => {
                        // Go back to query input
                        app.state = AppState::CustomQueryInput;
                        app.field_selection_state = None; // Reset field selection
                    }
                    _ => {}
                },
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // If there's a connection status message, show it at the top
    if let Some(ref status) = app.connection_status {
        let status_paragraph = Paragraph::new(Text::styled(
            status.as_str(),
            Style::default().fg(Color::Green),
        ))
        .block(Block::default().borders(Borders::NONE));
        let status_area = ratatui::layout::Rect {
            x: 0,
            y: 0,
            width: size.width,
            height: 1,
        };
        f.render_widget(status_paragraph, status_area);
    }

    // If there's an error message, show it at the top
    if let Some(ref error) = app.error_message {
        let error_paragraph = Paragraph::new(Text::styled(
            error.as_str(),
            Style::default().fg(Color::Red),
        ))
        .block(Block::default().borders(Borders::NONE));
        let error_area = ratatui::layout::Rect {
            x: 0,
            y: 0,
            width: size.width,
            height: 1,
        };
        f.render_widget(error_paragraph, error_area);
    }

    // Main content area
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)].as_ref()) // Status bar + main content
        .split(size);

    let main_area = content_chunks[1];

    match app.state {
        AppState::ConnectionSelection => render_connection_selection(f, app, main_area),
        AppState::Connecting => render_connecting(f, app, main_area),
        AppState::ConnectionError => render_connection_error(f, app, main_area),
        AppState::TableList => render_table_list(f, app, main_area),
        AppState::TableData => render_table_data(f, app, main_area),
        AppState::FieldDetail => render_field_detail(f, app, main_area),
        AppState::CustomQueryInput => render_custom_query_input(f, app, main_area),
        AppState::CustomQuery => render_custom_query_results(f, app, main_area),
    }
}

fn render_connection_selection(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let connections = app.config.list_connections();

    let items: Vec<ListItem> = connections
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select Connection"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut app.connections_list_state);
}

fn render_connecting(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let text = if let Some(ref status) = app.connection_status {
        status.as_str()
    } else {
        "Connecting..."
    };

    let paragraph = Paragraph::new(Span::raw(text))
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .style(Style::default().fg(Color::Yellow));

    f.render_widget(paragraph, area);

    let help_text = Paragraph::new(Span::raw("Press ESC to go back, 'q' to quit"))
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().add_modifier(Modifier::ITALIC));

    // Position help text at the bottom
    let help_area = ratatui::layout::Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(2),
        width: area.width,
        height: 2,
    };
    f.render_widget(help_text, help_area);
}

fn render_connection_error(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let error_text = if let Some(ref error) = app.error_message {
        error.as_str()
    } else {
        "Unknown error occurred"
    };

    let paragraph = Paragraph::new(Span::raw(error_text))
        .block(Block::default().borders(Borders::ALL).title("Error"))
        .style(Style::default().fg(Color::Red));

    f.render_widget(paragraph, area);

    let help_text = Paragraph::new(Span::raw(
        "Press 'c' or ESC to go back to connection selection, 'q' to quit",
    ))
    .block(Block::default().borders(Borders::NONE))
    .style(Style::default().add_modifier(Modifier::ITALIC));

    // Position help text at the bottom
    let help_area = ratatui::layout::Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(2),
        width: area.width,
        height: 2,
    };
    f.render_widget(help_text, help_area);
}

fn render_table_list(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .tables
        .iter()
        .map(|name| ListItem::new(name.as_str()))
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Tables"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut app.tables_list_state);

    let help_text = Paragraph::new(Span::raw(
        "Use ↑↓ to navigate, Enter to select, 's' for SQL query, 'c' for connections, ESC for back, 'q' to quit",
    ))
    .block(Block::default().borders(Borders::NONE))
    .style(Style::default().add_modifier(Modifier::ITALIC));

    // Position help text at the bottom
    let help_area = ratatui::layout::Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(2),
        width: area.width,
        height: 2,
    };
    f.render_widget(help_text, help_area);
}

fn render_table_data(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    // Split each column name into name and type (if available)
    let mut column_names: Vec<String> = Vec::new();
    let mut column_types: Vec<String> = Vec::new();

    for column in &app.table_columns {
        if let Some(pos) = column.find(" (") {
            let name = &column[..pos];
            let type_part = &column[pos + 2..column.len() - 1]; // Remove the trailing ')'
            column_names.push(name.to_string());
            column_types.push(type_part.to_string());
        } else {
            // If no type information is present, just use the column name as is
            column_names.push(column.to_string());
            column_types.push("".to_string());
        }
    }

    // Create headers for the table - column names
    let header_names: Vec<Span> = column_names.iter().map(|c| Span::raw(c.as_str())).collect();

    // Create headers for the table - data types
    let header_types: Vec<Span> = column_types.iter().map(|t| Span::raw(t.as_str())).collect();

    // Create header rows
    let header_row_names = Row::new(header_names)
        .height(1)
        .style(Style::default().add_modifier(Modifier::BOLD));

    let header_row_types = Row::new(header_types)
        .height(1)
        .style(Style::default().add_modifier(Modifier::ITALIC));

    // Create rows for the table
    let rows: Vec<Row> = app
        .table_data
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let cells: Vec<Span> = row
                .iter()
                .enumerate()
                .map(|(j, cell)| {
                    // Check if this cell is selected
                    let mut cell_style = Style::default();
                    if Some(i) == app.table_data_state.selected()
                        && app.field_selection_state.is_some()
                        && app.field_selection_state.unwrap() == j
                    {
                        // This is the currently selected field in the selected row
                        cell_style = Style::default().bg(Color::Yellow).fg(Color::Black);
                    } else if Some(i) == app.table_data_state.selected() {
                        // This is in the currently selected row
                        cell_style = Style::default().bg(Color::LightBlue);
                    }
                    Span::styled(cell.as_str(), cell_style)
                })
                .collect();
            Row::new(cells).height(1)
        })
        .collect();

    // Combine headers and data rows into a single table
    let mut table_rows = Vec::new();
    table_rows.push(header_row_names);
    table_rows.push(header_row_types);
    table_rows.extend(rows);

    let widths: Vec<Constraint> = app
        .table_columns
        .iter()
        .map(|_| Constraint::Percentage(100 / app.table_columns.len().max(1) as u16))
        .collect();

    let table = Table::new(table_rows, widths).block(Block::default().borders(Borders::ALL).title(
        format!(
            "Table: {} (Page {}/{})",
            app.current_table.as_ref().unwrap_or(&"Unknown".to_string()),
            app.current_page + 1,
            app.max_page
        ),
    ));

    f.render_stateful_widget(table, area, &mut app.table_data_state);

    let help_text = Paragraph::new(Span::raw("Use ↑↓ to navigate rows, ←→ to navigate fields in row, Enter to view field detail, PageUp/PageDown to change pages, 't' for tables, ESC for back, 'c' for connections, 'q' to quit"))
        .block(Block::default().borders(Borders::NONE))
        .style(Style::default().add_modifier(Modifier::ITALIC));

    // Position help text at the bottom
    let help_area = ratatui::layout::Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(2),
        width: area.width,
        height: 2,
    };
    f.render_widget(help_text, help_area);
}

fn render_field_detail(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(90), Constraint::Percentage(10)].as_ref())
        .split(area);

    let value_to_display = app
        .selected_field_value
        .as_deref()
        .unwrap_or("No value selected");

    // Create a paragraph with the field value, potentially long text
    let field_para = Paragraph::new(Text::from(value_to_display))
        .block(Block::default().borders(Borders::ALL).title("Field Detail"))
        .style(Style::default().fg(Color::White))
        .wrap(ratatui::widgets::Wrap { trim: false })
        .scroll((app.field_detail_scroll, 0)); // Add vertical scrolling

    f.render_widget(field_para, chunks[0]);

    let help_text = Paragraph::new(Span::raw(
        "Use ↑↓ to scroll, ESC to return to table view, 'q' to quit",
    ))
    .block(Block::default().borders(Borders::NONE))
    .style(Style::default().add_modifier(Modifier::ITALIC));

    f.render_widget(help_text, chunks[1]);
}

fn render_custom_query_input(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(area);

    // Input area
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("Enter SQL Query");

    // Create input text with cursor at the correct position
    let input_text = {
        let mut chars: Vec<char> = app.custom_query_input.chars().collect();
        if std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % 1000
            < 500
        {
            // Insert blinking cursor at the current cursor position
            if app.custom_query_cursor_position <= chars.len() {
                chars.insert(app.custom_query_cursor_position, '|');
            }
        }
        chars.into_iter().collect::<String>()
    };

    let input_paragraph = Paragraph::new(input_text)
        .block(input_block)
        .style(Style::default().fg(Color::Yellow));

    f.render_widget(input_paragraph, chunks[0]);

    // Help text
    let help_text = Paragraph::new(Span::raw(
        "Type your SQL query and press Enter to execute. Press ESC to go back to table list.",
    ))
    .block(Block::default().borders(Borders::NONE))
    .style(Style::default().add_modifier(Modifier::ITALIC));

    f.render_widget(help_text, chunks[1]);
}

fn render_custom_query_results(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    // Create headers for the table
    let header_names: Vec<Span> = app
        .custom_query_result_columns
        .iter()
        .map(|c| Span::raw(c.as_str()))
        .collect();

    // Create header rows
    let header_row_names = Row::new(header_names)
        .height(1)
        .style(Style::default().add_modifier(Modifier::BOLD));

    // Create rows for the table
    let rows: Vec<Row> = app
        .custom_query_result_data
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let cells: Vec<Span> = row
                .iter()
                .enumerate()
                .map(|(j, cell)| {
                    // Check if this cell is selected
                    let mut cell_style = Style::default();
                    if Some(i) == app.table_data_state.selected()
                        && app.field_selection_state.is_some()
                        && app.field_selection_state.unwrap() == j
                    {
                        // This is the currently selected field in the selected row
                        cell_style = Style::default().bg(Color::Yellow).fg(Color::Black);
                    } else if Some(i) == app.table_data_state.selected() {
                        // This is in the currently selected row
                        cell_style = Style::default().bg(Color::LightBlue);
                    }
                    Span::styled(cell.as_str(), cell_style)
                })
                .collect();
            Row::new(cells).height(1)
        })
        .collect();

    // Combine headers and data rows into a single table
    let mut table_rows = Vec::new();
    table_rows.push(header_row_names);
    table_rows.extend(rows);

    let widths: Vec<Constraint> = app
        .custom_query_result_columns
        .iter()
        .map(|_| Constraint::Percentage(100 / app.custom_query_result_columns.len().max(1) as u16))
        .collect();

    let table = Table::new(table_rows, widths).block(Block::default().borders(Borders::ALL).title(
        format!(
            "Query Results (Page {}/{})",
            app.custom_query_current_page + 1,
            app.custom_query_max_page
        ),
    ));

    f.render_stateful_widget(table, area, &mut app.table_data_state);

    let help_text = Paragraph::new(Span::raw(
        "Use ↑↓ to navigate rows, ←→ to navigate fields in row, Enter to view field detail, PageUp/PageDown to change pages, 's' for query input, 't' for tables, 'c' for connections, ESC for back, 'q' to quit",
    ))
    .block(Block::default().borders(Borders::NONE))
    .style(Style::default().add_modifier(Modifier::ITALIC));

    // Position help text at the bottom
    let help_area = ratatui::layout::Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(2),
        width: area.width,
        height: 2,
    };
    f.render_widget(help_text, help_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        unsafe {
            std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
        }

        let app = App::new().unwrap();
        assert_eq!(app.state, AppState::ConnectionSelection);
        assert!(app.connection.is_none());
        assert!(app.tables.is_empty());
        assert!(app.table_data.is_empty());
    }

    #[test]
    fn test_new_with_connection() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        unsafe {
            std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
        }

        let app = App::new_with_connection("test_conn".to_string()).unwrap();
        assert_eq!(app.state, AppState::Connecting);
        assert!(app.connection_status.is_some());
        assert!(
            app.connection_status
                .unwrap()
                .contains("Connecting to test_conn")
        );
    }

    #[test]
    fn test_navigation_between_connections() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        unsafe {
            std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
        }

        let mut app = App::new().unwrap();

        // Manually add some connections to the config for testing
        let conn1 = crate::config::ConnectionInfo {
            host: "localhost".to_string(),
            port: 5432,
            database: "test_db1".to_string(),
            username: "user1".to_string(),
            password: "pass1".to_string(),
            name: "conn1".to_string(),
        };

        let conn2 = crate::config::ConnectionInfo {
            host: "localhost".to_string(),
            port: 5433,
            database: "test_db2".to_string(),
            username: "user2".to_string(),
            password: "pass2".to_string(),
            name: "conn2".to_string(),
        };

        app.config.add_connection(conn1).unwrap();
        app.config.add_connection(conn2).unwrap();

        // Test initial state
        assert_eq!(app.connections_list_state.selected(), None);

        // Initialize app to select first connection
        app.init();
        assert_eq!(app.connections_list_state.selected(), Some(0));

        // Test next_connection when on first item - should go to second
        app.next_connection();
        assert_eq!(app.connections_list_state.selected(), Some(1));

        // Test next_connection when on last item - should wrap to first
        app.next_connection();
        assert_eq!(app.connections_list_state.selected(), Some(0));

        // Test previous_connection when on first item - should wrap to last
        app.previous_connection();
        assert_eq!(app.connections_list_state.selected(), Some(1));
    }

    #[test]
    fn test_navigation_between_tables() {
        let mut app = App::new().unwrap();

        // Add some mock tables for testing
        app.tables = vec![
            "table1".to_string(),
            "table2".to_string(),
            "table3".to_string(),
        ];
        app.tables_list_state.select(Some(0));

        // Test next_table when on first item - should go to second
        app.next_table();
        assert_eq!(app.tables_list_state.selected(), Some(1));

        // Test next_table when on last item - should wrap to first
        app.next_table();
        app.next_table(); // move to last item
        assert_eq!(app.tables_list_state.selected(), Some(0));

        // Test previous_table when on first item - should wrap to last
        app.previous_table();
        assert_eq!(app.tables_list_state.selected(), Some(2));
    }

    #[test]
    fn test_navigation_between_rows() {
        let mut app = App::new().unwrap();

        // Add some mock table data for testing
        app.table_data = vec![
            vec!["row1_col1".to_string(), "row1_col2".to_string()],
            vec!["row2_col1".to_string(), "row2_col2".to_string()],
            vec!["row3_col1".to_string(), "row3_col2".to_string()],
        ];
        app.table_data_state.select(Some(0));

        // Test next_row when on first item - should go to second
        app.next_row();
        assert_eq!(app.table_data_state.selected(), Some(1));

        // Test next_row when on last item - should wrap to first
        app.next_row();
        app.next_row(); // move to last item
        assert_eq!(app.table_data_state.selected(), Some(0));

        // Test previous_row when on first item - should wrap to last
        app.previous_row();
        assert_eq!(app.table_data_state.selected(), Some(2));
    }

    #[test]
    fn test_navigation_in_custom_query_results() {
        let mut app = App::new().unwrap();

        // Set state to CustomQuery and add mock custom query data
        app.state = AppState::CustomQuery;
        app.custom_query_result_data = vec![
            vec!["query_row1_col1".to_string(), "query_row1_col2".to_string()],
            vec!["query_row2_col1".to_string(), "query_row2_col2".to_string()],
            vec!["query_row3_col1".to_string(), "query_row3_col2".to_string()],
        ];
        app.table_data_state.select(Some(0));

        // Test next_row in custom query mode
        app.next_row();
        assert_eq!(app.table_data_state.selected(), Some(1));

        // Test previous_row in custom query mode
        app.previous_row();
        assert_eq!(app.table_data_state.selected(), Some(0));

        // Test wrapping behavior
        app.previous_row();
        assert_eq!(app.table_data_state.selected(), Some(2)); // Should wrap to last

        app.next_row();
        assert_eq!(app.table_data_state.selected(), Some(0)); // Should wrap to first
    }

    #[test]
    fn test_page_navigation() {
        let mut app = App::new().unwrap();

        // Set up pagination
        app.current_page = 2;
        app.max_page = 5;

        // Test next_page
        app.next_page();
        assert_eq!(app.current_page, 3);

        // Test previous_page
        app.previous_page();
        assert_eq!(app.current_page, 2);

        // Test boundary conditions
        app.current_page = 0;
        app.previous_page();
        assert_eq!(app.current_page, 0); // Should not go below 0

        app.current_page = 4;
        app.max_page = 5; // So valid pages are 0-4
        app.next_page();
        assert_eq!(app.current_page, 4); // Should not exceed max_page - 1
    }

    #[test]
    fn test_app_state_transitions() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        unsafe {
            std::env::set_var("HOME", temp_dir.path().to_str().unwrap());
        }

        let mut app = App::new().unwrap();

        // Start in ConnectionSelection
        assert_eq!(app.state, AppState::ConnectionSelection);

        // Transition to Connecting
        app.state = AppState::Connecting;
        assert_eq!(app.state, AppState::Connecting);

        // Transition to TableList
        app.state = AppState::TableList;
        assert_eq!(app.state, AppState::TableList);

        // Transition to TableData
        app.state = AppState::TableData;
        assert_eq!(app.state, AppState::TableData);

        // Transition to ConnectionError
        app.state = AppState::ConnectionError;
        assert_eq!(app.state, AppState::ConnectionError);
    }
}
