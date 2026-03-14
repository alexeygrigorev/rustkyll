# Issue 62: Playwright visual screenshot comparison

## Problem

Issues #49 and #57 required Playwright visual comparison of rustkyll vs Jekyll output but this was descoped. We need to verify that the rendered pages look the same in a real browser.

## Goal

Build a Playwright test suite that:
1. Serves Jekyll _site/ on one port, rustkyll _site/ on another (over HTTP so CSS, images, fonts, JS all load)
2. Visits key pages on both servers
3. Takes full-page screenshots
4. Compares screenshots with a pixel diff threshold
5. Verifies no 404 errors in browser console (all assets loading)

## Sites to compare

- DataTalksClub/datatalksclub.github.io
- kids-horror-stories-ru

## Pages to screenshot (at minimum)

- Homepage
- A blog post
- A collection page (e.g., events, books, podcast)
- An archive/listing page
- One other page

## Dependencies

- Node.js and Playwright must be installed

## Acceptance criteria

- Playwright test script exists and is runnable
- Sites are served over HTTP (not just raw file reading) so CSS/images/fonts/JS load
- No 404 errors in browser console for either server
- At least 5 pages screenshotted per site
- Pixel diff threshold defined (<5%)
- All pages pass the visual diff threshold for both sites
- Screenshots saved for review
- Results documented
