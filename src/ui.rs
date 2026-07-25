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

    if app.bars.is_empty() {
        return;
    }

    let cols = app.bars.len();
    let mid_y = inner.y + inner.height / 2;
    let half = (inner.height / 2) as f32;

    // 中线
    for x in inner.x..inner.x + inner.width {
        let cell = &mut f.buffer_mut()[(x, mid_y)];
        if cell.symbol() == " " {
            cell.set_char('·');
            cell.set_style(Style::default().fg(Color::DarkGray));
        }
    }

    // 频谱：从中线上下镜像延伸，细线（每列一格宽）
    let cell_w = (inner.width as usize / cols.max(1)).max(1);
    for i in 0..cols {
        let v = (app.bars[i] as f32).min(20.0) / 20.0;
        let h = v * half;
        let x = inner.x + (i * cell_w) as u16;

        let freq = i as f32 / (cols - 1).max(1) as f32;
        let color = spectrum_color(freq, 0.0);

        // 上半部分（从中线向上一格起算）
        draw_bar_half(f, x, mid_y.saturating_sub(1), inner.y, h, true, color);
        // 下半部分（从中线向下一格起算），镜像
        draw_bar_half(f, x, mid_y + 1, inner.bottom() - 1, h, false, color);
    }
}

fn spectrum_color(freq: f32, _intensity: f32) -> Color {
    // 一条柔和的频率渐变：青 -> 紫红 -> 琥珀，不动白
    if freq < 0.33 {
        blend(Color::Rgb(80, 200, 220), Color::Rgb(140, 130, 230), freq / 0.33)
    } else if freq < 0.66 {
        blend(Color::Rgb(140, 130, 230), Color::Rgb(220, 120, 170), (freq - 0.33) / 0.33)
    } else {
        blend(Color::Rgb(220, 120, 170), Color::Rgb(240, 200, 120), (freq - 0.66) / 0.34)
    }
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

/// 画从 `start_y` 沿 `direction` 方向（上为 true，下为 false）延伸 `h` 个单位的细线。
/// `bound` 是该方向上的不可逾越边界（向上画时是顶边 inner.y，向下画时是底边）。
fn draw_bar_half(
    f: &mut Frame,
    x: u16,
    start_y: u16,
    bound: u16,
    h: f32,
    upward: bool,
    color: Color,
) {
    if h <= 0.0 {
        return;
    }
    let full = h.floor() as u16;
    let frac = h - full as f32;

    let mut y = start_y;
    for k in 0..full {
        if upward {
            if y < bound {
                break;
            }
        } else if y > bound {
            break;
        }
        put(f, x, y, "█", color);
        if upward {
            if y == 0 {
                break;
            }
            y = y - 1;
        } else {
            y = y + 1;
        }
        let _ = k;
    }

    // 顶端/底端 sub-block 平滑收尾
    if frac > 0.0 {
        let level = ((frac * 8.0).ceil() as u8).clamp(1, 8);
        let ch = sub_block(level);
        if upward && y >= bound {
            put(f, x, y, ch, color);
        } else if !upward && y <= bound {
            put(f, x, y, ch, color);
        }
    }
}

fn put(f: &mut Frame, x: u16, y: u16, s: &str, color: Color) {
    f.buffer_mut().set_string(x, y, s, Style::default().fg(color));
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