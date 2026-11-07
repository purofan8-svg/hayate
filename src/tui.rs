//! Интерактивный поиск: печатаешь — фильтруется на лету, Tab — тип файлов.

use std::sync::Arc;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::index::{Filter, Index, Kind};
use crate::lang;

const ACCENT: Color = Color::Rgb(120, 190, 255);
const DIM: Color = Color::Rgb(120, 126, 138);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    All,
    Dirs,
    Files,
    Kind(Kind),
}

impl Mode {
    fn label(self) -> &'static str {
        match self {
            Mode::All => "всё",
            Mode::Dirs => "папки",
            Mode::Files => "файлы",
            Mode::Kind(k) => k.name(),
        }
    }

    fn next(self) -> Mode {
        match self {
            Mode::All => Mode::Dirs,
            Mode::Dirs => Mode::Files,
            Mode::Files => Mode::Kind(Kind::ALL[0]),
            Mode::Kind(k) => {
                let i = Kind::ALL.iter().position(|&x| x == k).unwrap_or(0);
                match Kind::ALL.get(i + 1) {
                    Some(&n) => Mode::Kind(n),
                    None => Mode::All,
                }
            }
        }
    }

    fn filter(self) -> Filter {
        match self {
            Mode::All => Filter::default(),
            Mode::Dirs => Filter { dirs_only: true, ..Default::default() },
            Mode::Files => Filter { files_only: true, ..Default::default() },
            Mode::Kind(k) => Filter { files_only: true, kinds: Some(k.bit()), ..Default::default() },
        }
    }
}

struct App {
    ix: Arc<Index>,
    query: String,
    mode: Mode,
    results: Vec<usize>,
    sel: usize,
    took_ms: f64,
    total: usize,
}

impl App {
    fn new(ix: Arc<Index>) -> App {
        let total = ix.len();
        App { ix, query: String::new(), mode: Mode::All, results: Vec::new(), sel: 0, took_ms: 0.0, total }
    }

    fn refresh(&mut self) {
        let r = self.ix.search(&self.query, 400, &self.mode.filter());
        self.results = r.hits;
        self.took_ms = r.took.as_secs_f64() * 1000.0;
        if self.sel >= self.results.len() {
            self.sel = 0;
        }
    }

    fn open_selected(&self) {
        if let Some(&i) = self.results.get(self.sel) {
            let p = self.ix.path_at(i).to_string();
            open_path(&p);
        }
    }
}

pub fn run(ix: Arc<Index>) -> std::io::Result<()> {
    let mut term = ratatui::init();
    let mut app = App::new(ix);
    let res = event_loop(&mut term, &mut app);
    ratatui::restore();
    res
}

type Term = ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>;

fn event_loop(term: &mut Term, app: &mut App) -> std::io::Result<()> {
    loop {
        term.draw(|f| draw(f, app))?;
        let Event::Key(k) = event::read()? else { continue };
        if k.kind != KeyEventKind::Press {
            continue;
        }
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Esc => return Ok(()),
            KeyCode::Char('c') if ctrl => return Ok(()),
            KeyCode::Char('u') if ctrl => {
                app.query.clear();
                app.refresh();
            }
            KeyCode::Tab => {
                app.mode = app.mode.next();
                app.refresh();
            }
            KeyCode::Char(ch) => {
                app.query.push(ch);
                app.refresh();
            }
            KeyCode::Backspace => {
                app.query.pop();
                app.refresh();
            }
            KeyCode::Down => {
                if app.sel + 1 < app.results.len() {
                    app.sel += 1;
                }
            }
            KeyCode::Up => {
                app.sel = app.sel.saturating_sub(1);
            }
            KeyCode::PageDown => app.sel = (app.sel + 15).min(app.results.len().saturating_sub(1)),
            KeyCode::PageUp => app.sel = app.sel.saturating_sub(15),
            KeyCode::Enter => app.open_selected(),
            _ => {}
        }
    }
}

