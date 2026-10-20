# BETA Cutting Room website (`@beta/marker-site`)

The public launch website: Home, Benchmark, Compare, Install, Help and Open source.
It is a static site. Pages are rendered to plain HTML at build time, so `dist/` works
when opened from disk (`dist/index.html`), from a USB drive, or from any static host.

- No trackers, no cookies, no CDN, no runtime network calls. One self-hosted OFL font
  (Bricolage Grotesque, `public/assets/fonts/OFL.txt`) for headings.
- Light theme only. Brand files and their rules: `public/assets/brand/README.md`.
- JavaScript: one tiny inline script in the head and `assets/site.js`, built from
  `src/client.ts` (scroll reveals, counters, workflow steps, screenshot slideshow).
  Without JavaScript every section is still shown.
- The build fails if a page tries to load anything from another server, or if the
  script uses a network API.

## Motion

All animations use only `transform` and `opacity` and run under the `.motion` class.
The head script sets that class only when the visitor has not asked for reduced
motion. With `prefers-reduced-motion: reduce`, or without JavaScript, there is no
motion and every element shows in its final place: the hero marker is already
nested, all workflow steps are lit, counters show their final value and the
screenshots show as a grid without a slideshow. If `assets/site.js` does not run
within 2.5 s, the head script removes `.motion` again, so content is never hidden.
The slideshow has a pause button and pauses on hover and keyboard focus. Other
animations run once and finish within about 5 seconds; the hero and the workflow
have a Replay button. A test checks that every CSS animation is under `.motion`.

## Build

From the repository root:

```sh
npm install
npm run build -w @beta/marker-site     # typecheck + render into marker/apps/site/dist/
npx vitest run marker/apps/site         # loader tests, also validates every data file
```

Node.js 22.18 or newer runs the TypeScript build script directly (type stripping).
There are no runtime npm dependencies; TypeScript and Vitest come from the root workspace.

## What to fill in before release

| What | Where |
|---|---|
| Download links, file size, SHA-256 (installer, portable ZIP, Linux server) | `src/config.ts` → `downloads` |
| Feedback links (bug, missing feature, feedback, email) | `src/config.ts` → `feedback` |
| Public address of the site (social previews) | `src/config.ts` → `siteUrl` |
| Public repository URL | `src/config.ts` → `repoUrl` (turns document paths into links) |
| Contact address | `src/config.ts` → `contactEmail` |
| Real screenshots | `public/assets/screens/*.png` (names in that folder's README) |
| Final benchmark data | `data/benchmark/*.json` (see below) |

`null` values show "Coming soon" or a placeholder line. Rebuild after each change.

## Benchmark data

The Benchmark page reads every `data/benchmark/*.json` file. The format is
`beta-site-benchmark/1`, documented in [`data/benchmark/SCHEMA.md`](data/benchmark/SCHEMA.md)
and checked by `src/benchmark.ts`.

The files shipped now are **sample data** (`"status": "sample"`), copied from the
documented results in `marker/docs/benchmarks.md` and `marker/bench/results/` by
`scripts/make-sample-data.py`. The page labels them as sample data. When the benchmark
runner exports a `final` file with the same `id`, it replaces the sample. A `provisional`
file (an unfinished run) is shown next to the sample with a "Provisional" badge and its
progress. Invalid files stop the build.

Never put geometry, factory data, buyer names or styles in these files. The loader
rejects geometry keys.

## Screenshots of the site

```sh
node marker/apps/site/scripts/screenshots.mjs --out <folder outside git>
```

Uses a local Chrome or Edge in headless mode (DevTools protocol, no npm packages).
It scrolls through every page so the reveals run, captures it at 1440 px and 390 px
wide, adds the home page with reduced motion, and writes `report.json`. It fails if
a page makes any non-local request or logs an error.

## Layout

| Path | Contents |
|---|---|
| `src/build.ts` | Build script: loads data, renders pages, copies assets, checks for remote loads |
| `src/brand.ts` | Logo geometry and colours (one source for site, app and icons) |
| `src/client.ts` | Browser motion script (types stripped at build by `src/client-build.ts`) |
| `scripts/make-brand.ts` | Writes all brand files, favicons, desktop icons and the social preview |
| `src/benchmark.ts` | Benchmark JSON loader and validator (pure functions) |
| `src/charts.ts` | Plain SVG charts (diverging bars, dot plot, bars); values are also text |
| `src/illustrations.ts` | Original illustrative SVGs and screenshot placeholders |
| `src/pages/*.ts` | One file per page |
| `src/style.css` | All styles (light theme) and all motion |
| `data/benchmark/` | Benchmark data and its schema |
| `public/` | Files copied as they are (brand, font, favicon, screenshots) |

Competitor facts on the Compare page come from `marker/docs/V04-RESEARCH.md` and
`marker/docs/PLAN.md`. Keep the "vendor documents" / "not verified" wording when you
change them, and add the source.
