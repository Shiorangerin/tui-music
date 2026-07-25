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
            Constraint::Min(6),
            Constraint::Length(16),
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
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // 每列一格宽、列数 = inner.width，保证填满整宽
    let cols = inner.width as usize;
    if cols == 0 {
        return;
    }
    let max_h = inner.height as i32; // 总可用高度

    // 重采样 smoothed 到 cols 列
    let src = &app.smoothed;
    let v: Vec<f32> = if src.is_empty() {
        vec![0.0; cols]
    } else if src.len() == cols {
        src.clone()
    } else {
        (0..cols)
            .map(|i| {
                let p = i as f32 * (src.len() as f32 - 1.0) / (cols as f32 - 1.0).max(1.0);
                let lo = p.floor() as usize;
                let hi = (lo + 1).min(src.len() - 1);
                let t = p - lo as f32;
                src[lo] * (1.0 - t) + src[hi] * t
            })
            .collect()
    };

    // 底线：整行实线
    let bottom = inner.bottom() - 1;
    for x in inner.x..inner.x + inner.width {
        let cell = &mut f.buffer_mut()[(x, bottom)];
        cell.set_char('─');
        cell.set_style(Style::default().fg(Color::Rgb(60, 62, 80)));
    }

    // 阈值：能量不足一格高的列视作静默
    let bar_thresh = 1.0 / max_h as f32;

    // 柱子：从底线上方一格向上画密集小点
    for i in 0..cols {
        let energy = v[i].clamp(0.0, 1.0);
        if energy < bar_thresh {
            continue;
        }
        let h = energy * max_h as f32;
        let full = h.floor() as i32;
        let frac = h - full as f32;

        let freq = i as f32 / (cols as f32 - 1.0).max(1.0);
        let color = spectrum_color(freq);

        let x = inner.x + i as u16;
        let top = inner.y as i32;
        let base = bottom as i32 - 1; // 底线上方一格起步

        let mut drawn = 0i32;
        while drawn < full {
            let y = base - drawn;
            if y < top {
                break;
            }
            put_dot(f, x, y as u16, color, false);
            drawn += 1;
        }
        // 顶端 1/8 像素精度半点收尾（仅当已有整格才画，避免恒底座）
        if frac > 0.0 && full >= 1 {
            let y = base - drawn;
            if y >= top {
                put_dot(f, x, y as u16, color, true);
            }
        }
    }
}

fn put_dot(f: &mut Frame, x: u16, y: u16, color: Color, half: bool) {
    let ch = if half { '∙' } else { '·' };
    let style = if half {
        Style::default().fg(dim_color(color, 0.6))
    } else {
        Style::default().fg(color)
    };
    f.buffer_mut().set_string(x, y, ch.encode_utf8(&mut [0u8; 4]), style);
}

fn spectrum_color(freq: f32) -> Color {
    if freq < 0.33 {
        blend(Color::Rgb(120, 220, 240), Color::Rgb(170, 140, 250), freq / 0.33)
    } else if freq < 0.66 {
        blend(Color::Rgb(170, 140, 250), Color::Rgb(250, 130, 200), (freq - 0.33) / 0.33)
    } else {
        blend(Color::Rgb(250, 130, 200), Color::Rgb(250, 210, 160), (freq - 0.66) / 0.34)
    }
}

fn dim_color(c: Color, factor: f32) -> Color {
    let (r, g, b) = to_rgb(c);
    Color::Rgb(
        (r as f32 * factor) as u8,
        (g as f32 * factor) as u8,
        (b as f32 * factor) as u8,
    )
}

fn blend(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let (ar, ag, ab) = to_rgb(a);
    let (br, bg, bb) = to_rgb(b);
    let r = (ar as f32 * (1.0 - t) + br as f32 * t) as u8;
    let g = (ag as f32 * (1.0 - t) + bg as f32 * t) as u8;
    let bch = (ab as f32 * (1.0 - t) + bb as f32 * t) as u8;
    Color::Rgb(r, g, bch)
}

fn to_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Cyan => (0, 220, 220),
        Color::Blue => (60, 120, 230),
        Color::Magenta => (220, 80, 200),
        Color::Yellow => (240, 210, 90),
        Color::White => (230, 230, 230),
        _ => (150, 150, 150),
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