import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { feedbackLinks, site } from './config.ts';
import { easeOutCubic, formatCount, wrapIndex } from './client.ts';
import { clientScript, usesNetwork } from './client-build.ts';
import { MARK, markSvg } from './brand.ts';
import { loadRuns } from './benchmark.ts';
import { layout, pages } from './layout.ts';
import { demoVideo, homePage } from './pages/home.ts';
import { benchmarkPage } from './pages/benchmark.ts';
import { installPage } from './pages/install.ts';
import { docsPage } from './pages/docs.ts';
import { openSourcePage } from './pages/open-source.ts';
import { comparePage } from './pages/compare.ts';
import type { BuildContext } from './pages/context.ts';
import { readHero, heroLive } from './hero-live.ts';
import { topFaq, nestingAnswer } from './faq.ts';

const srcDir = fileURLToPath(new URL('./', import.meta.url));
const dataDir = fileURLToPath(new URL('../data/benchmark/', import.meta.url));
const files = readdirSync(dataDir).filter((f) => f.endsWith('.json')).map((name) => ({ name, text: readFileSync(dataDir + name, 'utf8') }));
const hero = readHero(readFileSync(fileURLToPath(new URL('../data/hero-marker.json', import.meta.url)), 'utf8'));
const ctx: BuildContext = { runs: loadRuns(files).runs, screens: new Set(['marker-editor.png', 'style-pattern.png', 'pattern.png', 'cut-plan.png', 'rolls.png', 'spreading.png', 'bundles.png']), hero };
const home = homePage(ctx).value;

describe('config', () => {
  it('has a complete portable ZIP download', () => {
    const p = site.downloads.portable;
    // The lead updates the packages at release; URL and file name must name the same version.
    const m = /^https:\/\/github\.com\/.+\/releases\/download\/v(\d+\.\d+\.\d+)\/BETA-Cutting-Room_(\d+\.\d+\.\d+)_windows-x64-portable\.zip$/.exec(p.url ?? '');
    expect(m).not.toBeNull();
    expect(m![1]).toBe(m![2]);
    expect(p.url!.endsWith(p.file)).toBe(true);
    expect(p.size).toMatch(/^\d+(\.\d+)? MiB$/);
    expect(p.sha256).toMatch(/^[0-9a-f]{64}$/);
  });

  it('gives every download a size', () => {
    for (const d of Object.values(site.downloads)) {
      if (d.url) expect(d.size).toMatch(/MiB$/);
      else expect(d.sha256).toBeNull();
    }
  });

  it('points feedback links at labelled GitHub issues and the contact address', () => {
    expect(site.feedback.bug).toBe('https://github.com/sohitqwer-del/beta-cutting-room/issues/new?labels=bug');
    expect(site.feedback.missing).toBe('https://github.com/sohitqwer-del/beta-cutting-room/issues/new?labels=missing-feature');
    expect(site.feedback.idea).toBe('https://github.com/sohitqwer-del/beta-cutting-room/issues/new?labels=feedback');
    expect(site.feedback.email).toBe(`mailto:${site.contactEmail}`);
  });

  it('lists the three feedback links in order and skips unset ones', () => {
    expect(feedbackLinks().map((l) => l.label)).toEqual(['Report a bug', 'Tell us what is missing', 'Share feedback']);
    const saved = site.feedback.missing;
    site.feedback.missing = null;
    try { expect(feedbackLinks().map((l) => l.key)).toEqual(['bug', 'idea']); } finally { site.feedback.missing = saved; }
  });

  it('has an https site address for social previews', () => {
    expect(site.siteUrl).toMatch(/^https:\/\/.+\/$/);
  });
});

describe('motion helpers', () => {
  it('eases from 0 to 1 and clamps', () => {
    expect(easeOutCubic(0)).toBe(0);
    expect(easeOutCubic(1)).toBe(1);
    expect(easeOutCubic(2)).toBe(1);
    expect(easeOutCubic(-1)).toBe(0);
    expect(easeOutCubic(0.5)).toBeCloseTo(0.875);
  });

  it('formats counters like the build: real minus sign, optional plus, no negative zero', () => {
    expect(formatCount(-0.7296, 2, true)).toBe('−0.73');
    expect(formatCount(0.4208, 2, true)).toBe('+0.42');
    expect(formatCount(0.4208, 2, false)).toBe('0.42');
    expect(formatCount(-0.001, 2, true)).toBe('0.00');
    expect(formatCount(300, 0, false)).toBe('300');
  });

  it('wraps carousel indexes both ways', () => {
    expect(wrapIndex(5, 1, 6)).toBe(0);
    expect(wrapIndex(0, -1, 6)).toBe(5);
    expect(wrapIndex(2, 1, 6)).toBe(3);
    expect(wrapIndex(0, 1, 0)).toBe(0);
  });
});