fn tag_color(k: Kind) -> Color {
    match k {
        Kind::Image => Color::Rgb(190, 150, 255),
        Kind::Video => Color::Rgb(255, 120, 140),
        Kind::Audio => Color::Rgb(120, 220, 170),
        Kind::Doc => Color::Rgb(235, 200, 120),
        Kind::Text => Color::Rgb(170, 190, 200),
        Kind::Archive => Color::Rgb(230, 160, 90),
        Kind::Code => Color::Rgb(130, 200, 255),
        Kind::Exec => Color::Rgb(255, 160, 90),
        Kind::Font => Color::Rgb(200, 180, 255),
        Kind::Model => Color::Rgb(140, 230, 210),
        Kind::Other => DIM,
    }
}

fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(1), Constraint::Length(1)]).split(f.area());

    let input = Paragraph::new(Line::from(vec![
        Span::styled("› ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::raw(app.query.clone()),
        Span::styled("▏", Style::default().fg(ACCENT)),
    ]))
    .block(Block::bordered().border_style(Style::default().fg(DIM)).title(Span::styled(
        format!(" [{}] ", app.mode.label()),
        Style::default().fg(ACCENT),
    )));
    f.render_widget(input, chunks[0]);

    let items: Vec<ListItem> = app
        .results
        .iter()
        .map(|&i| {
            let is_dir = app.ix.is_dir(i);
            let (tag, color) = if is_dir { (lang::t("folder"), ACCENT) } else { (app.ix.kind_at(i).name(), tag_color(app.ix.kind_at(i))) };
            let path = app.ix.path_at(i);
            let size = if is_dir { "—".to_string() } else { human(app.ix.size_at(i)) };
            let line = Line::from(vec![
                Span::styled(format!("{:<11}", tag), Style::default().fg(color)),
                Span::styled(path.to_string(), Style::default()),
                Span::styled(format!("  {size}"), Style::default().fg(DIM)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(if app.results.is_empty() { None } else { Some(app.sel) });
    let list = List::new(items)
        .block(Block::bordered().border_style(Style::default().fg(DIM)))
        .highlight_style(Style::default().bg(Color::Rgb(40, 52, 70)).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(list, chunks[1], &mut state);

    let status = format!(
        " {} {} {} · {} · {} ",
        app.results.len(),
        lang::t("of"),
        lang::fmt_num(app.total),
        lang::ms(app.took_ms),
        lang::t("Tab — type · ↑↓ — select · Enter — open · Esc — exit")
    );
    f.render_widget(Paragraph::new(status).style(Style::default().fg(DIM)), chunks[2]);
}

fn human(n: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut x = n as f64;
    let mut i = 0;
    while x >= 1024.0 && i < 4 {
        x /= 1024.0;
        i += 1;
    }
    if i == 0 { format!("{n} B") } else { format!("{x:.1} {}", U[i]) }
}

#[cfg(windows)]
fn open_path(p: &str) {
    let _ = std::process::Command::new("cmd").args(["/C", "start", "", p]).spawn();
}

#[cfg(not(windows))]
fn open_path(p: &str) {
    let _ = std::process::Command::new("xdg-open").arg(p).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn draws_without_panic() {
        let mut ix = Index::default();
        ix.push("C:/pics/a.png", 10, 0, (Kind::Image as u8) << 2);
        let mut app = App::new(Arc::new(ix));
        app.query = "a".into();
        app.refresh();
        let mut t = Terminal::new(TestBackend::new(90, 20)).unwrap();
        t.draw(|f| draw(f, &app)).unwrap();
        let text: String = t.backend().buffer().content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("a.png"), "список не отрисован: {text}");
    }

    #[test]
    fn tab_cycles_filters() {
        let mut m = Mode::All;
        let mut seen = vec![];
        for _ in 0..14 {
            seen.push(m.label());
            m = m.next();
        }
        assert!(seen.contains(&"папки"));
        assert!(seen.contains(&"картинки"));
        assert_eq!(m, Mode::All, "цикл должен вернуться в начало");
    }
}