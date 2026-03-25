use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use domain_scan_core::ir::{EntityKind, ScanIndex};
use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{Frame, Terminal};

// ---------------------------------------------------------------------------
// Tree node model
// ---------------------------------------------------------------------------

/// A single node in the TUI tree. Parent nodes have children; leaf nodes do not.
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Display label for this node.
    pub label: String,
    /// Entity kind (for coloring / icons).
    pub kind: EntityKind,
    /// Depth in the tree (0 = top-level parent).
    pub depth: u16,
    /// Number of children (methods/properties). 0 for leaf nodes.
    pub child_count: usize,
    /// Whether this parent is currently expanded.
    pub expanded: bool,
    /// If this is a child node, index of its parent in the flat vec. None for parents.
    pub parent_idx: Option<usize>,
    /// File location (file:line).
    pub file_loc: String,
}

// ---------------------------------------------------------------------------
// TuiApp
// ---------------------------------------------------------------------------

/// The main TUI application state. Designed to be testable without a real terminal.
pub struct TuiApp {
    /// Flat list of all tree nodes (parents + children interleaved).
    pub nodes: Vec<TreeNode>,
    /// Currently selected index into the visible nodes list.
    pub selected: usize,
    /// Whether the app should quit.
    pub should_quit: bool,
    /// Search query (when `/` is pressed).
    pub search_query: Option<String>,
    /// Whether we're in search input mode.
    pub search_mode: bool,
    /// Title for the TUI header.
    pub title: String,
}

impl TuiApp {
    /// Build a TUI tree from a ScanIndex showing interfaces with their methods.
    pub fn from_interfaces(index: &ScanIndex) -> Self {
        let mut nodes = Vec::new();
        let interfaces = index.get_interfaces(None);

        for iface in &interfaces {
            let parent_idx = nodes.len();
            nodes.push(TreeNode {
                label: format!("{} ({} methods)", iface.name, iface.methods.len()),
                kind: EntityKind::Interface,
                depth: 0,
                child_count: iface.methods.len(),
                expanded: false,
                parent_idx: None,
                file_loc: format!("{}:{}", iface.file.display(), iface.span.start_line),
            });

            for method in &iface.methods {
                let async_str = if method.is_async { "async " } else { "" };
                let ret = method.return_type.as_deref().unwrap_or("void");
                nodes.push(TreeNode {
                    label: format!("{async_str}{}(...) -> {ret}", method.name),
                    kind: EntityKind::Method,
                    depth: 1,
                    child_count: 0,
                    expanded: false,
                    parent_idx: Some(parent_idx),
                    file_loc: String::new(),
                });
            }
        }

        Self {
            nodes,
            selected: 0,
            should_quit: false,
            search_query: None,
            search_mode: false,
            title: "Interfaces".to_string(),
        }
    }

    /// Build a TUI tree from a ScanIndex showing services with their methods/routes.
    pub fn from_services(index: &ScanIndex) -> Self {
        let mut nodes = Vec::new();
        let services = index.get_services(None);

        for svc in &services {
            let parent_idx = nodes.len();
            nodes.push(TreeNode {
                label: format!(
                    "{} [{:?}] ({} methods, {} routes)",
                    svc.name,
                    svc.kind,
                    svc.methods.len(),
                    svc.routes.len(),
                ),
                kind: EntityKind::Service,
                depth: 0,
                child_count: svc.methods.len() + svc.routes.len(),
                expanded: false,
                parent_idx: None,
                file_loc: format!("{}:{}", svc.file.display(), svc.span.start_line),
            });

            for method in &svc.methods {
                let async_str = if method.is_async { "async " } else { "" };
                let ret = method.return_type.as_deref().unwrap_or("void");
                nodes.push(TreeNode {
                    label: format!("{async_str}{}(...) -> {ret}", method.name),
                    kind: EntityKind::Method,
                    depth: 1,
                    child_count: 0,
                    expanded: false,
                    parent_idx: Some(parent_idx),
                    file_loc: String::new(),
                });
            }

            for route in &svc.routes {
                nodes.push(TreeNode {
                    label: format!("{:?} {} -> {}", route.method, route.path, route.handler),
                    kind: EntityKind::Method,
                    depth: 1,
                    child_count: 0,
                    expanded: false,
                    parent_idx: Some(parent_idx),
                    file_loc: String::new(),
                });
            }
        }

        Self {
            nodes,
            selected: 0,
            should_quit: false,
            search_query: None,
            search_mode: false,
            title: "Services".to_string(),
        }
    }

