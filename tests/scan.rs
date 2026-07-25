use std::path::PathBuf;
use tui_music::library::{is_audio, scan};

#[test]
fn detects_audio_extensions() {
    assert!(is_audio(&PathBuf::from("foo.mp3")));
    assert!(is_audio(&PathBuf::from("BAR.FLAC")));
    assert!(!is_audio(&PathBuf::from("notes.txt")));
    assert!(!is_audio(&PathBuf::from("noext")));
}

#[test]
fn scans_generated_sample_dir() {
    let dir = PathBuf::from("/tmp/tmtest/music");
    if !dir.exists() {
        return;
    }
    let tracks = scan(&dir);
    assert!(tracks.len() >= 2);
    assert!(tracks.iter().any(|t| t.path.extension().unwrap_or_default() == "mp3"));
}