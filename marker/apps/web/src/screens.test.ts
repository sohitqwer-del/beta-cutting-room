import { describe, expect, it } from 'vitest';
import { backgroundLoadAllowed, holdBackground, whenIdle } from './screens';

describe('background engine downloads', () => {
  it('always loads ahead in the desktop app', () => {
    expect(backgroundLoadAllowed({ desktop: true, connection: { saveData: true }, wide: false })).toBe(true);
  });
  it('never with data saver or on a slow connection', () => {
    expect(backgroundLoadAllowed({ desktop: false, connection: { saveData: true, effectiveType: '4g' }, wide: true })).toBe(false);
    expect(backgroundLoadAllowed({ desktop: false, connection: { effectiveType: '3g' }, wide: true })).toBe(false);
    expect(backgroundLoadAllowed({ desktop: false, connection: { effectiveType: '4g' }, wide: false })).toBe(true);
  });
  it('without connection facts: wide screens yes, phones no (most likely mobile data)', () => {
    expect(backgroundLoadAllowed({ desktop: false, wide: true })).toBe(true);
    expect(backgroundLoadAllowed({ desktop: false, wide: false })).toBe(false);
  });
});

describe('holding background downloads', () => {
  it('queues background work until the last hold is released, and releases only once', () => {
    const g = globalThis as { __TAURI__?: unknown; requestIdleCallback?: (fn: () => void) => number };
    const saved = { tauri: g.__TAURI__, idle: g.requestIdleCallback };
    g.__TAURI__ = {}; g.requestIdleCallback = fn => { fn(); return 1; };
    try {
      let ran = 0;
      const a = holdBackground(), b = holdBackground();
      whenIdle(() => { ran++; });
      expect(ran).toBe(0);
      a(); a();
      expect(ran).toBe(0);
      b();
      expect(ran).toBe(1);
      whenIdle(() => { ran++; });
      expect(ran).toBe(2);
    } finally { g.__TAURI__ = saved.tauri; g.requestIdleCallback = saved.idle; }
  });
});