    /// Build a TUI tree from a ScanIndex showing schemas with their fields.
    pub fn from_schemas(index: &ScanIndex) -> Self {
        let mut nodes = Vec::new();
        let schemas = index.get_schemas(None);

        for schema in &schemas {
            let parent_idx = nodes.len();
            nodes.push(TreeNode {
                label: format!(
                    "{} [{}] ({} fields)",
                    schema.name,
                    schema.source_framework,
                    schema.fields.len(),
                ),
                kind: EntityKind::Schema,
                depth: 0,
                child_count: schema.fields.len(),
                expanded: false,
                parent_idx: None,
                file_loc: format!("{}:{}", schema.file.display(), schema.span.start_line),
            });

            for field in &schema.fields {
                let ty = field.type_annotation.as_deref().unwrap_or("?");
                let opt = if field.is_optional { "?" } else { "" };
                nodes.push(TreeNode {
                    label: format!("{}{opt}: {ty}", field.name),
                    kind: EntityKind::Schema,
                    depth: 1,
                    child_count: 0,
                    expanded: false,
                    parent_idx: Some(parent_idx),
                    file_loc: String::new(),
                });
            }
        }

        Self {
            nodes,
            selected: 0,
            should_quit: false,
            search_query: None,
            search_mode: false,
            title: "Schemas".to_string(),
        }
    }

    /// Build a generic entity list (for scan/search/stats etc.) — flat, no children.
    pub fn from_entity_list(index: &ScanIndex, title: &str) -> Self {
        let mut nodes = Vec::new();

        for file in &index.files {
            for iface in &file.interfaces {
                nodes.push(TreeNode {
                    label: format!("interface: {}", iface.name),
                    kind: EntityKind::Interface,
                    depth: 0,
                    child_count: iface.methods.len(),
                    expanded: false,
                    parent_idx: None,
                    file_loc: format!("{}:{}", iface.file.display(), iface.span.start_line),
                });
            }
            for svc in &file.services {
                nodes.push(TreeNode {
                    label: format!("service: {}", svc.name),
                    kind: EntityKind::Service,
                    depth: 0,
                    child_count: svc.methods.len(),
                    expanded: false,
                    parent_idx: None,
                    file_loc: format!("{}:{}", svc.file.display(), svc.span.start_line),
                });
            }
            for class in &file.classes {
                nodes.push(TreeNode {
                    label: format!("class: {}", class.name),
                    kind: EntityKind::Class,
                    depth: 0,
                    child_count: class.methods.len(),
                    expanded: false,
                    parent_idx: None,
                    file_loc: format!("{}:{}", class.file.display(), class.span.start_line),
                });
            }
            for schema in &file.schemas {
                nodes.push(TreeNode {
                    label: format!("schema: {}", schema.name),
                    kind: EntityKind::Schema,
                    depth: 0,
                    child_count: schema.fields.len(),
                    expanded: false,
                    parent_idx: None,
                    file_loc: format!("{}:{}", schema.file.display(), schema.span.start_line),
                });
            }
        }

        Self {
            nodes,
            selected: 0,
            should_quit: false,
            search_query: None,
            search_mode: false,
            title: title.to_string(),
        }
    }

    /// Get the list of visible node indices (skipping children of collapsed parents).
    pub fn visible_indices(&self) -> Vec<usize> {
        let mut visible = Vec::new();
        for (i, node) in self.nodes.iter().enumerate() {
            if let Some(parent_idx) = node.parent_idx {
                // This is a child node — only visible if parent is expanded.
                if self.nodes[parent_idx].expanded {
                    visible.push(i);
                }
            } else {
                // Top-level parent — always visible.
                visible.push(i);
            }
        }
        visible
    }

