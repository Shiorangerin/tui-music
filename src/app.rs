use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use tui_music::library::{scan, Track};
use tui_music::player::{Player, RepeatMode};

pub struct App {
    pub music_dir: PathBuf,
    pub tracks: Vec<Track>,
    pub display: Vec<usize>,
    pub selected: Option<usize>,
    pub current: Option<usize>,
    pub repeat: RepeatMode,
    pub shuffle: bool,
    pub volume: f32,
    pub player: Player,
    pub bars: Vec<u16>,
    pub smoothed: Vec<f32>,
    pub should_quit: bool,
    pub search: String,
    pub search_active: bool,
}

impl App {
    pub fn new(music_dir: PathBuf) -> anyhow::Result<Self> {
        let tracks = if music_dir.exists() {
            scan(&music_dir)
        } else {
            Vec::new()
        };
        let display = (0..tracks.len()).collect();
        Ok(Self {
            music_dir,
            tracks,
            display,
            selected: None,
            current: None,
            repeat: RepeatMode::All,
            shuffle: false,
            volume: 0.6,
            player: Player::new()?,
            bars: Vec::new(),
            smoothed: Vec::new(),
            should_quit: false,
            search: String::new(),
            search_active: false,
        })
    }

    fn rebuild_display(&mut self) {
        if self.search.is_empty() {
            self.display = (0..self.tracks.len()).collect();
        } else {
            let q = self.search.to_lowercase();
            self.display = self
                .tracks
                .iter()
                .enumerate()
                .filter(|(_, t)| {
                    t.display_name().to_lowercase().contains(&q)
                        || t.subtitle().to_lowercase().contains(&q)
                })
                .map(|(i, _)| i)
                .collect();
        }
        let n = self.display.len();
        if n == 0 {
            self.selected = None;
        } else if self.selected.map_or(true, |s| s >= n) {
            self.selected = Some(0);
        }
    }

    pub fn enter_search(&mut self) {
        self.search_active = true;
    }

    pub fn exit_search(&mut self, keep: bool) {
        self.search_active = false;
        if !keep {
            self.search.clear();
            self.rebuild_display();
        }
    }

    pub fn search_push(&mut self, c: char) {
        self.search.push(c);
        self.rebuild_display();
    }

    pub fn search_pop(&mut self) {
        self.search.pop();
        self.rebuild_display();
    }

    fn orig_of(&self, view_i: usize) -> Option<usize> {
        self.display.get(view_i).copied()
    }

    pub fn selected_mut(&mut self, delta: i32) {
        let n = self.display.len();
        if n == 0 {
            return;
        }
        let cur = self.selected.unwrap_or_else(|| if delta > 0 { 0 } else { n - 1 });
        let mut next = cur as i32 + delta;
        while next < 0 {
            next += n as i32;
        }
        let next = (next as usize) % n;
        self.selected = Some(next);
    }

    pub fn play_selected(&mut self) -> anyhow::Result<()> {
        if let Some(v) = self.selected {
            if let Some(o) = self.orig_of(v) {
                self.play_track(o)?;
                self.current = Some(v);
            }
        }
        Ok(())
    }

    pub fn play_track(&mut self, orig_i: usize) -> anyhow::Result<()> {
        if orig_i >= self.tracks.len() {
            return Ok(());
        }
        let path = self.tracks[orig_i].path.clone();
        self.player.play_file(&path)?;
        self.player.set_volume(self.volume);
        Ok(())
    }

    pub fn next(&mut self) -> anyhow::Result<()> {
        let n = self.display.len();
        if n == 0 {
            return Ok(());
        }
        let cur = self.current.unwrap_or(0);
        let next_v = if self.shuffle {
            if n == 1 { 0 } else {
                let r = (rand_next(n)) as usize;
                if r == cur { (r + 1) % n } else { r }
            }
        } else {
            (cur + 1) % n
        };
        if let Some(o) = self.orig_of(next_v) {
            self.play_track(o)?;
            self.current = Some(next_v);
        }
        Ok(())
    }

