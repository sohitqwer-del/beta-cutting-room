# BETA Cutting Room brand files

The BETA mark is a "B" made of three nested pattern pieces: a stem with a
notch and two bowls, cut from a marker strip. Large versions also show a grain
line, a drill hole and the dashed selvedges of the fabric.

Licence: Apache-2.0, like the rest of BETA Cutting Room. You may use these
files to refer to the project, for example in articles, slides or a link to the
download page. Do not use them in a way that suggests that your product is BETA
or that the BETA project endorses you.

## Files

| File | Use |
|---|---|
| `logo-mark.svg` | Main mark: green tile. App icons, avatars, favicons. |
| `logo-mark-colour.svg` | Mark without the tile, in brand colours, for light backgrounds. |
| `logo-mark-mono.svg` | One colour (dark green ink), for stamps, print and fax. |
| `logo-mark-mono-white.svg` | One colour white, for dark backgrounds and photos. |
| `logo-wordmark.svg` | Mark with "BETA Cutting Room", colour. Website and documents. |
| `logo-wordmark-mono.svg` | Wordmark in one dark colour. |
| `logo-wordmark-white.svg` | Wordmark in white, for dark backgrounds. |
| `favicon.svg`, `favicon-32.png` | Browser tab icon. |
| `apple-touch-icon.png` | 180 x 180 home-screen icon. |
| `icon-512.png` | 512 x 512 app icon. |
| `icon-192.png` | 192 x 192 icon for the website manifest. |
| `social-preview.png` | 1280 x 640 image for GitHub (Settings, Social preview). |
| `social-card.png` | 1200 x 630 link preview (Open Graph and X) on every page: a real app screen, the logo and one line. |
| `social-square.png` | 1080 x 1080 image for square posts (LinkedIn, Instagram). |

`social-card.png` and `social-square.png` are made by `node marker/apps/site/scripts/make-social.ts`
(add `--out <folder>` for copies) from `assets/screens/path-marker-1200.webp`, the logo and the tagline in `src/config.ts`.

Keep clear space around the mark of at least a quarter of its width. Do not
stretch, rotate or recolour the pieces. The smallest size is 16 px for the tile
and 20 px high for the wordmark.

## Colours

| Name | Hex | Use |
|---|---|---|
| Cutting green | `#176b52` | Tile, links, buttons (white text on it passes WCAG AA) |
| Ink | `#10231c` | Text, mono mark |
| Amber | `#f4b740` | Upper bowl, stitch lines, badges (use dark text on it) |
| Mint | `#c9ecd9` | Lower bowl |
| Paper | `#fbfaf6` | Page background |

## Type

Headings and the wordmark use **Bricolage Grotesque** (SIL Open Font License
1.1, `../fonts/OFL.txt`). The wordmark letters are outlined, so the SVG files
need no font. Body text uses the system font.

## Rebuild

All files come from `src/brand.ts` and `scripts/make-brand.ts`. The script
also writes the web app favicon and the desktop icons
(`marker/apps/desktop/src-tauri/icons/icon.ico` and `icon.png`). In an empty
folder outside the repository, run
`npm pack @fontsource/bricolage-grotesque@5.3.0` and extract it. Then, from the
repository root:

```sh
node marker/apps/site/scripts/make-brand.ts --font-dir <extracted>/package/files
```

It uses a local Chrome or Edge in headless mode and no npm packages.
