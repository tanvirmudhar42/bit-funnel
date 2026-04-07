use bitfunnel::BitFunnelIndex;
use clap::Parser;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, stdout, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "bitfunnel-cli")]
#[command(about = "Interactive BitFunnel search CLI - search files as you type")]
#[command(long_about = "An interactive terminal-based search tool that updates results in real-time as you type. Perfect for quickly finding files containing specific keywords.")]
struct Args {
    /// Directory or files to index
    #[arg(short, long, default_value = ".")]
    path: String,

    /// Recursively index directories
    #[arg(short, long)]
    recursive: bool,

    /// File extensions to include (e.g., txt,rs,md)
    #[arg(short, long, value_delimiter = ',')]
    extensions: Option<Vec<String>>,

    /// Save the index to a file after indexing
    #[arg(long)]
    save: Option<String>,

    /// Load the index from a file instead of indexing
    #[arg(long)]
    load: Option<String>,
}

struct App {
    query: String,
    results: Vec<bitfunnel::SearchResult>,
    selected: usize,
    scroll: usize,
    document_count: usize,
    list_state: ListState,
}

impl App {
    fn new(document_count: usize) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            scroll: 0,
            document_count,
            list_state,
        }
    }

    fn update_search(&mut self, index: &BitFunnelIndex) {
        if self.query.trim().is_empty() {
            self.results.clear();
            self.selected = 0;
            self.scroll = 0;
            self.list_state.select(None);
        } else {
            self.results = index.search(&self.query);
            self.selected = 0;
            self.scroll = 0;
            if !self.results.is_empty() {
                self.list_state.select(Some(0));
            } else {
                self.list_state.select(None);
            }
        }
    }

    fn next_result(&mut self) {
        if !self.results.is_empty() {
            self.selected = (self.selected + 1) % self.results.len();
            // Auto-scroll to keep selected item visible (assuming ~20 visible items)
            let visible_count = 20;
            if self.selected >= self.scroll + visible_count {
                self.scroll = self.selected.saturating_sub(visible_count - 1);
            }
        }
    }

    fn prev_result(&mut self) {
        if !self.results.is_empty() {
            self.selected = if self.selected == 0 {
                self.results.len() - 1
            } else {
                self.selected - 1
            };
            // Auto-scroll to keep selected item visible
            if self.selected < self.scroll {
                self.scroll = self.selected;
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize index
    let mut index = if let Some(load_path) = args.load {
        println!("Loading index from {}...", load_path);
        BitFunnelIndex::load_from_file(load_path)?
    } else {
        let mut index = BitFunnelIndex::with_defaults();

        println!("Indexing files...");
        let files = collect_files(&args.path, args.recursive, &args.extensions)?;

        let mut indexed_count = 0;
        let mut error_count = 0;

        for file in &files {
            match index.index_file(file) {
                Ok(_) => {
                    indexed_count += 1;
                    if indexed_count % 10 == 0 {
                        print!("\rIndexed {} files...", indexed_count);
                        io::stdout().flush()?;
                    }
                }
                Err(e) => {
                    error_count += 1;
                    eprintln!("\nWarning: Failed to index {}: {}", file.to_string_lossy(), e);
                }
            }
        }

        println!("\rIndexed {} files ({} errors)", indexed_count, error_count);
        index
    };

    if let Some(save_path) = args.save {
        println!("Saving index to {}...", save_path);
        index.save_to_file(save_path)?;
    }
    
    println!("\nStarting interactive search...\n");

    // Run interactive search
    interactive_search(&index)?;

    Ok(())
}

fn collect_files(
    path: &str,
    recursive: bool,
    extensions: &Option<Vec<String>>,
) -> anyhow::Result<Vec<PathBuf>> {
    let path = PathBuf::from(path);
    let mut files = Vec::new();

    if path.is_file() {
        if should_index(&path, extensions) {
            files.push(path);
        }
    } else if path.is_dir() {
        if recursive {
            collect_files_recursive(&path, &mut files, extensions)?;
        } else {
            for entry in std::fs::read_dir(&path)? {
                let entry = entry?;
                let file_path = entry.path();
                if file_path.is_file() && should_index(&file_path, extensions) {
                    files.push(file_path);
                }
            }
        }
    } else {
        anyhow::bail!("Path does not exist: {}", path.display());
    }

    Ok(files)
}

fn collect_files_recursive(
    dir: &std::path::Path,
    files: &mut Vec<PathBuf>,
    extensions: &Option<Vec<String>>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_files_recursive(&path, files, extensions)?;
        } else if path.is_file() && should_index(&path, extensions) {
            files.push(path);
        }
    }
    Ok(())
}

