import { test, expect, Page } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';
import { PNG } from 'pngjs';
// pixelmatch v6 is ESM-only; when loaded via CJS (Playwright's default),
// the default export ends up on .default.
// eslint-disable-next-line @typescript-eslint/no-require-imports
const pixelmatchModule = require('pixelmatch');
const pixelmatch: typeof import('pixelmatch').default =
  pixelmatchModule.default || pixelmatchModule;

// Configuration from environment
const JEKYLL_URL = process.env.JEKYLL_URL || 'http://localhost:4100';
const RUSTKYLL_URL = process.env.RUSTKYLL_URL || 'http://localhost:4101';
const SCREENSHOT_DIR = process.env.SCREENSHOT_DIR || path.join(__dirname, '..', 'screenshots');
const DIFF_THRESHOLD = parseFloat(process.env.DIFF_THRESHOLD || '0.0');
const SITE_NAME = process.env.SITE_NAME || 'unknown';

// Page definitions per site
interface PageDef {
  name: string;
  path: string;
}

const DTC_PAGES: PageDef[] = [
  // Listing/index pages (1-11)
  { name: 'homepage', path: '/' },
  { name: 'articles-listing', path: '/articles.html' },
  { name: 'books-listing', path: '/books.html' },
  { name: 'podcast-listing', path: '/podcast.html' },
  { name: 'events-listing', path: '/events.html' },
  { name: 'courses-listing', path: '/courses.html' },
  { name: 'people-listing', path: '/people.html' },
  { name: 'support', path: '/support.html' },
  { name: 'tools', path: '/tools.html' },
  { name: 'slack-community', path: '/slack.html' },
  { name: 'slack-guidelines', path: '/slack/guidelines.html' },
  // Blog posts (12-14)
  { name: 'blog-segmentation', path: '/blog/segmentation.html' },
  { name: 'blog-practical-guide', path: '/blog/practical-guide-better-code.html' },
  { name: 'blog-data-roles', path: '/blog/data-roles.html' },
  // Book detail pages (15-16)
  { name: 'book-ml-bookcamp', path: '/books/20201214-ml-bookcamp.html' },
  { name: 'book-reinforcement-learning', path: '/books/20210111-reinforcement-learning.html' },
  // Podcast episode pages (17-18)
  { name: 'podcast-ab-testing', path: '/podcast/ab-testing-and-product-experimentation.html' },
  { name: 'podcast-ai-ecology', path: '/podcast/ai-for-ecology-biodiversity-and-conservation.html' },
  // People detail pages (19-20)
  { name: 'person-alexeygrigorev', path: '/people/alexeygrigorev.html' },
  { name: 'person-aaishamuhammad', path: '/people/aaishamuhammad.html' },
  // Course page (21)
  { name: 'course-ml-zoomcamp', path: '/courses/2021-winter-ml-zoomcamp.html' },
  // Conference page (22)
  { name: 'conference-2021-feb', path: '/conferences/2021-feb.html' },
];

const KIDS_PAGES: PageDef[] = [
  { name: 'homepage', path: '/' },
  { name: 'story-orchid', path: '/stories/001-orchid/' },
  { name: 'story-silkworm', path: '/stories/002-silkworm-curse/' },
  { name: 'story-toy', path: '/stories/003-childhood-toy/' },
];

const MOJOMBO_PAGES: PageDef[] = [
  { name: 'homepage', path: '/' },
  { name: 'post-blogging-hacker', path: '/2008/11/17/blogging-like-a-hacker.html' },
  { name: 'post-git-parable', path: '/2009/05/19/the-git-parable.html' },
  { name: 'post-readme-driven', path: '/2010/08/23/readme-driven-development.html' },
  { name: 'post-open-source', path: '/2011/11/22/open-source-everything.html' },
];

const JUST_THE_DOCS_PAGES: PageDef[] = [
  { name: 'homepage', path: '/' },
  { name: 'configuration', path: '/docs/configuration/' },
  { name: 'customization', path: '/docs/customization/' },
  { name: 'navigation', path: '/docs/navigation/' },
  { name: 'search', path: '/docs/search/' },
];

