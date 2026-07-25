use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use tui_music::library::{scan, Track};
use tui_music::player::{Player, RepeatMode};

pub struct App {
    pub music_dir: PathBuf,
    pub tracks: Vec<Track>,
    pub selected: Option<usize>,
    pub current: Option<usize>,
    pub repeat: RepeatMode,
    pub shuffle: bool,
    pub volume: f32,
    pub player: Player,
    pub bars: [u16; tui_music::viz::BARS],
    pub should_quit: bool,
}

impl App {
    pub fn new(music_dir: PathBuf) -> anyhow::Result<Self> {
        let tracks = if music_dir.exists() {
            scan(&music_dir)
        } else {
            Vec::new()
        };
        Ok(Self {
            music_dir,
            tracks,
            selected: None,
            current: None,
            repeat: RepeatMode::All,
            shuffle: false,
            volume: 0.6,
            player: Player::new()?,
            bars: [0u16; tui_music::viz::BARS],
            should_quit: false,
        })
    }

    pub fn selected_mut(&mut self, delta: i32) {
        let n = self.tracks.len();
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
        if let Some(i) = self.selected {
            self.play_track(i)?;
        }
        Ok(())
    }

    pub fn play_track(&mut self, i: usize) -> anyhow::Result<()> {
        if i >= self.tracks.len() {
            return Ok(());
        }
        let path = self.tracks[i].path.clone();
        self.player.play_file(&path)?;
        self.player.set_volume(self.volume);
        self.current = Some(i);
        Ok(())
    }

    pub fn next(&mut self) -> anyhow::Result<()> {
        let n = self.tracks.len();
        if n == 0 {
            return Ok(());
        }
        let cur = self.current.unwrap_or(0);
        let next_i = if self.shuffle {
            if n == 1 { 0 } else {
                let r = (rand_next(n)) as usize;
                if r == cur { (r + 1) % n } else { r }
            }
        } else {
            (cur + 1) % n
        };
        self.play_track(next_i)?;
        Ok(())
    }

    pub fn prev(&mut self) -> anyhow::Result<()> {
        let n = self.tracks.len();
        if n == 0 {
            return Ok(());
        }
        let cur = self.current.unwrap_or(0);
        let prev_i = if cur == 0 { n - 1 } else { cur - 1 };
        self.play_track(prev_i)?;
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

    pub fn update(&mut self) -> anyhow::Result<()> {
        self.player.update_position();

        if let Some(i) = self.current {
            let d = self.tracks[i].duration;
            let mut finished = self.player.is_finished();
            if d > 0.0 && self.player.position.as_secs_f64() >= d && self.player.is_finished() {
                finished = true;
            }
            if finished && self.player.playing {
                self.player.playing = false;
                match self.repeat {
                    RepeatMode::One => {
                        self.play_track(i)?;
                    }
                    _ => {
                        if i + 1 < self.tracks.len() || matches!(self.repeat, RepeatMode::All) {
                            self.next()?;
                        } else {
                            self.player.playing = false;
                        }
                    }
                }
            }
        }

        let samples = self.player.viz.latest(tui_music::viz::FFT_N);
        self.bars = tui_music::viz::compute(&samples.clone());
        Ok(())
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match (ctrl, key.code) {
            (false, KeyCode::Char('q') | KeyCode::Esc) => self.should_quit = true,
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