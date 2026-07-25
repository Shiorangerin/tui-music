use std::time::Duration;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::App;
use tui_music::player::RepeatMode;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(8),
            Constraint::Length(3),
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_list(f, app, chunks[1]);
    draw_viz(f, app, chunks[2]);
    draw_footer(f, app, chunks[3]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let title = Span::styled(
        " ♪ tui-music ",
        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
    );
    let dir = Span::raw(format!(" · {}", app.music_dir.display()));
    let line = Line::from(vec![title, dir]);
    let block = Block::default().borders(Borders::ALL).title(line);
    f.render_widget(block, area);
}

fn draw_list(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(" 播放列表 ({}) ", app.tracks.len()),
            Style::default().fg(Color::Cyan),
        ));
    let items: Vec<ListItem> = app
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let marker = if Some(i) == app.current { "▶ " } else { " " };
            let name = if Some(i) == app.current {
                t.display_name()
            } else {
                t.display_name()
            };
            let style = if Some(i) == app.current {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(Line::from(vec![
                Span::styled(marker.to_string(), Style::default().fg(Color::Green)),
                Span::styled(name, style),
                Span::raw("  "),
                Span::styled(t.subtitle(), Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();
    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::REVERSED),
    );
    let mut state = ListState::default();
    state.select(app.selected);
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_viz(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" 可视化 ", Style::default().fg(Color::Green)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let bars = app.bars;
    if bars.iter().all(|&v| v == 0) {
        return;
    }

    let cols = bars.len();
    let cell_w = (inner.width as usize / cols.max(1)).max(1);
    for i in 0..cols {
        let col = bars[i];
        let x = inner.x + (i * cell_w) as u16;
        let h = (col as u16).min(inner.height);
        for y in 0..h {
            let py = inner.bottom() - 1 - y;
            if py < inner.y {
                break;
            }
            let (ch, color) = if y < inner.height / 3 {
                ("▁", Color::Green)
            } else if y < (inner.height * 2) / 3 {
                ("▃", Color::Yellow)
            } else {
                ("█", Color::Red)
            };
            f.buffer_mut()
                .set_string(x, py, ch, Style::default().fg(color));
        }
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let rep = match app.repeat {
        RepeatMode::Off => "关",
        RepeatMode::One => "🔂",
        RepeatMode::All => "🔁",
    };
    let st = if app.player.playing { "▶" } else { "⏸" };
    let pos = fmt_dur(app.player.position);
    let pct = if let Some(idx) = app.current {
        let d = app.tracks[idx].duration;
        if d > 0.0 { app.player.position.as_secs_f64() / d } else { 0.0 }
    } else { 0.0 };
    let pct = pct.clamp(0.0, 1.0);
    let info = Span::raw(format!(" {} {}  [{}]  🔊{}  {}", st, rep, pos, (app.volume*100.0) as i32, now_playing(app)));
    let bar_w = area.width.saturating_sub(2) as usize;
    let filled = (pct * bar_w as f64) as usize;
    let progress = format!("[{}{}]", "─".repeat(filled), "·".repeat(bar_w - filled));
    let line = Line::from(vec![info, Span::raw("  "), Span::raw(progress)]);
    let block = Block::default().borders(Borders::ALL);
    let p = Paragraph::new(line).block(block);
    f.render_widget(p, area);
}

fn now_playing(app: &App) -> String {
    match app.current {
        Some(i) => format!("{}", app.tracks[i].display_name()),
        None => "无".to_string(),
    }
}

fn fmt_dur(d: Duration) -> String {
    let s = d.as_secs();
    format!("{}:{:02}", s / 60, s % 60)
}