import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { feedbackLinks, site } from './config.ts';
import { easeOutCubic, formatCount, wrapIndex } from './client.ts';
import { clientScript, usesNetwork } from './client-build.ts';
import { MARK, markSvg } from './brand.ts';
import { loadRuns } from './benchmark.ts';
import { layout, pages } from './layout.ts';
import { homePage } from './pages/home.ts';
import { benchmarkPage } from './pages/benchmark.ts';
import { installPage } from './pages/install.ts';
import { docsPage } from './pages/docs.ts';
import { openSourcePage } from './pages/open-source.ts';
import { comparePage } from './pages/compare.ts';
import type { BuildContext } from './pages/context.ts';

const srcDir = fileURLToPath(new URL('./', import.meta.url));
const dataDir = fileURLToPath(new URL('../data/benchmark/', import.meta.url));
const files = readdirSync(dataDir).filter((f) => f.endsWith('.json')).map((name) => ({ name, text: readFileSync(dataDir + name, 'utf8') }));
const ctx: BuildContext = { runs: loadRuns(files).runs, screens: new Set(['marker-editor.png', 'pattern.png', 'cut-plan.png', 'rolls.png', 'spreading.png', 'bundles.png']) };
const home = homePage(ctx).value;

describe('config', () => {
  it('has a complete portable ZIP download', () => {
    const p = site.downloads.portable;
    expect(p.url).toMatch(/^https:\/\/github\.com\/.+\/releases\/download\/v0\.4\.0\/BETA-Cutting-Room_0\.4\.0_windows-x64-portable\.zip$/);
    expect(p.url!.endsWith(p.file)).toBe(true);
    expect(p.size).toMatch(/^\d+(\.\d+)? MiB$/);
    expect(p.sha256).toMatch(/^[0-9a-f]{64}$/);
  });

  it('gives every download a size', () => {
    for (const d of Object.values(site.downloads)) expect(d.size).toMatch(/MiB$/);
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
  it('says what BETA is, that it is a fun AI-assisted project, and asks for help', () => {
    expect(home).toContain('cutting-room</span> software</span> for garment factories');
    expect(home).toContain('A fun project, built with modern AI tools');
    expect(home).toContain('Built for fun, with the help of modern AI tools');
    expect(home).toContain('BETA is young and not perfect yet.');
    expect(home).toContain('id="help"');
  });

  it('links all three downloads and every feedback channel', () => {
    for (const d of Object.values(site.downloads)) expect(home).toContain(`href="${d.url}"`);
    for (const l of feedbackLinks()) expect(home).toContain(`href="${l.url}"`);
    expect(home).toContain(`href="${site.feedback.email}"`);
  });

  it('shows mean, median and win/tie/loss together, from the data, and claims no win', () => {
    const bench = home.slice(home.indexOf('id="benchmark"'), home.indexOf('id="features"'));
    expect(bench).toMatch(/\+0\.42<\/span><small> pp<\/small>[\s\S]*Mean difference/);
    expect(bench).toContain('−0.30');
    expect(bench).toContain('25 wins');
    expect(bench).toContain('32 losses');
    expect(bench).toContain('−0.73');
    expect(bench).toContain('−1.22');
    expect(bench).toMatch(/84 wins[\s\S]*12 ties[\s\S]*204 losses/);
    expect(bench).toContain('No general win over Lectra.');
    expect(home).not.toMatch(/BETA (beats|outperforms|is better than)/i);
  });

  it('keeps final numbers in the HTML for counters (works without JavaScript)', () => {
    expect(home).toMatch(/data-count="-0\.7295[0-9]*" data-decimals="2" data-signed="1">−0\.73</);
  });

  it('gives every screenshot alt text and a fixed size (no layout shift)', () => {
    const imgs = home.match(/<img [^>]+>/g) ?? [];
    expect(imgs.length).toBe(6);
    for (const img of imgs) {
      expect(img).toMatch(/alt="Screenshot of the .+ screen in BETA Cutting Room"/);
      expect(img).toContain('width="1920" height="1200"');
      expect(img).toContain('loading="lazy"');
    }
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
      expect(doc).toContain(`content="${site.siteUrl}assets/brand/social-preview.png"`);
      expect(doc).not.toMatch(/<(script|link|img)\b[^>]*\b(src|href)="(https?:)?\/\//);
      expect(doc).not.toContain('theme-toggle');
    }
  });

  it('install page explains the portable ZIP with its checksum', () => {
    const install = rendered['install.html']!;
    expect(install).toContain('id="portable"');
    expect(install).toContain(site.downloads.portable.sha256!);
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
