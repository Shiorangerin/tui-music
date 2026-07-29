use std::path::{Path, PathBuf};
use lofty::probe::Probe;
use lofty::file::TaggedFileExt;
use lofty::prelude::{Accessor, AudioFile};
use walkdir::WalkDir;

#[derive(Clone)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub duration: f64,
}

impl Track {
    pub fn display_name(&self) -> String {
        if self.title.is_empty() {
            self.path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "?".to_string())
        } else {
            self.title.clone()
        }
    }

    pub fn subtitle(&self) -> String {
        if self.artist.is_empty() {
            self.path
                .parent()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        } else {
            self.artist.clone()
        }
    }
}

const EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "vorbis", "m4a", "aac", "opus", "aiff", "aif", "alac"];

pub fn is_audio(p: &Path) -> bool {
    let e = match p.extension() {
        Some(e) => e.to_string_lossy().to_lowercase(),
        None => return false,
    };
    EXTENSIONS.contains(&e.as_str())
}

pub fn scan(root: &Path) -> Vec<Track> {
    let mut tracks = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() || !is_audio(p) {
            continue;
        }
        match load(p) {
            Ok(t) => tracks.push(t),
            Err(_) => tracks.push(Track {
                path: p.to_path_buf(),
                title: String::new(),
                artist: String::new(),
                duration: 0.0,
            }),
        }
    }
    tracks.sort_by(|a, b| a.display_name().to_lowercase().cmp(&b.display_name().to_lowercase()));
    tracks
}

fn load(p: &Path) -> anyhow::Result<Track> {
    let tagged = Probe::open(p)?.read()?;
    let tags = tagged.primary_tag();
    let (title, artist) = if let Some(t) = tags {
        (
            t.title().map(|s| s.to_string()).unwrap_or_default(),
            t.artist().map(|s| s.to_string()).unwrap_or_default(),
        )
    } else {
        (String::new(), String::new())
    };
    let duration = tagged.properties().duration().as_secs_f64();
    Ok(Track {
        path: p.to_path_buf(),
        title,
        artist,
        duration,
    })
}

pub fn default_music_dir() -> PathBuf {
    if let Some(dir) = dirs_fallback() {
        return dir;
    }
    PathBuf::from(".")
}

fn dirs_fallback() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("Music"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Music"))
    }
}