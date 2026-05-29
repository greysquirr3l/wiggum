# T09 — Screens and Widgets

> **Depends on**: T08 complete.

## Goal

Implement all visual components: sidebar (teams/channels tree), message list, status bar, help overlay, loading screen, and login screen.

## Project Context

- TUI framework: `ratatui 0.30`
- All rendering is synchronous — pull state from `AppState`, output to `Frame`
- No I/O in render functions
- Layout: left sidebar (~30%), right message list (~70%), bottom status bar (1 line)
- Full architecture in `ARCHITECTURE.md § 9`

## Files to Create

### `crates/yakko-tui/src/ui/widgets/mod.rs`

```rust
pub mod help;
pub mod loading;
pub mod login;
pub mod message_list;
pub mod sidebar;
pub mod status_bar;
```

### `crates/yakko-tui/src/ui/widgets/sidebar.rs`

```rust
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use crate::app::state::SidebarState;

pub fn render(frame: &mut Frame, area: Rect, state: &SidebarState) {
    let items: Vec<ListItem> = state.conversations
        .iter()
        .map(|c| {
            let unread_style = if c.unread_count > 0 {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            let label = if c.unread_count > 0 {
                format!("{} ({})", c.display_name, c.unread_count)
            } else {
                c.display_name.clone()
            };
            ListItem::new(Line::from(Span::styled(label, unread_style)))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_index));

    let list = List::new(items)
        .block(Block::default().title("Conversations").borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut list_state);
}
```

### `crates/yakko-tui/src/ui/widgets/message_list.rs`

```rust
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use crate::app::state::MessagesState;

pub fn render(frame: &mut Frame, area: Rect, state: &MessagesState) {
    let lines: Vec<Line> = state.messages
        .iter()
        .map(|m| {
            let time = m.timestamp.format("%H:%M").to_string();
            let content = match &m.content {
                yakko_domain::entities::message::MessageContent::Plain(s) => s.clone(),
                yakko_domain::entities::message::MessageContent::Html(s) => strip_html(s),
            };
            let name_style = if m.is_mine {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Green)
            };
            Line::from(vec![
                Span::styled(format!("[{time}] "), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}: ", m.sender_display_name), name_style),
                Span::raw(content),
            ])
        })
        .collect();

    let total_lines = lines.len();
    let visible_height = area.height.saturating_sub(2) as usize; // account for borders
    let scroll = state.scroll_offset.min(total_lines.saturating_sub(visible_height));

    let para = Paragraph::new(lines)
        .block(Block::default().title("Messages").borders(Borders::ALL))
        .scroll((scroll as u16, 0));

    frame.render_widget(para, area);
}

fn strip_html(html: &str) -> String {
    // Simple tag stripper — not a full HTML parser
    let mut result = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => result.push(c),
            _ => {}
        }
    }
    result
}
```

### `crates/yakko-tui/src/ui/widgets/status_bar.rs`

```rust
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use crate::app::state::{AppState, ConnectionStatus};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let connection = match state.connection_status {
        ConnectionStatus::Realtime => Span::styled(" ● LIVE ", Style::default().fg(Color::Green)),
        ConnectionStatus::Polling => Span::styled(" ◷ POLL ", Style::default().fg(Color::Yellow)),
        ConnectionStatus::Disconnected => Span::styled(" ✗ OFF  ", Style::default().fg(Color::Red)),
    };

    let status_text = state.status_message
        .as_deref()
        .unwrap_or("Ready");

    let line = Line::from(vec![
        connection,
        Span::raw(" | "),
        Span::raw(status_text),
        Span::raw("  [?] help  [q] quit"),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}
```

### `crates/yakko-tui/src/ui/widgets/help.rs`

```rust
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

const HELP_TEXT: &str = r#"
  Navigation      Compose
  ──────────────  ──────────────
  j/↓   down      i      enter compose mode
  k/↑   up        Esc    exit compose mode
  h/←   sidebar   Enter  send message
  l/→   messages  Alt+↵  new line
  Enter select
  r     refresh   Messages
  q     quit      g  top
  ?     help      G  bottom
"#;

pub fn render(frame: &mut Frame, area: Rect) {
    // Center the help popup
    let popup_area = centered_rect(60, 70, area);
    frame.render_widget(Clear, popup_area); // clear background
    let para = Paragraph::new(HELP_TEXT)
        .block(Block::default().title("Help [?]").borders(Borders::ALL).style(Style::default().fg(Color::White)))
        .alignment(Alignment::Left);
    frame.render_widget(para, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    use ratatui::layout::{Constraint, Direction, Layout};
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
```

### `crates/yakko-tui/src/ui/widgets/loading.rs` and `login.rs`

Loading screen: centered spinner or "Loading…" text.  
Login screen: display the `auth_url` from `AppScreen::Login` with instructions.

### `crates/yakko-tui/src/ui/render.rs` (replace stub)

```rust
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};
use crate::app::state::{AppScreen, AppState};
use super::widgets;

pub fn render(frame: &mut Frame, state: &AppState) {
    match &state.screen {
        AppScreen::Loading => widgets::loading::render(frame, frame.area()),
        AppScreen::Login { auth_url } => widgets::login::render(frame, frame.area(), auth_url),
        AppScreen::Main | AppScreen::Help => render_main(frame, state),
    }
}

fn render_main(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[0]);

    widgets::sidebar::render(frame, body[0], &state.sidebar);
    widgets::message_list::render(frame, body[1], &state.messages);
    widgets::status_bar::render(frame, chunks[1], state);

    if state.screen == crate::app::state::AppScreen::Help {
        widgets::help::render(frame, frame.area());
    }
}
```

## Preflight

```bash
cargo build -p yakko-tui && cargo clippy -p yakko-tui -- -D warnings
```

## Exit Criteria

- `cargo run -p yakko-tui` launches and shows the main layout (empty lists are fine since no data yet)
- Help overlay appears on `?` and dismisses again
- Status bar visible at bottom
- Zero clippy warnings

## After Completion

Update PROGRESS.md row for T09 to `[x]`.  
Commit: `feat(tui): implement sidebar, message list, status bar, and help overlay`
