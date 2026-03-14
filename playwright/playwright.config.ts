import { defineConfig } from '@playwright/test';

/**
 * Playwright config for visual comparison between Jekyll and rustkyll.
 *
 * Servers are started externally by scripts/visual-compare.sh.
 * Base URLs are passed via environment variables:
 *   JEKYLL_URL  - e.g. http://localhost:4100
 *   RUSTKYLL_URL - e.g. http://localhost:4101
 *   SCREENSHOT_DIR - output directory for screenshots and diffs
 *   DIFF_THRESHOLD - max pixel diff ratio (0.0-1.0), default 0.05
 */
export default defineConfig({
  testDir: './tests',
  timeout: 60_000,
  expect: {
    timeout: 10_000,
  },
  fullyParallel: false,
  retries: 0,
  workers: 1,
  reporter: [['list']],
  use: {
    viewport: { width: 1280, height: 720 },
    screenshot: 'off',
    trace: 'off',
  },
  projects: [
    {
      name: 'chromium-desktop',
      use: {
        browserName: 'chromium',
        viewport: { width: 1280, height: 720 },
      },
    },
  ],
});