describe('browser script', () => {
  const js = clientScript(readFileSync(srcDir + 'client.ts', 'utf8'));

  it('becomes a classic script with no exports, imports or network calls', () => {
    expect(js).not.toMatch(/^\s*(export|import)\s/m);
    expect(usesNetwork(js)).toBe(false);
    expect(() => new Function(js)).not.toThrow();
  });

  it('refuses a source that imports something', () => {
    expect(() => clientScript("import x from 'y';\nexport const a = 1;")).toThrow(/must not import/);
  });

  it('detects network APIs', () => {
    expect(usesNetwork('fetch("/x")')).toBe(true);
    expect(usesNetwork('navigator.sendBeacon(u)')).toBe(true);
    expect(usesNetwork('const fetcher = 1')).toBe(false);
  });
});

describe('brand mark', () => {
  it('draws the tile, the three pieces and optional details', () => {
    const tile = markSvg({ variant: 'tile', detail: true, standalone: true, label: 'Logo' });
    expect(tile).toContain('xmlns="http://www.w3.org/2000/svg"');
    expect(tile).toContain('role="img" aria-label="Logo"');
    for (const d of [MARK.stem, MARK.upper, MARK.lower, MARK.grain, MARK.selvedge]) expect(tile).toContain(d);
  });

  it('is decorative without a label and has no tile in mono', () => {
    const mono = markSvg({ variant: 'mono' });
    expect(mono).toContain('aria-hidden="true"');
    expect(mono).not.toContain('<rect');
    expect(mono).not.toContain(MARK.grain);
  });

  it('ships the same mark as the site favicon', () => {
    const fav = readFileSync(fileURLToPath(new URL('../public/assets/favicon.svg', import.meta.url)), 'utf8');
    expect(fav).toContain(MARK.stem);
  });
});

describe('home page', () => {
  it('links the browser app with clear guidance on job size', () => {
    for (const page of [home, installPage().value]) {
      expect(page).toContain(`href="${site.webAppUrl}"`);
      expect(page).toMatch(/open the app in your browser/i);
      expect(page).toContain('Best for trying and small jobs; for big markers use the desktop app');
    }
  });

  it('says in one line what BETA is and who it is for, with Try and Download first', () => {
    const hero = home.slice(home.indexOf('class="hero'), home.indexOf('id="paths"'));
    expect(hero).toContain('<h1 class="hero-title">Free, open-source <span class="hl"><span class="nowrap">cutting-room</span> software</span> for garment factories</h1>');
    expect(hero).toContain('Patterns, markers and cut plans, in your browser.');
    expect(hero.indexOf('Try it now, no install')).toBeGreaterThan(-1);
    expect(hero).toContain(`href="${site.tourUrl}"`);
    expect(hero).toContain('href="#download"');
    expect(site.tourUrl).toBe('app/#/tour');
  });

  it('says it is a fun AI-assisted project and asks for help', () => {
    expect(home).toContain('A fun project, built with modern AI tools');
    expect(home).toContain('A fun project, built with modern AI tools: we need your help');
    expect(home).toContain('BETA is young and not perfect yet.');
    expect(home).toContain('id="help"');
  });

  it('offers three paths into the real app, each with a screenshot and three points', () => {
    const block = home.slice(home.indexOf('id="paths"'), home.indexOf('id="features"'));
    for (const [id, href] of [['path-paper', 'app/#/quick'], ['path-pattern', 'app/#/style'], ['path-room', 'app/#/marker']]) {
      const card = block.slice(block.indexOf(`id="${id}"`), block.indexOf('</article>', block.indexOf(`id="${id}"`)));
      expect(card).toContain(`href="${href}"`);
      expect(card.match(/<li>/g)?.length).toBe(3);
      expect(card).toMatch(/<img [^>]*src="assets\/screens\/path-[a-z]+-\d+\.webp"/);
    }
  });

  it('groups the features and marks what is new in 0.8, with an honest not-yet list', () => {
    const block = home.slice(home.indexOf('id="features"'), home.indexOf('id="benchmark"'));
    for (const g of ['Pattern', 'Marker', 'Cutting room', 'Small factories']) expect(block).toContain(`<h3>${g}</h3>`);
    expect(block.match(/New in 0\.8/g)?.length).toBeGreaterThanOrEqual(4);
    expect(block).toContain('Not yet, or not proven');
  });

  it('answers the top eight questions, with nesting numbers from the data', () => {
    const faq = topFaq(ctx.runs);
    expect(faq.length).toBe(8);
    expect(home).toContain('id="faq"');
    const answer = nestingAnswer(ctx.runs).value;
    expect(answer).toContain('1.39 points less efficient on average, median 1.57 points less efficient; 77 wins, 12 ties, 211 losses');
    expect(answer).toContain('0.42 points more efficient on average, median 0.30 points less efficient; 25 wins, 3 ties, 32 losses');
  });

  it('links available downloads and every feedback channel', () => {
    for (const d of Object.values(site.downloads)) if (d.url) expect(home).toContain(`href="${d.url}"`);
    for (const l of feedbackLinks()) expect(home).toContain(`href="${l.url}"`);
    expect(home).toContain(`href="${site.feedback.email}"`);
  });

  it('shows mean, median and win/tie/loss together, from the data, and claims no win', () => {
    const bench = home.slice(home.indexOf('id="benchmark"'), home.indexOf('id="privacy"'));
    expect(bench).toMatch(/\+0\.42<\/span><small> pp<\/small>[\s\S]*Mean difference/);
    expect(bench).toContain('−0.30');
    expect(bench).toContain('25 wins');
    expect(bench).toContain('32 losses');
    expect(bench).toContain('−1.39');
    expect(bench).toContain('−1.57');
    expect(bench).toMatch(/77 wins[\s\S]*12 ties[\s\S]*211 losses/);
    expect(bench).toContain('No general win over Lectra.');
    expect(bench).toContain('Folded fabric is still behind open fabric');
    expect(home).not.toMatch(/BETA (beats|outperforms|is better than)/i);
  });

  it('keeps final numbers in the HTML for counters (works without JavaScript)', () => {
    expect(home).toMatch(/data-count="-1\.391[0-9]*" data-decimals="2" data-signed="1">−1\.39</);
  });

  it('gives every screenshot alt text, a fixed size (no layout shift), small WebP sources and lazy loading', () => {
    const imgs = home.match(/<img [^>]+>/g) ?? [];
    expect(imgs.length).toBe(3);
    for (const img of imgs) {
      expect(img).toMatch(/alt="[A-Z][^"]{20,}"/);
      expect(img).toMatch(/width="\d+" height="\d+"/);
      expect(img).toContain('loading="lazy"');
      expect(img).toMatch(/srcset="[^"]+\.webp \d+w, [^"]+\.webp \d+w"/);
      for (const src of img.match(/assets\/screens\/[a-z0-9-]+\.webp/g) ?? []) expect(readFileSync(fileURLToPath(new URL(`../public/${src}`, import.meta.url))).length).toBeLessThan(80_000);
    }
  });
});