    pub fn prev(&mut self) -> anyhow::Result<()> {
        let n = self.display.len();
        if n == 0 {
            return Ok(());
        }
        let cur = self.current.unwrap_or(0);
        let prev_v = if cur == 0 { n - 1 } else { cur - 1 };
        if let Some(o) = self.orig_of(prev_v) {
            self.play_track(o)?;
            self.current = Some(prev_v);
        }
        Ok(())
    }

    pub fn toggle_pause(&mut self) {
        if self.player.playing {
            self.player.pause();
        } else if self.current.is_some() {
            self.player.resume();
        }
    }

    pub fn cycle_repeat(&mut self) {
        match self.repeat {
            RepeatMode::Off => self.repeat = RepeatMode::All,
            RepeatMode::All => self.repeat = RepeatMode::One,
            RepeatMode::One => self.repeat = RepeatMode::Off,
        }
    }

    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
    }

    pub fn change_volume(&mut self, delta: f32) {
        self.volume = (self.volume + delta).clamp(0.0, 1.0);
        self.player.set_volume(self.volume);
    }

    fn smooth(&mut self, raw: Vec<f32>) {
        if self.smoothed.len() != raw.len() {
            self.smoothed = raw.clone();
            return;
        }
        // attack fast, release slow => bars rise quickly, fall gently
        const ATTACK: f32 = 0.45;
        const RELEASE: f32 = 0.08;
        for (i, &r) in raw.iter().enumerate() {
            let s = self.smoothed[i];
            let coeff = if r > s { ATTACK } else { RELEASE };
            self.smoothed[i] = s + (r - s) * coeff;
        }
    }

    pub fn update(&mut self, num_bars: usize) -> anyhow::Result<()> {
        self.player.update_position();

        if let Some(v) = self.current {
            if let Some(o) = self.orig_of(v) {
                let d = self.tracks[o].duration;
                let mut finished = self.player.is_finished();
                if d > 0.0 && self.player.position.as_secs_f64() >= d && self.player.is_finished() {
                    finished = true;
                }
                if finished && self.player.playing {
                    self.player.playing = false;
                    match self.repeat {
                        RepeatMode::One => {
                            self.play_track(o)?;
                        }
                        _ => {
                            let n = self.display.len();
                            if v + 1 < n || matches!(self.repeat, RepeatMode::All) {
                                self.next()?;
                            }
                        }
                    }
                }
            }
        }

        let samples = self.player.viz.latest(tui_music::viz::FFT_N);
        let raw = tui_music::viz::compute(&samples, num_bars.max(16));
        self.smooth(raw);
        self.bars = self.smoothed.iter().map(|&v| (v * 20.0) as u16).collect();
        Ok(())
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if self.search_active {
            match key.code {
                KeyCode::Esc => self.exit_search(false),
                KeyCode::Enter => self.exit_search(true),
                KeyCode::Backspace => self.search_pop(),
                KeyCode::Char(c) => self.search_push(c),
                _ => {}
            }
            return Ok(());
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match (ctrl, key.code) {
            (false, KeyCode::Char('q') | KeyCode::Esc) => self.should_quit = true,
            (false, KeyCode::Char('/')) | (false, KeyCode::Char('f')) => self.enter_search(),
            (false, KeyCode::Up | KeyCode::Char('k')) => self.selected_mut(-1),
            (false, KeyCode::Down | KeyCode::Char('j')) => self.selected_mut(1),
            (false, KeyCode::Enter) | (false, KeyCode::Char('l')) => self.play_selected()?,
            (false, KeyCode::Char(' ')) => self.toggle_pause(),
            (false, KeyCode::Char('n')) | (false, KeyCode::Tab) => self.next()?,
            (false, KeyCode::Char('p')) | (false, KeyCode::BackTab) => self.prev()?,
            (false, KeyCode::Char('r')) => self.cycle_repeat(),
            (false, KeyCode::Char('s')) => self.toggle_shuffle(),
            (false, KeyCode::Char('=')) | (false, KeyCode::Char('+')) => self.change_volume(0.05),
            (false, KeyCode::Char('-')) => self.change_volume(-0.05),
            _ => {}
        }
        Ok(())
    }
}

fn rand_next(n: usize) -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;
    let mut h = DefaultHasher::new();
    SystemTime::now().hash(&mut h);
    (h.finish() as u32) % (n.max(1) as u32)
}