    /// Handle a key event. Returns true if the event was handled.
    pub fn handle_event(&mut self, key: KeyEvent) -> bool {
        if self.search_mode {
            return self.handle_search_key(key);
        }

        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                true
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                true
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
                true
            }
            KeyCode::Enter => {
                self.toggle_selected();
                true
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.expand_selected();
                true
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.collapse_selected();
                true
            }
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.search_query = Some(String::new());
                true
            }
            KeyCode::Esc => {
                self.search_query = None;
                self.search_mode = false;
                true
            }
            _ => false,
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.search_mode = false;
                true
            }
            KeyCode::Backspace => {
                if let Some(ref mut q) = self.search_query {
                    q.pop();
                }
                self.jump_to_search_match();
                true
            }
            KeyCode::Char(c) => {
                if let Some(ref mut q) = self.search_query {
                    q.push(c);
                }
                self.jump_to_search_match();
                true
            }
            _ => false,
        }
    }

    fn jump_to_search_match(&mut self) {
        let query = match self.search_query {
            Some(ref q) if !q.is_empty() => q.to_lowercase(),
            _ => return,
        };

        let visible = self.visible_indices();
        for (vi, &ni) in visible.iter().enumerate() {
            if self.nodes[ni].label.to_lowercase().contains(&query) {
                self.selected = vi;
                return;
            }
        }
    }

    fn move_selection(&mut self, delta: i32) {
        let visible = self.visible_indices();
        if visible.is_empty() {
            return;
        }
        let max = visible.len().saturating_sub(1);
        if delta < 0 {
            self.selected = self.selected.saturating_sub(delta.unsigned_abs() as usize);
        } else {
            self.selected = (self.selected + delta as usize).min(max);
        }
    }

    fn toggle_selected(&mut self) {
        let visible = self.visible_indices();
        if let Some(&node_idx) = visible.get(self.selected) {
            let node = &self.nodes[node_idx];
            if node.parent_idx.is_none() && node.child_count > 0 {
                // Toggle parent expand/collapse
                self.nodes[node_idx].expanded = !self.nodes[node_idx].expanded;
            }
        }
    }

    fn expand_selected(&mut self) {
        let visible = self.visible_indices();
        if let Some(&node_idx) = visible.get(self.selected) {
            let node = &self.nodes[node_idx];
            if node.parent_idx.is_none() && node.child_count > 0 {
                self.nodes[node_idx].expanded = true;
            }
        }
    }

    fn collapse_selected(&mut self) {
        let visible = self.visible_indices();
        if let Some(&node_idx) = visible.get(self.selected) {
            let node = &self.nodes[node_idx];
            if node.parent_idx.is_none() {
                // Collapse this parent
                self.nodes[node_idx].expanded = false;
            } else if let Some(parent_idx) = node.parent_idx {
                // On a child — collapse the parent and move selection to it
                self.nodes[parent_idx].expanded = false;
                // Find the parent in visible indices and select it
                let new_visible = self.visible_indices();
                for (vi, &ni) in new_visible.iter().enumerate() {
                    if ni == parent_idx {
                        self.selected = vi;
                        break;
                    }
                }
            }
        }
    }

    /// Render the TUI into the given frame.
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(1),    // Tree
                Constraint::Length(3), // Footer / search bar
            ])
            .split(area);

        self.render_header(frame, chunks[0]);
        self.render_tree(frame, chunks[1]);
        self.render_footer(frame, chunks[2]);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let visible_count = self.visible_indices().len();
        let total_parents = self.nodes.iter().filter(|n| n.parent_idx.is_none()).count();
        let header_text = format!(
            " {} — {} entities ({} visible)",
            self.title, total_parents, visible_count
        );
        let header = Paragraph::new(header_text)
            .block(Block::default().borders(Borders::ALL).title("domain-scan"));
        frame.render_widget(header, area);
    }

    fn render_tree(&self, frame: &mut Frame, area: Rect) {
        let visible = self.visible_indices();

        let items: Vec<ListItem> = visible
            .iter()
            .enumerate()
            .map(|(vi, &ni)| {
                let node = &self.nodes[ni];
                let is_selected = vi == self.selected;

                let indent = "  ".repeat(node.depth as usize);
                let prefix = if node.parent_idx.is_some() {
                    format!("{indent}  ")
                } else if node.child_count > 0 {
                    if node.expanded {
                        format!("{indent}v ")
                    } else {
                        format!("{indent}> ")
                    }
                } else {
                    format!("{indent}  ")
                };

                let kind_color = kind_color(node.kind);

                let line = if is_selected {
                    Line::from(vec![Span::styled(
                        format!("{prefix}{}", node.label),
                        Style::default()
                            .fg(kind_color)
                            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
                    )])
                } else {
                    Line::from(vec![
                        Span::styled(prefix, Style::default().fg(Color::DarkGray)),
                        Span::styled(&node.label, Style::default().fg(kind_color)),
                    ])
                };

                ListItem::new(line)
            })
            .collect();

        let tree = List::new(items).block(Block::default().borders(Borders::ALL));
        frame.render_widget(tree, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let text = if self.search_mode {
            let q = self.search_query.as_deref().unwrap_or("");
            format!(" /{q}")
        } else {
            let visible = self.visible_indices();
            let loc = visible
                .get(self.selected)
                .map(|&ni| self.nodes[ni].file_loc.as_str())
                .unwrap_or("");
            if loc.is_empty() {
                " [↑↓] navigate  [Enter] expand/collapse  [→] expand  [←] collapse  [/] search  [q] quit".to_string()
            } else {
                format!(" {loc}  |  [↑↓] navigate  [Enter] toggle  [/] search  [q] quit")
            }
        };
        let footer = Paragraph::new(text).block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, area);
    }

    /// Run the TUI event loop with a real terminal.
    pub fn run<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        B::Error: 'static,
    {
        loop {
            terminal.draw(|f| self.render(f))?;

            if self.should_quit {
                break;
            }

            if let Event::Key(key) = event::read()? {
                self.handle_event(key);
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn kind_color(kind: EntityKind) -> Color {
    match kind {
        EntityKind::Interface => Color::Cyan,
        EntityKind::Service => Color::Green,
        EntityKind::Class => Color::Yellow,
        EntityKind::Schema => Color::Magenta,
        EntityKind::Method => Color::White,
        EntityKind::Function => Color::Blue,
        EntityKind::Impl => Color::LightRed,
        EntityKind::TypeAlias => Color::LightCyan,
    }
}

// ---------------------------------------------------------------------------
// Terminal setup/teardown for real usage
// ---------------------------------------------------------------------------

pub fn setup_terminal(
) -> Result<Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>, Box<dyn std::error::Error>>
{
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn teardown_terminal(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Helper to build a minimal ScanIndex with test interfaces.
    fn test_scan_index() -> ScanIndex {
        use domain_scan_core::index::build_index;
        use domain_scan_core::ir::{
            BuildStatus, Confidence, InterfaceDef, InterfaceKind, IrFile, Language,
            MethodSignature, Span, Visibility,
        };
        use std::path::PathBuf;

        let span = |start: u32, end: u32| Span {
            start_line: start,
            end_line: end,
            start_col: 0,
            end_col: 0,
            byte_range: (0, 0),
        };

        let ir_file = IrFile {
            path: PathBuf::from("test.ts"),
            language: Language::TypeScript,
            content_hash: String::new(),
            build_status: BuildStatus::Built,
            confidence: Confidence::High,
            interfaces: vec![
                InterfaceDef {
                    name: "UserRepository".to_string(),
                    file: PathBuf::from("test.ts"),
                    span: span(1, 10),
                    visibility: Visibility::Public,
                    generics: vec![],
                    extends: vec![],
                    methods: vec![
                        MethodSignature {
                            name: "findById".to_string(),
                            span: span(2, 2),
                            is_async: true,
                            parameters: vec![],
                            return_type: Some("Promise<User>".to_string()),
                            has_default: false,
                        },
                        MethodSignature {
                            name: "findAll".to_string(),
                            span: span(3, 3),
                            is_async: true,
                            parameters: vec![],
                            return_type: Some("Promise<User[]>".to_string()),
                            has_default: false,
                        },
                    ],
                    properties: vec![],
                    language_kind: InterfaceKind::Interface,
                    decorators: vec![],
                },
                InterfaceDef {
                    name: "EventHandler".to_string(),
                    file: PathBuf::from("test.ts"),
                    span: span(12, 20),
                    visibility: Visibility::Public,
                    generics: vec![],
                    extends: vec![],
                    methods: vec![MethodSignature {
                        name: "handle".to_string(),
                        span: span(13, 13),
                        is_async: false,
                        parameters: vec![],
                        return_type: Some("void".to_string()),
                        has_default: false,
                    }],
                    properties: vec![],
                    language_kind: InterfaceKind::Interface,
                    decorators: vec![],
                },
            ],
            services: vec![],
            classes: vec![],
            functions: vec![],
            schemas: vec![],
            implementations: vec![],
            type_aliases: vec![],
            imports: vec![],
            exports: vec![],
        };

        build_index(PathBuf::from("."), vec![ir_file], 0, 0, 0)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_initial_state_all_collapsed() {
        let index = test_scan_index();
        let app = TuiApp::from_interfaces(&index);

        // Should have 2 parents + 3 children = 5 total nodes
        assert_eq!(app.nodes.len(), 5);

        // Only 2 visible (parents collapsed)
        let visible = app.visible_indices();
        assert_eq!(visible.len(), 2);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_enter_expands_parent() {
        let index = test_scan_index();
        let mut app = TuiApp::from_interfaces(&index);

        // Initially 2 visible
        assert_eq!(app.visible_indices().len(), 2);

        // Press Enter on first parent -> expands
        app.handle_event(key(KeyCode::Enter));
        assert!(app.nodes[0].expanded);

        // Now 4 visible: UserRepository + 2 methods + EventHandler
        assert_eq!(app.visible_indices().len(), 4);
    }

    #[test]
    fn test_second_enter_collapses_parent() {
        let index = test_scan_index();
        let mut app = TuiApp::from_interfaces(&index);

        // Expand
        app.handle_event(key(KeyCode::Enter));
        assert!(app.nodes[0].expanded);
        assert_eq!(app.visible_indices().len(), 4);

        // Collapse
        app.handle_event(key(KeyCode::Enter));
        assert!(!app.nodes[0].expanded);
        assert_eq!(app.visible_indices().len(), 2);
    }

    #[test]
    fn test_arrow_navigation() {
        let index = test_scan_index();
        let mut app = TuiApp::from_interfaces(&index);

        assert_eq!(app.selected, 0);

        // Down
        app.handle_event(key(KeyCode::Down));
        assert_eq!(app.selected, 1);

        // Up
        app.handle_event(key(KeyCode::Up));
        assert_eq!(app.selected, 0);

        // Up at top stays at 0
        app.handle_event(key(KeyCode::Up));
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_right_expands_left_collapses() {
        let index = test_scan_index();
        let mut app = TuiApp::from_interfaces(&index);

        // Right expands
        app.handle_event(key(KeyCode::Right));
        assert!(app.nodes[0].expanded);

        // Left collapses
        app.handle_event(key(KeyCode::Left));
        assert!(!app.nodes[0].expanded);
    }

    #[test]
    fn test_left_on_child_collapses_parent_and_moves_selection() {
        let index = test_scan_index();
        let mut app = TuiApp::from_interfaces(&index);

        // Expand first parent
        app.handle_event(key(KeyCode::Right));
        assert!(app.nodes[0].expanded);

        // Move down to first child
        app.handle_event(key(KeyCode::Down));
        assert_eq!(app.selected, 1);

        // Left on child -> collapses parent, selection moves to parent
        app.handle_event(key(KeyCode::Left));
        assert!(!app.nodes[0].expanded);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_q_quits() {
        let index = test_scan_index();
        let mut app = TuiApp::from_interfaces(&index);

        assert!(!app.should_quit);
        app.handle_event(key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_render_does_not_panic() {
        let index = test_scan_index();
        let app = TuiApp::from_interfaces(&index);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| app.render(f)).unwrap();
    }

    #[test]
    fn test_render_expanded_shows_children() {
        let index = test_scan_index();
        let mut app = TuiApp::from_interfaces(&index);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Expand
        app.handle_event(key(KeyCode::Enter));

        terminal.draw(|f| app.render(f)).unwrap();

        // Verify the buffer contains method names
        let buf = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect::<String>();
        assert!(
            buf.contains("findById"),
            "Buffer should contain method name 'findById'"
        );
        assert!(
            buf.contains("findAll"),
            "Buffer should contain method name 'findAll'"
        );
    }

    #[test]
    fn test_search_mode() {
        let index = test_scan_index();
        let mut app = TuiApp::from_interfaces(&index);

        // Enter search mode
        app.handle_event(key(KeyCode::Char('/')));
        assert!(app.search_mode);

        // Type 'E' to search for EventHandler
        app.handle_event(key(KeyCode::Char('E')));
        app.handle_event(key(KeyCode::Char('v')));

        // Should jump to EventHandler (index 1 in visible)
        assert_eq!(app.selected, 1);

        // Exit search mode
        app.handle_event(key(KeyCode::Esc));
        assert!(!app.search_mode);
    }

    #[test]
    fn test_navigation_with_expanded_children() {
        let index = test_scan_index();
        let mut app = TuiApp::from_interfaces(&index);

        // Expand first parent
        app.handle_event(key(KeyCode::Enter));
        // visible: [UserRepository, findById, findAll, EventHandler]

        // Navigate down through all visible items
        app.handle_event(key(KeyCode::Down)); // -> findById
        assert_eq!(app.selected, 1);

        app.handle_event(key(KeyCode::Down)); // -> findAll
        assert_eq!(app.selected, 2);

        app.handle_event(key(KeyCode::Down)); // -> EventHandler
        assert_eq!(app.selected, 3);

        // Down at bottom stays at bottom
        app.handle_event(key(KeyCode::Down));
        assert_eq!(app.selected, 3);
    }

    #[test]
    fn test_empty_scan_index() {
        let index =
            domain_scan_core::index::build_index(std::path::PathBuf::from("."), vec![], 0, 0, 0);
        let app = TuiApp::from_interfaces(&index);

        assert!(app.nodes.is_empty());
        assert_eq!(app.visible_indices().len(), 0);

        // Navigation on empty list should not panic
        let mut app = app;
        app.handle_event(key(KeyCode::Down));
        app.handle_event(key(KeyCode::Enter));
    }
}
