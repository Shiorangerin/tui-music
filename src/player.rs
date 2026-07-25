use anyhow::Result;
use rodio::source::Source;
use rodio::{Decoder, OutputStream, Sink};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Ring buffer of recent PCM samples (mono mix) for visualization.
#[derive(Clone)]
pub struct VizBuffer {
    inner: Arc<Mutex<Vec<f32>>>,
    capacity: usize,
}

impl VizBuffer {
    pub fn new(capacity: usize) -> Self {
        Self { inner: Arc::new(Mutex::new(Vec::with_capacity(capacity))), capacity }
    }

    pub fn push(&self, s: f32) {
        let mut v = self.inner.lock().unwrap();
        if v.len() >= self.capacity {
            v.remove(0);
        }
        v.push(s);
    }

    pub fn latest(&self, n: usize) -> Vec<f32> {
        let v = self.inner.lock().unwrap();
        let len = v.len();
        let start = len.saturating_sub(n);
        let mut out = v[start..].to_vec();
        if out.len() < n {
            out.resize(n, 0.0);
        }
        out
    }
}

/// A Source wrapper that down-mixes to mono and mirrors samples into a VizBuffer.
struct MonoTee<S: Source<Item = f32>> {
    inner: S,
    viz: VizBuffer,
}

impl<S: Source<Item = f32>> Iterator for MonoTee<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let ch = u16::max(self.inner.channels(), 1) as usize;
        let mut sum = 0.0f32;
        let mut n = 0usize;
        for _ in 0..ch {
            if let Some(s) = self.inner.next() {
                sum += s;
                n += 1;
            } else {
                break;
            }
        }
        if n == 0 {
            return None;
        }
        let mono = sum / n as f32;
        self.viz.push(mono);
        Some(mono)
    }
}

impl<S: Source<Item = f32>> Source for MonoTee<S> {
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

pub enum RepeatMode {
    Off,
    One,
    All,
}

pub struct Player {
    _stream: OutputStream,
    sink: Sink,
    pub viz: VizBuffer,
    pub playing: bool,
    pub position: Duration,
    pub last_tick: std::time::Instant,
}

impl Player {
    pub fn new() -> Result<Self> {
        let (stream, handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&handle)?;
        sink.pause();
        Ok(Self {
            _stream: stream,
            sink,
            viz: VizBuffer::new(4096),
            playing: false,
            position: Duration::ZERO,
            last_tick: std::time::Instant::now(),
        })
    }

    pub fn play_file(&mut self, path: &Path) -> Result<()> {
        let file = std::fs::File::open(path)?;
        let dec = Decoder::new(file)?;
        let tee = MonoTee { inner: dec.convert_samples(), viz: self.viz.clone() };
        self.sink.clear();
        self.sink.append(tee);
        self.sink.play();
        self.playing = true;
        self.position = Duration::ZERO;
        self.last_tick = std::time::Instant::now();
        Ok(())
    }

    pub fn pause(&mut self) {
        self.sink.pause();
        self.playing = false;
    }

    pub fn resume(&mut self) {
        self.sink.play();
        self.playing = true;
        self.last_tick = std::time::Instant::now();
    }

    pub fn is_finished(&self) -> bool {
        self.sink.empty()
    }

    pub fn update_position(&mut self) {
        if self.playing {
            let now = std::time::Instant::now();
            let dt = now - self.last_tick;
            self.position += dt;
            self.last_tick = now;
        } else {
            self.last_tick = std::time::Instant::now();
        }
    }

    pub fn set_volume(&self, v: f32) {
        self.sink.set_volume(v);
    }
}