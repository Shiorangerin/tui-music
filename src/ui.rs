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
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(8),
            Constraint::Length(4),
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_search(f, app, chunks[1]);
    draw_list(f, app, chunks[2]);
    draw_spectrum(f, app, chunks[3]);
    draw_footer(f, app, chunks[4]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let title = Span::styled(
        " tui-music ",
        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
    );
    let dir = Span::raw(format!("  dir: {}", app.music_dir.display()));
    let line = Line::from(vec![title, dir]);
    let block = Block::default().borders(Borders::ALL).title(line);
    f.render_widget(block, area);
}

fn draw_search(f: &mut Frame, app: &App, area: Rect) {
    let label = Span::styled(" search ", Style::default().fg(Color::Yellow));
    let query = Span::raw(format!("  {}", app.search));
    let cursor = if app.search_active {
        Span::styled("_", Style::default().add_modifier(Modifier::RAPID_BLINK))
    } else {
        Span::raw("")
    };
    let hits = Span::raw(format!("   [{} / {}]", app.display.len(), app.tracks.len()));
    let line = Line::from(vec![label, query, cursor, hits]);
    let block = Block::default().borders(Borders::ALL);
    let p = Paragraph::new(line).block(block);
    f.render_widget(p, area);
}

fn draw_list(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            format!(" playlist ({}) ", app.display.len()),
            Style::default().fg(Color::Cyan),
        ));
    let items: Vec<ListItem> = app
        .display
        .iter()
        .enumerate()
        .map(|(view_i, &orig)| {
            let t = &app.tracks[orig];
            let is_cur = Some(view_i) == app.current;
            let marker = if is_cur { ">" } else { " " };
            let name = t.display_name();
            let style = if is_cur {
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

fn draw_spectrum(f: &mut Frame, app: &App, area: Rect) {
    let inner = {
        let block = Block::default().borders(Borders::ALL);
        let r = block.inner(area);
        f.render_widget(block, area);
        r
    };

    let bars = app.bars;
    if bars.iter().all(|&v| v == 0) {
        return;
    }

    let cols = bars.len();
    let cell_w = (inner.width as usize / cols.max(1)).max(1);
    let max_bar = 20u16;
    let height = inner.height as f32;
    let bottom = inner.bottom();

    for i in 0..cols {
        let v = bars[i].min(max_bar) as f32 / max_bar as f32;
        let x = inner.x + (i * cell_w) as u16;
        let top_y = bottom - ((v * height).round() as u16).max(1);
        let ty = top_y.max(inner.y);
        let frac = (v * height) - (bottom - 1 - ty) as f32;
        let sub = if frac >= 0.875 {
            8
        } else if frac >= 0.75 {
            7
        } else if frac >= 0.625 {
            6
        } else if frac >= 0.5 {
            5
        } else if frac >= 0.375 {
            4
        } else if frac >= 0.25 {
            3
        } else if frac >= 0.125 {
            2
        } else {
            1
        };
        let ch = sub_block(sub);

        let freq = i as f32 / (cols - 1).max(1) as f32;
        let color = if freq < 0.33 {
            Color::Cyan
        } else if freq < 0.66 {
            Color::Magenta
        } else {
            Color::Yellow
        };

        if ty < bottom {
            f.buffer_mut()
                .set_string(x, ty, ch, Style::default().fg(color));
        }
    }

    let axis_y = (inner.y + inner.height / 2).max(inner.y).min(bottom - 1);
    for x in (inner.x..inner.x + inner.width).step_by(2) {
        let cell = &mut f.buffer_mut()[(x, axis_y)];
        if cell.symbol() == " " {
            cell.set_char('.');
            cell.set_style(Style::default().fg(Color::DarkGray));
        }
    }
}

fn sub_block(level: u8) -> &'static str {
    match level {
        1 => "▁",
        2 => "▂",
        3 => "▃",
        4 => "▄",
        5 => "▅",
        6 => "▆",
        7 => "▇",
        _ => "█",
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let rep = match app.repeat {
        RepeatMode::Off => "Off",
        RepeatMode::One => "One",
        RepeatMode::All => "List",
    };
    let st = if app.player.playing { "Playing" } else { "Paused" };
    let shuf = if app.shuffle { "On" } else { "Off" };
    let pos = fmt_dur(app.player.position);

    let (cur_dur, name) = if let Some(v) = app.current {
        if let Some(o) = app.display.get(v) {
            let t = &app.tracks[*o];
            (t.duration, t.display_name())
        } else {
            (0.0, "None".to_string())
        }
    } else {
        (0.0, "None".to_string())
    };

    let pct = if cur_dur > 0.0 {
        app.player.position.as_secs_f64() / cur_dur
    } else {
        0.0
    };
    let pct = pct.clamp(0.0, 1.0);

    let vol_s = format!("   volume: {}%", (app.volume * 100.0) as i32);
    let pos_s = format!("   pos: {} /", pos);
    let dur_s = format!(" {}", fmt_secs(cur_dur));
    let now_s = format!("   now: {}", name);
    let status = Line::from(vec![
        Span::styled(" state ", Style::default().fg(Color::DarkGray)),
        Span::raw(": "),
        Span::styled(st.to_string(), Style::default().fg(Color::Green)),
        Span::raw("   repeat: "),
        Span::styled(rep.to_string(), Style::default().fg(Color::Cyan)),
        Span::raw("   shuffle: "),
        Span::styled(shuf.to_string(), Style::default().fg(Color::Cyan)),
        Span::raw(vol_s),
        Span::raw(pos_s),
        Span::raw(dur_s),
        Span::raw(now_s),
    ]);

    let bar_w = area.width.saturating_sub(2) as usize;
    let filled = (pct * bar_w as f64) as usize;
    let progress = format!("[{}{}]", "=".repeat(filled), "-".repeat(bar_w - filled));
    let progress_line = Line::from(Span::styled(
        progress,
        Style::default().fg(Color::Magenta),
    ));

    let help = Line::from(vec![
        Span::styled(
            " f/find  j/k move  enter play  space pause  n/p next/prev  r repeat  s shuffle  +/- vol  q quit ",
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let block = Block::default().borders(Borders::ALL);
    let para = Paragraph::new(vec![status, progress_line, help]).block(block);
    f.render_widget(para, area);
}

fn fmt_dur(d: Duration) -> String {
    let s = d.as_secs();
    format!("{}:{:02}", s / 60, s % 60)
}

fn fmt_secs(s: f64) -> String {
    let s = s as u64;
    format!("{}:{:02}", s / 60, s % 60)
}