fn should_index(path: &std::path::Path, extensions: &Option<Vec<String>>) -> bool {
    if let Some(exts) = extensions {
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            exts.iter().any(|e| e.to_lowercase() == ext_str)
        } else {
            false
        }
    } else {
        true
    }
}

fn interactive_search(index: &BitFunnelIndex) -> anyhow::Result<()> {
    // Setup terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen, EnableMouseCapture)?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(index.document_count());
    let mut show_details = false;
    let mut details_result: Option<bitfunnel::SearchResult> = None;

    let mut last_query = String::new();
    
    loop {
        // Update search results only if query changed
        if app.query != last_query {
            app.update_search(index);
            last_query = app.query.clone();
        }

        // Render UI
        terminal.draw(|f| {
            if show_details {
                render_file_details(f, &details_result);
            } else {
                render_main(f, &mut app);
            }
        })?;

        // Handle input
        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = crossterm::event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char(c) if !show_details => {
                            app.query.push(c);
                        }
                        KeyCode::Backspace if !show_details => {
                            app.query.pop();
                        }
                        KeyCode::Down if !show_details && !app.results.is_empty() => {
                            app.next_result();
                        }
                        KeyCode::Up if !show_details && !app.results.is_empty() => {
                            app.prev_result();
                        }
                        KeyCode::Enter if !show_details => {
                            if let Some(result) = app.results.get(app.selected) {
                                show_details = true;
                                details_result = Some(result.clone());
                            }
                        }
                        KeyCode::Esc => {
                            if show_details {
                                show_details = false;
                                details_result = None;
                            } else {
                                break;
                            }
                        }
                        KeyCode::Char('q') if !show_details => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Restore terminal
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}

fn render_main(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // Create layout: header, search bar, results, footer
    let chunks = Layout::default()
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(3),  // Search bar
            Constraint::Min(0),     // Results
            Constraint::Length(3),  // Footer
        ])
        .split(size);

    // Header
    let header_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    let header_text = Line::from(vec![
        Span::styled(" BitFunnel ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("Search CLI"),
    ]);
    f.render_widget(Paragraph::new(header_text).block(header_block), chunks[0]);

    // Search bar
    let search_block = Block::default()
        .borders(Borders::ALL)
        .title(" Search ")
        .style(Style::default().fg(Color::Yellow));
    let search_text = if app.query.is_empty() {
        Line::from(Span::styled(
            "Type to search...",
            Style::default().fg(Color::Gray),
        ))
    } else {
        Line::from(vec![
            Span::raw(&app.query),
            Span::styled("█", Style::default().fg(Color::Yellow)),
        ])
    };
    f.render_widget(Paragraph::new(search_text).block(search_block), chunks[1]);

    // Results
    if app.query.trim().is_empty() {
        let help_text = Paragraph::new("Start typing to search files...")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title(" Results "));
        f.render_widget(help_text, chunks[2]);
    } else if app.results.is_empty() {
        let no_results = Paragraph::new("No results found")
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL).title(" Results "));
        f.render_widget(no_results, chunks[2]);
    } else {
        let items: Vec<ListItem> = app
            .results
            .iter()
            .enumerate()
            .skip(app.scroll)
            .take(20)
            .map(|(display_idx, result)| {
                // display_idx is relative to scroll, but we need absolute index for comparison
                let absolute_idx = app.scroll + display_idx;
                let is_selected = absolute_idx == app.selected;
                
                let path_str = result.document.path.clone();
                let preview = get_preview(&result.document.content, &app.query, 50);
                
                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default()
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!("{:3}. ", absolute_idx + 1),
                            style.fg(Color::Cyan).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            path_str.clone(),
                            style.fg(Color::White).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            format!("     Score: {:.1}% | ", result.score),
                            style.fg(Color::Gray),
                        ),
                        Span::styled(preview.clone(), style.fg(Color::Gray)),
                    ]),
                ])
                .style(style)
            })
            .collect();

        let results_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" Results ({}) ", app.results.len()));
        
        let list = List::new(items)
            .block(results_block)
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));
        
        // Ensure selected index is valid
        if app.selected >= app.results.len() {
            app.selected = 0;
        }
        
        // Update list state to reflect the selected item relative to scroll
        let relative_selection = app.selected.saturating_sub(app.scroll);
        // Make sure relative_selection is within bounds of visible items
        let max_relative = (app.results.len().saturating_sub(app.scroll)).min(20) - 1;
        let safe_relative = relative_selection.min(max_relative);
        app.list_state.select(Some(safe_relative));
        f.render_stateful_widget(list, chunks[2], &mut app.list_state);
    }

    // Footer
    let footer_text = Line::from(vec![
        Span::styled("↑/↓", Style::default().fg(Color::Cyan)),
        Span::raw(" Navigate | "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" View | "),
        Span::styled("ESC/q", Style::default().fg(Color::Cyan)),
        Span::raw(" Quit | "),
        Span::raw(format!("Indexed: {} files", app.document_count)),
    ]);
    let footer_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(Paragraph::new(footer_text).block(footer_block), chunks[3]);
}