// GitHub Pages theme sites all have the same structure: index + another-page
const GITHUB_THEME_PAGES: PageDef[] = [
  { name: 'homepage', path: '/' },
  { name: 'another-page', path: '/another-page.html' },
];

function getPagesForSite(siteName: string): PageDef[] {
  if (siteName.includes('datatalksclub')) {
    return DTC_PAGES;
  }
  if (siteName.includes('kids-horror')) {
    return KIDS_PAGES;
  }
  if (siteName.includes('mojombo')) {
    return MOJOMBO_PAGES;
  }
  if (siteName.includes('just-the-docs')) {
    return JUST_THE_DOCS_PAGES;
  }
  // GitHub Pages theme sites
  if (siteName.match(/cayman|slate|leap-day|midnight|hacker|architect|time-machine|merlot|dinky/)) {
    return GITHUB_THEME_PAGES;
  }
  // Fallback: just test homepage
  return [{ name: 'homepage', path: '/' }];
}

interface NetworkError {
  url: string;
  status: number;
}

/**
 * Seed Math.random() deterministically so that pages with random JS
 * (e.g., random contribution graphs) produce identical output across
 * separate page loads. Uses a simple mulberry32 PRNG seeded with a
 * fixed value.
 */
async function seedMathRandom(page: Page): Promise<void> {
  await page.addInitScript(() => {
    // mulberry32 PRNG -- deterministic, fast, good distribution
    let seed = 42;
    Math.random = function () {
      seed |= 0;
      seed = (seed + 0x6d2b79f5) | 0;
      let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  });
}

/**
 * Visit a page and collect console errors and network 404s.
 * Returns the list of 404 errors found.
 */
async function visitAndCollectErrors(page: Page, url: string): Promise<NetworkError[]> {
  const errors: NetworkError[] = [];

  page.on('response', (response) => {
    if (response.status() === 404) {
      errors.push({ url: response.url(), status: 404 });
    }
  });

  await page.goto(url, { waitUntil: 'networkidle', timeout: 30_000 });

  // Wait a bit for any lazy-loaded resources
  await page.waitForTimeout(1000);

  return errors;
}

/**
 * Take a full-page screenshot and return the file path.
 */
async function takeFullPageScreenshot(page: Page, filePath: string): Promise<void> {
  await page.screenshot({
    path: filePath,
    fullPage: true,
  });
}

/**
 * Compare two PNG screenshots using pixelmatch.
 * Returns { diffRatio, diffPixels, totalPixels, diffImagePath }.
 */
function compareScreenshots(
  jekyllPath: string,
  rustkyllPath: string,
  diffPath: string
): { diffRatio: number; diffPixels: number; totalPixels: number } {
  const jekyllImg = PNG.sync.read(fs.readFileSync(jekyllPath));
  const rustkyllImg = PNG.sync.read(fs.readFileSync(rustkyllPath));

  // Use the larger dimensions to handle different page heights
  const width = Math.max(jekyllImg.width, rustkyllImg.width);
  const height = Math.max(jekyllImg.height, rustkyllImg.height);

  // Create canvases of equal size, filling extra space with white
  const normalizeImage = (img: PNG, w: number, h: number): Buffer => {
    const buf = Buffer.alloc(w * h * 4, 255); // white background
    for (let y = 0; y < img.height; y++) {
      for (let x = 0; x < img.width; x++) {
        const srcIdx = (y * img.width + x) * 4;
        const dstIdx = (y * w + x) * 4;
        buf[dstIdx] = img.data[srcIdx];
        buf[dstIdx + 1] = img.data[srcIdx + 1];
        buf[dstIdx + 2] = img.data[srcIdx + 2];
        buf[dstIdx + 3] = img.data[srcIdx + 3];
      }
    }
    return buf;
  };

  const img1Data = normalizeImage(jekyllImg, width, height);
  const img2Data = normalizeImage(rustkyllImg, width, height);

  const diff = new PNG({ width, height });
  const diffPixels = pixelmatch(img1Data, img2Data, diff.data, width, height, {
    threshold: 0.15,
  });

  fs.writeFileSync(diffPath, PNG.sync.write(diff));

  const totalPixels = width * height;
  const diffRatio = totalPixels > 0 ? diffPixels / totalPixels : 0;

  return { diffRatio, diffPixels, totalPixels };
}

// Ensure screenshot directory exists
fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });

const pages = getPagesForSite(SITE_NAME);

for (const pageDef of pages) {
  test(`visual compare: ${SITE_NAME} - ${pageDef.name}`, async ({ page }) => {
    const safeName = pageDef.name.replace(/[^a-zA-Z0-9_-]/g, '_');
    const safeSite = SITE_NAME.replace(/[^a-zA-Z0-9_-]/g, '_');
    const prefix = `${safeSite}__${safeName}`;

    const jekyllScreenshot = path.join(SCREENSHOT_DIR, `${prefix}__jekyll.png`);
    const rustkyllScreenshot = path.join(SCREENSHOT_DIR, `${prefix}__rustkyll.png`);
    const diffImage = path.join(SCREENSHOT_DIR, `${prefix}__diff.png`);

    // Seed Math.random() deterministically so both Jekyll and rustkyll
    // page loads produce identical random output (e.g., contribution graphs)
    await seedMathRandom(page);

    // Visit Jekyll page
    const jekyllUrl = `${JEKYLL_URL}${pageDef.path}`;
    console.log(`Visiting Jekyll: ${jekyllUrl}`);
    const jekyllErrors = await visitAndCollectErrors(page, jekyllUrl);

    // Check for 404 errors on Jekyll side (informational, not a hard fail)
    if (jekyllErrors.length > 0) {
      console.log(`Jekyll 404s on ${pageDef.name}:`);
      for (const err of jekyllErrors) {
        console.log(`  404: ${err.url}`);
      }
    }

    await takeFullPageScreenshot(page, jekyllScreenshot);
    expect(fs.statSync(jekyllScreenshot).size).toBeGreaterThan(1024);

    // Visit rustkyll page
    const rustkyllUrl = `${RUSTKYLL_URL}${pageDef.path}`;
    console.log(`Visiting rustkyll: ${rustkyllUrl}`);
    const rustkyllErrors = await visitAndCollectErrors(page, rustkyllUrl);

    // 404 errors on rustkyll side are a hard fail
    if (rustkyllErrors.length > 0) {
      // Filter out errors that also occur on Jekyll side (known missing assets)
      const jekyllErrorUrls = new Set(
        jekyllErrors.map((e) => e.url.replace(JEKYLL_URL, ''))
      );
      const rustkyllOnlyErrors = rustkyllErrors.filter(
        (e) => !jekyllErrorUrls.has(e.url.replace(RUSTKYLL_URL, ''))
      );

      if (rustkyllOnlyErrors.length > 0) {
        const msg = rustkyllOnlyErrors.map((e) => `404: ${e.url}`).join('\n');
        console.error(`Rustkyll-only 404s on ${pageDef.name}:\n${msg}`);
        // Fail the test for rustkyll-only 404s
        expect(
          rustkyllOnlyErrors.length,
          `Rustkyll has ${rustkyllOnlyErrors.length} asset 404 errors not present in Jekyll:\n${msg}`
        ).toBe(0);
      }
    }

    await takeFullPageScreenshot(page, rustkyllScreenshot);
    expect(fs.statSync(rustkyllScreenshot).size).toBeGreaterThan(1024);

    // Compare screenshots
    const result = compareScreenshots(jekyllScreenshot, rustkyllScreenshot, diffImage);

    console.log(
      `${pageDef.name}: diff=${(result.diffRatio * 100).toFixed(2)}% ` +
        `(${result.diffPixels}/${result.totalPixels} pixels)`
    );

    // Check threshold
    expect(
      result.diffRatio,
      `Page "${pageDef.name}" pixel diff ${(result.diffRatio * 100).toFixed(2)}% ` +
        `exceeds threshold ${(DIFF_THRESHOLD * 100).toFixed(2)}%. ` +
        `See diff image: ${diffImage}`
    ).toBeLessThanOrEqual(DIFF_THRESHOLD);
  });
}
