//! Progress reporting for downloads

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;

/// Progress reporter for downloads
pub struct ProgressReporter {
    multi: Arc<MultiProgress>,
}

impl ProgressReporter {
    pub fn new() -> Self {
        Self {
            multi: Arc::new(MultiProgress::new()),
        }
    }

    /// Create a new download progress bar
    pub fn new_download(&self, name: &str, total_size: u64) -> DownloadProgress {
        let pb = self.multi.add(ProgressBar::new(total_size));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  {spinner:.cyan} {msg:20} [{bar:30.cyan/dim}] {bytes}/{total_bytes} ({bytes_per_sec})")
                .unwrap()
                .progress_chars("━━╸━"),
        );
        pb.set_message(name.to_string());
        DownloadProgress { pb }
    }

    /// Create a spinner for indeterminate progress
    pub fn new_spinner(&self, message: &str) -> DownloadProgress {
        let pb = self.multi.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        pb.set_message(message.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        DownloadProgress { pb }
    }

    /// Create a summary progress bar
    pub fn new_summary(&self, total: u64, message: &str) -> DownloadProgress {
        let pb = self.multi.add(ProgressBar::new(total));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  {msg}\n  [{bar:40.cyan/dim}] {pos}/{len}")
                .unwrap()
                .progress_chars("━━╸━"),
        );
        pb.set_message(message.to_string());
        DownloadProgress { pb }
    }
}

impl Default for ProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

/// A single download's progress
pub struct DownloadProgress {
    pb: ProgressBar,
}

impl DownloadProgress {
    /// Update progress
    pub fn set_position(&self, pos: u64) {
        self.pb.set_position(pos);
    }

    /// Increment progress
    pub fn inc(&self, delta: u64) {
        self.pb.inc(delta);
    }

    /// Mark as complete
    pub fn finish(&self) {
        self.pb.finish_and_clear();
    }

    /// Mark as complete with a message
    pub fn finish_with_message(&self, msg: &str) {
        self.pb.finish_with_message(msg.to_string());
    }

    /// Update message
    pub fn set_message(&self, msg: &str) {
        self.pb.set_message(msg.to_string());
    }
}
