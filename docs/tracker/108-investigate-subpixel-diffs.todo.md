# Issue 108: Investigate "sub-pixel" differences (3-15K pixels)

## Problem

5 pages classified as "sub-pixel font rendering noise" but some have significant pixel counts:
- /support.html: 3 pixels (likely real noise)
- /blog/segmentation.html: 13 pixels (likely real noise)
- /blog/data-roles.html: 3,847 pixels — NOT just noise, investigate
- /people/alexeygrigorev.html: 51 pixels (may be real noise)
- /books/20210111-reinforcement-learning.html: 15,088 pixels — NOT just noise, investigate

## Goal

Investigate each page, identify the actual cause of pixel differences. Fix real differences, confirm actual noise cases.

## Acceptance criteria

- Each of the 5 pages investigated with diff image inspection
- Real differences fixed (not dismissed as noise)
- After fixes, all 5 pages at 0% pixel diff or documented as genuine sub-pixel noise (<10 pixels)
