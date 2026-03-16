//! Build progress reporting for rustkyll.
//!
//! Provides a `ProgressReporter` that emits phase indicators and a progress bar
//! to stderr during the build. All real-time progress goes to stderr; the final
//! summary is left to the caller (stdout).
//!
//! When `quiet` is true, all progress output is suppressed.
//! When stderr is not a TTY, the progress bar falls back to simple line-by-line
//! output without ANSI escape codes.
//!
//! On TTY, the progress bar uses a two-line layout:
//! - Line 1: Progress bar with elapsed time (fixed width, stable)
//! - Line 2: Current file being rendered (updates independently)

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};

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
    ///
    /// On TTY: prints the message without a trailing newline so the cursor stays
    /// on the same line, allowing `phase_done()` to overwrite it in place.
    /// Uses `\r` and ANSI erase-line to cleanly replace any previous in-progress text.
    /// On non-TTY: no-op; only `phase_done()` prints the final line.
    pub fn phase(&self, message: &str) {
        if self.quiet || !self.is_tty {
            return;
        }
        // \r returns to start of line; \x1b[2K erases the entire line
        eprint!("\r\x1b[2K{}", message);
        let _ = std::io::stderr().flush();
    }

    /// Emit a phase completion message with detail.
    ///
    /// On TTY: uses carriage return + ANSI erase to overwrite the in-progress message
    /// from `phase()`, then prints the completed message with a newline.
    /// On non-TTY: prints the final line once (since `phase()` was a no-op).
    pub fn phase_done(&self, message: &str) {
        if self.quiet {
            return;
        }
        if self.is_tty {
            // \r returns to start of line; \x1b[2K erases the entire line
            eprintln!("\r\x1b[2K{}", message);
        } else {
            eprintln!("{}", message);
        }
    }

    /// Create a progress bar for rendering pages.
    ///
    /// Returns a `RenderProgress` that can be shared across threads.
    /// When quiet, returns a no-op progress tracker.
    ///
    /// On TTY, creates a two-line layout using `MultiProgress`:
    /// - Line 1: `Rendering pages [========>---] 650/789 (1.2s)`
    /// - Line 2: `  -> current-file.html`
    pub fn render_progress(&self, total: u64, prefix: &str) -> RenderProgress {
        if self.quiet {
            return RenderProgress {
                bar: None,
                file_line: None,
                _multi: None,
                counter: Arc::new(AtomicUsize::new(0)),
                total: total as usize,
            };
        }

        if self.is_tty {
            let multi = MultiProgress::with_draw_target(ProgressDrawTarget::stderr());

            let bar = multi.add(ProgressBar::new(total));
            bar.set_style(
                ProgressStyle::with_template(
                    "{prefix} [{wide_bar:.cyan/blue}] {pos}/{len} ({elapsed})",
                )
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("=>-"),
            );
            bar.set_prefix(prefix.to_string());

            let file_line = multi.add(ProgressBar::new_spinner());
            file_line.set_style(
                ProgressStyle::with_template("  \u{2192} {wide_msg}")
                    .unwrap_or_else(|_| ProgressStyle::default_spinner()),
            );
            // Start with empty message so the second line is present but blank
            file_line.set_message("");

            RenderProgress {
                bar: Some(bar),
                file_line: Some(file_line),
                _multi: Some(multi),
                counter: Arc::new(AtomicUsize::new(0)),
                total: total as usize,
            }
        } else {
            // Non-TTY: use a hidden bar -- we will print lines manually
            let bar = ProgressBar::new(total);
            bar.set_draw_target(ProgressDrawTarget::hidden());
            bar.set_prefix(prefix.to_string());

            RenderProgress {
                bar: Some(bar),
                file_line: None,
                _multi: None,
                counter: Arc::new(AtomicUsize::new(0)),
                total: total as usize,
            }
        }
    }
}

/// Thread-safe progress tracker for page rendering.
///
/// On TTY, maintains two lines: a progress bar and a current-file indicator.
/// The progress bar stays stable while the filename updates below it.
pub struct RenderProgress {
    bar: Option<ProgressBar>,
    file_line: Option<ProgressBar>,
    _multi: Option<MultiProgress>,
    counter: Arc<AtomicUsize>,
    total: usize,
}