fn render_file_details(f: &mut Frame, result: &Option<bitfunnel::SearchResult>) {
    let size = f.size();
    
    if let Some(result) = result {
        let chunks = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
            .split(size);

        // Header
        let header_text = Line::from(vec![
            Span::styled(" File: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(result.document.path.clone()),
            Span::styled(
                format!(" | Score: {:.1}%", result.score),
                Style::default().fg(Color::Green),
            ),
        ]);
        let header_block = Block::default()
            .borders(Borders::ALL)
            .title(" File Details ");
        f.render_widget(Paragraph::new(header_text).block(header_block), chunks[0]);

        // Content
        let content_block = Block::default()
            .borders(Borders::ALL)
            .title(" Content ");
        let content = Paragraph::new(result.document.content.as_str())
            .block(content_block)
            .wrap(Wrap { trim: true })
            .scroll((0, 0));
        f.render_widget(content, chunks[1]);

        // Footer
        let footer_text = Line::from(vec![
            Span::styled("ESC", Style::default().fg(Color::Cyan)),
            Span::raw(" Back"),
        ]);
        let footer_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Gray));
        f.render_widget(Paragraph::new(footer_text).block(footer_block), chunks[2]);
    }
}

fn get_preview(content: &str, query: &str, max_len: usize) -> String {
    let content_lower = content.to_lowercase();
    let query_lower = query.to_lowercase();
    
    // Try to find a snippet containing the query
    if let Some(pos) = content_lower.find(&query_lower) {
        let start = pos.saturating_sub(20);
        let end = (pos + query.len() + 40).min(content.len());
        let snippet = &content[start..end];
        
        if snippet.len() > max_len {
            format!("...{}...", &snippet[..max_len])
        } else {
            format!("...{}...", snippet)
        }
    } else {
        // Just take the beginning
        if content.len() > max_len {
            format!("{}...", &content[..max_len])
        } else {
            content.to_string()
        }
    }
}
