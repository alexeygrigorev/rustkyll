//! Build progress reporting for rustkyll.
//!
//! Provides a `ProgressReporter` that emits phase indicators and a progress bar
//! to stderr during the build. All real-time progress goes to stderr; the final
//! summary is left to the caller (stdout).
//!
//! When `quiet` is true, all progress output is suppressed.
//! When stderr is not a TTY, the progress bar falls back to simple line-by-line
//! output without ANSI escape codes.

use std::io::IsTerminal;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

/// Controls how progress is reported during a build.
#[derive(Clone)]
pub struct ProgressReporter {
    quiet: bool,
    is_tty: bool,
}

impl ProgressReporter {
    /// Create a new progress reporter.
    ///
    /// - `quiet`: if true, suppress all progress output.
    pub fn new(quiet: bool) -> Self {
        let is_tty = std::io::stderr().is_terminal();
        Self { quiet, is_tty }
    }

    /// Create a reporter that writes to a buffer (for testing).
    #[cfg(test)]
    pub fn new_with_tty(quiet: bool, is_tty: bool) -> Self {
        Self { quiet, is_tty }
    }

    /// Returns true if this reporter is in quiet mode.
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    /// Emit a phase start message (e.g., "Loading config...").
    pub fn phase(&self, message: &str) {
        if self.quiet {
            return;
        }
        eprintln!("{}", message);
    }

    /// Emit a phase completion message with detail (e.g., "Loading collections... 6 collections, 1543 items").
    pub fn phase_done(&self, message: &str) {
        if self.quiet {
            return;
        }
        eprintln!("{}", message);
    }

    /// Create a progress bar for rendering pages.
    ///
    /// Returns a `RenderProgress` that can be shared across threads.
    /// When quiet, returns a no-op progress tracker.
    pub fn render_progress(&self, total: u64, prefix: &str) -> RenderProgress {
        if self.quiet {
            return RenderProgress {
                bar: None,
                counter: Arc::new(AtomicUsize::new(0)),
                total: total as usize,
            };
        }

        let bar = ProgressBar::new(total);

        if self.is_tty {
            bar.set_draw_target(ProgressDrawTarget::stderr());
            bar.set_style(
                ProgressStyle::with_template("{prefix} [{wide_bar:.cyan/blue}] {pos}/{len}  {msg}")
                    .unwrap_or_else(|_| ProgressStyle::default_bar())
                    .progress_chars("=>-"),
            );
        } else {
            // Non-TTY: use a hidden bar -- we will print lines manually
            bar.set_draw_target(ProgressDrawTarget::hidden());
        }
        bar.set_prefix(prefix.to_string());

        RenderProgress {
            bar: Some(bar),
            counter: Arc::new(AtomicUsize::new(0)),
            total: total as usize,
        }
    }
}

/// Thread-safe progress tracker for page rendering.
pub struct RenderProgress {
    bar: Option<ProgressBar>,
    counter: Arc<AtomicUsize>,
    total: usize,
}

impl RenderProgress {
    /// Increment the progress counter by 1 and set the current file message.
    ///
    /// This is called from inside rayon's `par_iter` closure so the progress
    /// bar updates in real time as each page is rendered.
    pub fn inc(&self, current_file: &str) {
        self.counter.fetch_add(1, Ordering::Relaxed);
        if let Some(ref bar) = self.bar {
            bar.inc(1);
            bar.set_message(current_file.to_string());
        }
    }

    /// Get the current count.
    pub fn count(&self) -> usize {
        self.counter.load(Ordering::Relaxed)
    }

    /// Get the total.
    pub fn total(&self) -> usize {
        self.total
    }

    /// Finish the progress bar.
    pub fn finish(&self) {
        if let Some(ref bar) = self.bar {
            bar.finish_and_clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quiet_mode_suppresses_output() {
        let reporter = ProgressReporter::new(true);
        assert!(reporter.is_quiet());
        // These should not panic or produce output
        reporter.phase("Loading config...");
        reporter.phase_done("Loading config... done");
        let progress = reporter.render_progress(100, "Rendering");
        progress.inc("test.html");
        progress.finish();
    }

    #[test]
    fn test_normal_mode_not_quiet() {
        let reporter = ProgressReporter::new(false);
        assert!(!reporter.is_quiet());
    }

    #[test]
    fn test_render_progress_counter_thread_safe() {
        let reporter = ProgressReporter::new_with_tty(false, false);
        let progress = reporter.render_progress(100, "Rendering");

        // Use scoped threads so we can pass &progress (which is not Send)
        std::thread::scope(|s| {
            for _ in 0..10 {
                s.spawn(|| {
                    for i in 0..10 {
                        progress.inc(&format!("page-{i}.html"));
                    }
                });
            }
        });

        // 10 threads x 10 increments = 100
        assert_eq!(progress.count(), 100);
    }

    #[test]
    fn test_render_progress_quiet_returns_noop() {
        let reporter = ProgressReporter::new(true);
        let progress = reporter.render_progress(50, "Rendering");
        assert!(progress.bar.is_none());
        progress.inc("file.html");
        assert_eq!(progress.count(), 1);
        progress.finish();
    }

    #[test]
    fn test_render_progress_tracks_total() {
        let reporter = ProgressReporter::new(true);
        let progress = reporter.render_progress(42, "Test");
        assert_eq!(progress.total(), 42);
    }

    #[test]
    fn test_phase_messages_in_normal_mode() {
        // This test verifies that phase() and phase_done() don't panic
        // in normal mode. We can't easily capture stderr in a unit test,
        // but integration tests will verify actual output.
        let reporter = ProgressReporter::new_with_tty(false, false);
        reporter.phase("Loading config...");
        reporter.phase_done("Loading collections... 5 collections, 100 items");
    }
}