impl RenderProgress {
    /// Increment the progress counter by 1 and set the current file message.
    ///
    /// On TTY: updates the progress bar (line 1) and the file indicator (line 2)
    /// independently, so the bar stays stable while filenames change.
    /// This is called from inside rayon's `par_iter` closure so the progress
    /// bar updates in real time as each page is rendered.
    pub fn inc(&self, current_file: &str) {
        self.counter.fetch_add(1, Ordering::Relaxed);
        if let Some(ref bar) = self.bar {
            bar.inc(1);
        }
        if let Some(ref file_line) = self.file_line {
            file_line.set_message(current_file.to_string());
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

    /// Finish the progress bar and clear both lines.
    pub fn finish(&self) {
        if let Some(ref file_line) = self.file_line {
            file_line.finish_and_clear();
        }
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
        assert!(progress.file_line.is_none());
        assert!(progress._multi.is_none());
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

    #[test]
    fn test_tty_mode_phase_and_phase_done() {
        // Verify the TTY code path executes without panic
        let reporter = ProgressReporter::new_with_tty(false, true);
        reporter.phase("Loading...");
        reporter.phase_done("Loading... 5 items");
    }

    #[test]
    fn test_non_tty_mode_phase_is_noop() {
        // In non-TTY mode, phase() should be a no-op (no output).
        // phase_done() should print the final message.
        // We verify the code paths execute without panic.
        let reporter = ProgressReporter::new_with_tty(false, false);
        reporter.phase("Loading..."); // should be no-op
        reporter.phase_done("Loading... 5 items"); // should print once
    }

    #[test]
    fn test_phase_done_without_prior_phase() {
        // Calling phase_done() without phase() first should not panic
        // in either TTY or non-TTY mode.
        let tty_reporter = ProgressReporter::new_with_tty(false, true);
        tty_reporter.phase_done("Loading... 5 items");

        let non_tty_reporter = ProgressReporter::new_with_tty(false, false);
        non_tty_reporter.phase_done("Loading... 5 items");
    }

    #[test]
    fn test_quiet_mode_phase_and_phase_done_suppressed() {
        // Quiet mode: both phase() and phase_done() should be no-ops
        let tty_reporter = ProgressReporter::new_with_tty(true, true);
        tty_reporter.phase("Loading...");
        tty_reporter.phase_done("Loading... 5 items");

        let non_tty_reporter = ProgressReporter::new_with_tty(true, false);
        non_tty_reporter.phase("Loading...");
        non_tty_reporter.phase_done("Loading... 5 items");
    }

    #[test]
    fn test_multiple_phases_tty_mode() {
        // Multiple phase/phase_done cycles should work on TTY
        let reporter = ProgressReporter::new_with_tty(false, true);
        reporter.phase("Phase 1...");
        reporter.phase_done("Phase 1... done");
        reporter.phase("Phase 2...");
        reporter.phase_done("Phase 2... done");
    }

    #[test]
    fn test_phase_without_phase_done_tty() {
        // Some phases only call phase() without phase_done().
        // The next phase() should overwrite cleanly.
        let reporter = ProgressReporter::new_with_tty(false, true);
        reporter.phase("Loading config...");
        reporter.phase("Loading data files...");
        reporter.phase_done("Loading data files... 6 files");
    }

    #[test]
    fn test_tty_render_progress_has_two_lines() {
        // TTY mode should create both a progress bar and a file line
        let reporter = ProgressReporter::new_with_tty(false, true);
        let progress = reporter.render_progress(100, "Rendering pages");
        assert!(progress.bar.is_some());
        assert!(progress.file_line.is_some());
        assert!(progress._multi.is_some());
        progress.inc("test-page.html");
        assert_eq!(progress.count(), 1);
        progress.finish();
    }

    #[test]
    fn test_non_tty_render_progress_has_no_file_line() {
        // Non-TTY mode should have a bar but no file line (no ANSI cursor tricks)
        let reporter = ProgressReporter::new_with_tty(false, false);
        let progress = reporter.render_progress(100, "Rendering pages");
        assert!(progress.bar.is_some());
        assert!(progress.file_line.is_none());
        assert!(progress._multi.is_none());
        progress.inc("test-page.html");
        assert_eq!(progress.count(), 1);
        progress.finish();
    }

    #[test]
    fn test_tty_render_progress_thread_safe() {
        // Verify two-line progress works correctly with concurrent threads
        let reporter = ProgressReporter::new_with_tty(false, true);
        let progress = reporter.render_progress(100, "Rendering");

        std::thread::scope(|s| {
            for _ in 0..10 {
                s.spawn(|| {
                    for i in 0..10 {
                        progress.inc(&format!("page-{i}.html"));
                    }
                });
            }
        });

        assert_eq!(progress.count(), 100);
        progress.finish();
    }

    #[test]
    fn test_render_progress_finish_clears_both_lines() {
        // Verify finish() does not panic and clears both bars
        let reporter = ProgressReporter::new_with_tty(false, true);
        let progress = reporter.render_progress(10, "Rendering");
        for i in 0..10 {
            progress.inc(&format!("page-{i}.html"));
        }
        progress.finish();
        // After finish, the bars should be cleared (no visual residue)
    }

    #[test]
    fn test_quiet_render_progress_finish_is_noop() {
        // Quiet mode finish should not panic
        let reporter = ProgressReporter::new_with_tty(true, true);
        let progress = reporter.render_progress(10, "Rendering");
        progress.finish();
    }
}
