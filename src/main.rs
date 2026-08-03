mod app;
mod ui;

use std::io;
use std::io::Stdout;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

#[derive(Parser)]
#[command(name = "tui-music", version, about = "A terminal music player")]
struct Args {
    /// Path to music directory (default: ~/Music)
    #[arg(short, long)]
    music: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let music_dir = args
        .music
        .unwrap_or_else(tui_music::library::default_music_dir);

    let mut app = app::App::new(music_dir)?;
    if !app.playlists.is_empty() && !app.active_tracks().is_empty() {
        app.selected = Some(0);
    }

    with_terminal(|terminal| run(terminal, &mut app))
}

/// Enter raw mode + alternate screen for the duration of `f`, and always
/// restore the terminal afterwards — even if `f` errors or the app panics.
fn with_terminal<F>(f: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()>,
{
    enable_raw_mode()?;
    let result = (|| {
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        let r = f(&mut terminal);
        let _ = terminal.show_cursor();
        let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
        r
    })();
    let _ = disable_raw_mode();
    result
}

fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut app::App,
) -> anyhow::Result<()> {
    let tick = Duration::from_millis(33);
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(tick)? {
            while event::poll(Duration::ZERO).unwrap_or(false) {
                if let Event::Key(k) = event::read()? {
                    if k.kind != KeyEventKind::Release {
                        app.handle_key(k);
                    }
                }
            }
        }

        app.update(terminal.size()?.width.saturating_sub(2).max(16) as usize);

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