describe('demo video', () => {
  it('plays the self-hosted file with captions, no autoplay and no preload', () => {
    const v = demoVideo({ ...ctx, video: true }).value;
    expect(v).toContain('id="demo"');
    expect(v).toContain('Watch the demo');
    expect(v).toContain(site.demoVideo.length);
    expect(v).toMatch(/<video [^>]*controls[^>]*preload="none"/);
    expect(v.match(/<video [^>]*>/)![0]).not.toMatch(/autoplay|muted|loop/);
    expect(v).toContain('<source src="assets/video/beta-08-walkthrough.mp4" type="video/mp4">');
    expect(v).toContain('<track kind="captions" srclang="en" label="English" src="assets/video/beta-08-walkthrough.vtt" default>');
    expect(v).toContain('poster="assets/video/beta-08-poster-1280x720.png"');
  });

  it('links to a remote file without loading it, and hides when there is no video', () => {
    expect(demoVideo({ ...ctx, video: false }).value).toBe('');
    const saved = site.demoVideo.demoVideoUrl;
    site.demoVideo.demoVideoUrl = 'https://github.com/example/releases/download/v1/beta-demo-en.mp4';
    try {
      const v = demoVideo({ ...ctx, video: false }).value;
      expect(v).toContain('href="https://github.com/example/releases/download/v1/beta-demo-en.mp4"');
      expect(v).not.toContain('<video');
      expect(v).toMatch(/<img src="assets\/video\/beta-08-poster-1280x720\.png" alt="[^"]+"/);
    } finally { site.demoVideo.demoVideoUrl = saved; }
  });

  it('sits on the home page right after the hero when the file exists', () => {
    const page = homePage({ ...ctx, video: true }).value;
    expect(page.indexOf('id="demo"')).toBeGreaterThan(page.indexOf('class="hero"'));
    expect(page.indexOf('id="demo"')).toBeLessThan(page.indexOf('id="paths"'));
  });
});

