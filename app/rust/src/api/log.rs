use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Points the Rust-side core log at the same journal file the Dart
/// layer writes to; must be called once at startup with the app
/// support directory path.
pub fn set_core_log_file(path: String) {
    *LOG_PATH.lock().unwrap() = Some(PathBuf::from(path));
}

/// Appends one timestamped line to the core journal; silently no-op
/// until the file is set or if the disk write fails.
pub fn core_log(source: &str, message: &str) {
    let guard = LOG_PATH.lock().unwrap();
    let Some(path) = guard.as_ref() else {
        return;
    };
    let flat = message.replace('\n', "\\n");
    let line = format!("{} [rust-{}] {}\n", utc_stamp(), source, flat);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

/// UTC timestamp without pulling in a date library: civil-from-days.
fn utc_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    let days = (secs / 86400) as i64;
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    format!("{year:04}-{month:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_is_iso_like() {
        let stamp = utc_stamp();
        assert_eq!(stamp.len(), 20);
        assert!(stamp.starts_with("20"));
        assert!(stamp.ends_with('Z'));
        assert_eq!(stamp.split('-').count(), 3);
    }

    #[test]
    fn log_without_file_is_silent() {
        // must not panic even though no file was set in this process
        core_log("test", "no file configured");
    }
}
