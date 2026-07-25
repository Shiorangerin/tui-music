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

    let mid_y = inner.y + inner.height / 2;
    let half = (inner.height as f32 / 2.0).max(1.0);

    // 每一可用列宽 = 1，所以频段数 = inner.width，保证填满（无右侧空缺）
    let cols = inner.width as usize;
    if cols == 0 {
        return;
    }

    // 把 smoothed 数据重采样到 cols 列
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

    // 阈值：能量不足一格高的列视为静默，画中线点；有意义的能量列才画频谱
    let bar_thresh = 1.0 / half;

    // 中线：只在静默列铺一个淡点；有能量的列由频谱接管
    for (i, x) in (inner.x..inner.x + inner.width).enumerate() {
        if i < v.len() && v[i] > bar_thresh {
            continue; // 该列有足够频谱能量，不画静线
        }
        let cell = &mut f.buffer_mut()[(x, mid_y)];
        if cell.symbol() == " " {
            cell.set_char('·');
            cell.set_style(Style::default().fg(Color::Rgb(40, 42, 54)));
        }
    }

    // 每列一个 1-字符宽的小点列；能量决定上下两条镜像细线长度。
    // 关键：能量很小时 cull 掉一端（按整列偶数/奇数分布），避免稳态下两条平行线。
    for i in 0..cols {
        let energy = v[i].clamp(0.0, 1.0);
        if energy < bar_thresh {
            continue; // 能量太少，留中线
        }
        let h = energy * half;
        let full = h.floor() as i32;
        let frac = h - full as f32;

        let freq = i as f32 / (cols as f32 - 1.0).max(1.0);
        let color = spectrum_color(freq);

        let x = inner.x + i as u16;

        // 上半：从 mid_y - 1 起，向上画 full 个点 (避开中线那一行)
        let mut up = 0i32;
        while up < full {
            let y = mid_y as i32 - 1 - up;
            if y < inner.y as i32 {
                break;
            }
            put_dot(f, x, y as u16, color, false);
            up += 1;
        }
        if frac > 0.0 && up < half as i32 && full >= 1 {
            let y = mid_y as i32 - 1 - up;
            if y >= inner.y as i32 {
                put_dot(f, x, y as u16, color, true);
            }
        }

        // 下半镜像：从 mid_y + 1 起
        let mut dn = 0i32;
        while dn < full {
            let y = mid_y as i32 + 1 + dn;
            if y > inner.bottom() as i32 - 1 {
                break;
            }
            put_dot(f, x, y as u16, color, false);
            dn += 1;
        }
        if frac > 0.0 && dn < half as i32 && full >= 1 {
            let y = mid_y as i32 + 1 + dn;
            if y <= inner.bottom() as i32 - 1 {
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