describe('all pages', () => {
  const rendered: Record<string, string> = {
    'index.html': home, 'benchmark.html': benchmarkPage(ctx).value, 'compare.html': comparePage().value,
    'install.html': installPage().value, 'docs.html': docsPage(true).value, 'open-source.html': openSourcePage().value,
  };

  it('share a light, accessible layout with feedback links in the footer', () => {
    for (const meta of pages) {
      const doc = layout(meta, { value: rendered[meta.file]!, toString: () => rendered[meta.file]! } as never);
      expect(doc).toContain('<html lang="en">');
      expect(doc).toContain('<meta name="color-scheme" content="light">');
      expect(doc).toContain('class="skip-link" href="#main"');
      expect(doc).toContain('<main id="main"');
      expect(doc).toContain('Report a bug');
      expect(doc).toContain('Tell us what is missing');
      expect(doc).toContain('A fun project, built with the help of modern AI tools.');
      expect(doc).toContain(`<meta property="og:image" content="${site.siteUrl}assets/brand/social-card.png">`);
      expect(doc).toContain('<meta property="og:image:width" content="1200">');
      expect(doc).toContain('<meta property="og:image:height" content="630">');
      expect(doc).toContain(`<meta property="og:title" content="${meta.title}">`);
      expect(doc).toContain('<meta name="twitter:card" content="summary_large_image">');
      expect(doc).toContain(`<meta name="twitter:image" content="${site.siteUrl}assets/brand/social-card.png">`);
      expect(doc).toContain('<link rel="manifest" href="site.webmanifest">');
      expect(doc).toMatch(/<meta property="og:url" content="https:\/\//);
      expect(doc).not.toMatch(/<(script|link|img)\b[^>]*\b(src|href)="(https?:)?\/\//);
      expect(doc).not.toContain('theme-toggle');
    }
  });

  it('install page explains the portable ZIP with its checksum', () => {
    const install = rendered['install.html']!;
    expect(install).toContain('id="portable"');
    for (const download of Object.values(site.downloads)) if (download.sha256) expect(install).toContain(download.sha256!);
    expect(install).toContain(`'.\\${site.downloads.portable.file}'`);
  });
});

describe('stylesheet', () => {
  const css = readFileSync(srcDir + 'style.css', 'utf8');

  it('only animates under .motion, so reduced motion and no-JS show the final state', () => {
    const rules = css.replace(/\/\*[\s\S]*?\*\//g, '').split('}');
    const offenders = rules.filter((r) => /(^|[;{\s])animation\s*:/.test(r)).map((r) => r.slice(0, r.indexOf('{')).trim()).filter((sel) => !sel.split(',').every((s) => s.includes('.motion')));
    expect(offenders).toEqual([]);
  });

  it('has a reduced-motion override and loads no remote resources', () => {
    expect(css).toContain('@media (prefers-reduced-motion: reduce)');
    expect(css).not.toMatch(/url\(\s*["']?(https?:)?\/\//);
    expect(css).not.toMatch(/@import\s/);
  });
});

describe('hero: the recorded live marker', () => {
  it('holds real markers that get shorter, ending at the final result', () => {
    expect(hero.frames.length).toBeGreaterThanOrEqual(3);
    expect(hero.sizes).toEqual(['S', 'M', 'L', 'XL']);
    for (let i = 1; i < hero.frames.length; i++) expect(hero.frames[i]!.length_mm).toBeLessThanOrEqual(hero.frames[i - 1]!.length_mm);
    expect(hero.frames.every((f) => f.pieces.length === hero.frames[0]!.pieces.length)).toBe(true);
    expect(JSON.stringify(hero).length).toBeLessThan(40_000);
  });
  it('rejects a broken recording', () => {
    expect(() => readHero('{"frames":[],"width_mm":1}')).toThrow(/no frames/);
    expect(() => readHero(JSON.stringify({ ...hero, frames: [...hero.frames].reverse() }))).toThrow(/shorter/);
  });
  it('draws every frame, shows the final one without motion, and describes it in text', () => {
    const svg = heroLive(hero).value;
    expect(svg.match(/class="hz-frame/g)?.length).toBe(hero.frames.length);
    expect(svg.match(/class="hz-frame is-final"/g)?.length).toBe(1);
    expect(svg).toContain('role="img"');
    expect(svg).toMatch(/<desc id="hz-desc">Sizes S, M, L, XL on fabric 152\.4 cm wide/);
    expect(home).toContain('data-hero-live');
    expect(home).toContain('Recorded in the browser app');
  });
  it('can be paused (motion longer than 5 seconds)', () => {
    expect(home).toContain('data-hz-toggle');
    const js = clientScript(readFileSync(srcDir + 'client.ts', 'utf8'));
    expect(js).toContain('data-hero-live');
    expect(js).toContain('Pause the animation');
  });
});

describe('social cards', () => {
  it('ships the 1200 x 630 card and the 1080 x 1080 square as PNG files', () => {
    for (const [name, w, h] of [['social-card.png', 1200, 630], ['social-square.png', 1080, 1080]] as const) {
      const png = readFileSync(fileURLToPath(new URL(`../public/assets/brand/${name}`, import.meta.url)));
      expect(png.subarray(1, 4).toString()).toBe('PNG');
      expect(png.readUInt32BE(16)).toBe(w);
      expect(png.readUInt32BE(20)).toBe(h);
      expect(png.length).toBeLessThan(400_000);
    }
  });
});
