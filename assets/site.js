// BETA Cutting Room website motion. Generated from src/client.ts.
(function () {
'use strict';
// Browser script for the website: scroll reveals, counters, the workflow steps, the screenshot
// showcase and the hero replay. The build strips the types, removes `export` and wraps it in
// a function, so it runs as a classic script (also from file://). No imports, no network.
//
// Rules: content is always in the HTML. Motion only moves or fades it (transform, opacity).
// With prefers-reduced-motion, or without JavaScript, everything is shown in its final state.

/** Ease-out cubic, 0..1 -> 0..1. */
function easeOutCubic(t        )         {
  const c = Math.min(1, Math.max(0, t));
  return 1 - (1 - c) ** 3;
}

/** Formats a counter value like the build does: fixed decimals, real minus sign, optional plus. */
function formatCount(value        , decimals        , signed         )         {
  const s = Math.abs(value).toFixed(decimals);
  const zero = Number(s) === 0;
  if (value < 0 && !zero) return `−${s}`;
  return signed && value > 0 && !zero ? `+${s}` : s;
}

/** Index after moving `dir` steps through `n` items, wrapping around. */
function wrapIndex(i        , dir        , n        )         {
  return n > 0 ? (((i + dir) % n) + n) % n : 0;
}

function initMotion()       {
  const root = document.documentElement;
  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)');
  const motionOk = () => !reduce.matches;
  const canObserve = 'IntersectionObserver' in window;
  // The head script adds `motion` before first paint; we confirm it here. If this script
  // never runs, the head script removes `motion` again, so nothing stays hidden.
  if (!motionOk() || !canObserve) root.classList.remove('motion');
  root.classList.add('motion-ready');

  // Calls `fn` once when `el` first comes into view (or at once without observer support).
  // Safety net: a fast jump can skip the observer, so after each scroll we also run every
  // pending callback whose element is already above the bottom of the screen.
  const pending = new Map                     ();
  const onView = (el         , fn            , threshold = 0.2) => {
    if (!canObserve) { fn(); return; }
    const once = () => { if (pending.delete(el)) { io.disconnect(); fn(); } };
    const io = new IntersectionObserver((entries) => { if (entries.some((e) => e.isIntersecting)) once(); }, { threshold, rootMargin: '0px 0px -8% 0px' });
    pending.set(el, once);
    io.observe(el);
  };
  let sweepQueued = false;
  const sweep = () => {
    sweepQueued = false;
    for (const [el, once] of pending) if (el.getBoundingClientRect().top < window.innerHeight * 0.9) once();
  };
  window.addEventListener('scroll', () => { if (!sweepQueued) { sweepQueued = true; window.setTimeout(sweep, 250); } }, { passive: true });

  // Scroll reveal.
  document.querySelectorAll('[data-stagger]').forEach((group) => {
    Array.from(group.children).forEach((child, i) => (child               ).style.setProperty('--d', String(i)));
  });
  document.querySelectorAll('[data-reveal]').forEach((el) => onView(el, () => el.classList.add('is-in'), 0.12));

  // Counters: the HTML holds the final text; we count up to it.
  document.querySelectorAll             ('[data-count]').forEach((el) => {
    const to = Number(el.dataset.count); const decimals = Number(el.dataset.decimals ?? '0');
    const signed = el.dataset.signed === '1'; const final = el.textContent ?? '';
    if (!Number.isFinite(to) || !motionOk()) return;
    onView(el, () => {
      const start = performance.now(); const duration = 1400;
      const tick = (now        ) => {
        const t = (now - start) / duration;
        if (t >= 1) { el.textContent = final; return; }
        el.textContent = formatCount(to * easeOutCubic(t), decimals, signed);
        requestAnimationFrame(tick);
      };
      el.textContent = formatCount(0, decimals, signed);
      requestAnimationFrame(tick);
    }, 0.6);
  });

  // Header shadow after scrolling.
  const header = document.querySelector('.site-header');
  if (header) {
    let queued = false;
    const update = () => { queued = false; header.classList.toggle('is-scrolled', window.scrollY > 8); };
    window.addEventListener('scroll', () => { if (!queued) { queued = true; requestAnimationFrame(update); } }, { passive: true });
    update();
  }

  // Replay buttons restart a CSS animation by toggling `is-playing` on their target.
  document.querySelectorAll                   ('[data-replay]').forEach((btn) => {
    const target = document.getElementById(btn.dataset.replay ?? '');
    if (!target) return;
    btn.hidden = !motionOk();
    btn.addEventListener('click', () => {
      target.classList.remove('is-playing');
      void (target               ).offsetWidth;
      target.classList.add('is-playing');
    });
  });

  // Workflow: light the steps one by one when the section comes into view.
  document.querySelectorAll             ('[data-flow]').forEach((flow) => {
    const steps = Array.from(flow.querySelectorAll             ('[data-flow-step]'));
    const replay = flow.querySelector                   ('[data-flow-replay]');
    let timer = 0;
    const show = (n        ) => {
      flow.style.setProperty('--progress', String(steps.length > 1 ? n / (steps.length - 1) : 1));
      steps.forEach((s, i) => { s.classList.toggle('is-lit', i <= n); s.classList.toggle('is-current', i === n); });
    };
    const play = () => {
      window.clearInterval(timer);
      if (!motionOk()) { show(steps.length - 1); return; }
      let n = 0; show(0);
      timer = window.setInterval(() => { n += 1; show(n); if (n >= steps.length - 1) window.clearInterval(timer); }, 520);
    };
    flow.classList.add('flow-ready');
    show(motionOk() ? -1 : steps.length - 1);
    steps.forEach((s, i) => {
      s.addEventListener('mouseenter', () => { window.clearInterval(timer); show(i); });
      s.addEventListener('focusin', () => { window.clearInterval(timer); show(i); });
    });
    if (replay) { replay.hidden = !motionOk(); replay.addEventListener('click', play); }
    onView(flow, play, 0.3);
  });

  // Screenshot showcase: a tabbed carousel that advances on its own until paused.
  document.querySelectorAll             ('[data-carousel]').forEach((box) => {
    const slides = Array.from(box.querySelectorAll             ('[data-slide]'));
    const tabs = Array.from(box.querySelectorAll                   ('[data-tab]'));
    const toggle = box.querySelector                   ('[data-toggle]');
    const tablist = box.querySelector             ('[data-tablist]');
    if (slides.length < 2 || tabs.length !== slides.length || !tablist) return;
    box.classList.add('is-carousel');
    tablist.hidden = false;
    tablist.setAttribute('role', 'tablist');
    let current = 0; let timer = 0;
    let paused = !motionOk(); let held = false;
    const delay = 5000;
    slides.forEach((s, i) => { s.setAttribute('role', 'tabpanel'); s.setAttribute('aria-labelledby', tabs[i] .id); });
    tabs.forEach((t, i) => { t.setAttribute('role', 'tab'); t.setAttribute('aria-controls', slides[i] .id); });
    const select = (i        , focus = false) => {
      current = i;
      slides.forEach((s, k) => { s.classList.toggle('is-active', k === i); s.inert = k !== i; });
      // Load the shown slide and the next one now, so a change never shows an empty frame.
      for (const k of [i, wrapIndex(i, 1, slides.length)]) { const img = slides[k] .querySelector('img'); if (img) img.loading = 'eager'; }
      tabs.forEach((t, k) => { t.setAttribute('aria-selected', String(k === i)); t.tabIndex = k === i ? 0 : -1; t.classList.remove('is-timing'); });
      if (focus) tabs[i] .focus();
      schedule();
    };
    const running = () => !paused && !held && !document.hidden;
    const schedule = () => {
      window.clearTimeout(timer);
      const tab = tabs[current] ;
      tab.classList.remove('is-timing');
      if (!running()) return;
      void tab.offsetWidth;
      tab.style.setProperty('--slide-ms', `${delay}ms`);
      tab.classList.add('is-timing');
      timer = window.setTimeout(() => select(wrapIndex(current, 1, slides.length)), delay);
    };
    const showToggle = () => {
      if (!toggle) return;
      toggle.hidden = false;
      toggle.setAttribute('aria-pressed', String(paused));
      toggle.querySelector('[data-toggle-label]') .textContent = paused ? 'Play slideshow' : 'Pause slideshow';
      toggle.classList.toggle('is-paused', paused);
    };
    tabs.forEach((t, i) => {
      t.addEventListener('click', () => select(i));
      t.addEventListener('keydown', (e) => {
        const dir = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
        if (dir) { e.preventDefault(); select(wrapIndex(current, dir, slides.length), true); }
        if (e.key === 'Home') { e.preventDefault(); select(0, true); }
        if (e.key === 'End') { e.preventDefault(); select(slides.length - 1, true); }
      });
    });
    toggle?.addEventListener('click', () => { paused = !paused; showToggle(); schedule(); });
    box.addEventListener('mouseenter', () => { held = true; schedule(); });
    box.addEventListener('mouseleave', () => { held = false; schedule(); });
    box.addEventListener('focusin', () => { held = true; schedule(); });
    box.addEventListener('focusout', (e) => { if (!box.contains(e.relatedTarget               )) { held = false; schedule(); } });
    document.addEventListener('visibilitychange', schedule);
    showToggle();
    select(0);
  });

  // A gentle tilt of the hero card that follows the pointer (fine pointers only).
  const tilt = document.querySelector             ('[data-tilt]');
  if (tilt && motionOk() && window.matchMedia('(pointer: fine)').matches) {
    tilt.addEventListener('pointermove', (e) => {
      const r = tilt.getBoundingClientRect();
      tilt.style.setProperty('--rx', `${(((e.clientY - r.top) / r.height) - 0.5) * -4}deg`);
      tilt.style.setProperty('--ry', `${(((e.clientX - r.left) / r.width) - 0.5) * 5}deg`);
    });
    tilt.addEventListener('pointerleave', () => { tilt.style.setProperty('--rx', '0deg'); tilt.style.setProperty('--ry', '0deg'); });
  }
}

if (typeof document !== 'undefined') {
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', initMotion);
  else initMotion();
}

})();
