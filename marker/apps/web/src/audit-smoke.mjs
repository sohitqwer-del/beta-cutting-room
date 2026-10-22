// Headless integration regression. Run on a dedicated Vite origin with the real API:
// (await import('/src/audit-smoke.mjs')).runRollAuditChecks()
// Uses public original fixtures, actual form handlers and Rust responses. No API mocks.
import fixture from '../../../packages/marker-wasm/test/fixtures/cutting-room.json';
import { byId } from './screens';
const $ = id => byId(id);
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const checks = [];
function assert(value, label) { if (!value) throw new Error(`${label}: ${$('roll-status')?.textContent}; ${$('spread-status')?.textContent}; ${$('bundle-status')?.textContent}`); checks.push(label); }
function set(id, value, event = 'input') { $(id).value = String(value); $(id).dispatchEvent(new Event(event, { bubbles: true })); }
function screen(id) { document.querySelector(`[data-screen="${id}"]`).click(); }
async function until(test, label) { for (let i = 0; i < 300; i++) { if (test()) return; await delay(50); } throw new Error(`Timeout: ${label}; ${$('roll-status')?.textContent}; ${$('spread-status')?.textContent}; ${$('bundle-status')?.textContent}`); }
async function upload(id, name, value) {
  const data = new DataTransfer(); data.items.add(new File([typeof value === 'string' ? value : JSON.stringify(value)], name));
  $(id).files = data.files; $(id).dispatchEvent(new Event('change', { bubbles: true })); await delay(100);
}
async function inspect() { $('inspect-rolls').click(); await until(() => $('roll-status').textContent.includes('Inspection ready'), 'inspection'); }

export async function runRollAuditChecks() {
  checks.length = 0;
  screen(0); await upload('open-file', 'original.job.json', fixture.job); await upload('open-file', 'original.result.json', fixture.result);
  screen(6);
  const register = structuredClone(fixture.register);
  register.rolls[0].id = 'AUDIT-R1'; register.rolls[0].defects = [];
  await upload('roll-file', 'AUDIT.JSON', register);
  await until(() => $('roll-status').textContent.includes('1 rolls imported'), 'uppercase JSON import');
  assert($('roll-table').textContent.includes('AUDIT-R1'), 'uppercase JSON register imports');
  set('spread-source-select', 'marker', 'change'); set('spread-plies', 2);
  set('inspection-limit', 40); set('inspection-unit', 'per100_square_yards'); set('shrink-band', 2);
  await inspect();
  assert($('inspection-result').textContent.includes('PASS'), 'real inspection passes original synthetic stock');
  for (const [id, value] of [['inspection-limit', 20], ['inspection-unit', 'per100_square_metres'], ['shrink-band', 1]]) {
    set(id, value);
    assert(!$('inspection-result').textContent.includes('PASS'), `${id} clears obsolete inspection`);
    $('spread-rolls').click();
    await until(() => $('spread-status').textContent.includes('Inspect the current'), 'allocation guard');
    assert(!$('spread-result').textContent.includes('COMPLETE'), `${id} blocks allocation until re-inspected`);
    await inspect();
  }
  // Deliver the real server response after an operator changes the policy.
  const originalFetch = window.fetch;
  let release, started;
  const called = new Promise(resolve => { started = resolve; });
  const held = new Promise(resolve => { release = resolve; });
  window.fetch = async (...args) => {
    const response = await originalFetch(...args);
    if (String(args[0]).includes('/api/roll/inspect')) { started(); await held; }
    return response;
  };
  try {
    $('inspect-rolls').click();
    await Promise.race([called, delay(15000).then(() => { throw new Error('Inspection response did not arrive.'); })]);
    set('inspection-limit', 10); release();
    await until(() => $('roll-status').textContent.includes('Inspection settings changed. Inspect rolls again.'), 'stale response guard');
    assert(!$('inspection-result').textContent.includes('PASS'), 'in-flight response cannot restore an obsolete PASS');
  } finally { release(); window.fetch = originalFetch; }
  await inspect(); $('spread-rolls').click();
  await until(() => $('spread-result').textContent.includes('COMPLETE'), 'audited spread');
  document.querySelector('[data-spread-bundles]').click();
  await until(() => $('bundle-source').value === 'spread', 'bundle handoff');
  const identity = { lay: $('bundle-lay').value, plies: $('bundle-plies').value, runs: $('bundle-runs').value };
  set('bundle-source', 'marker', 'change'); set('bundle-lay', 'MANUAL-OTHER'); set('bundle-runs', '1,2,Manual');
  set('bundle-source', 'spread', 'change');
  assert($('bundle-lay').value === identity.lay && !identity.lay.includes('NaN'), 'reselection retains audited lay ID');
  assert($('bundle-plies').value === identity.plies && $('bundle-runs').value === identity.runs, 'reselection restores audited plies and shade runs');
  $('make-bundles').click(); await until(() => $('bundle-status').textContent.includes('bundles ready'), 'bundle generation');
  assert($('bundle-table').textContent.includes(identity.lay), 'reselected audited lay produces real bundles');
  // Reconcile actual use using original fixture geometry, including the saved lay.
  set('report-issued', 42); set('report-end', 0); set('report-pieces', 2); set('report-area', 800); set('report-allowance', 0);
  $('report-form').requestSubmit();
  await until(() => $('bundle-status').textContent === 'Cutting report reconciled by Rust.', 'cutting report');
  assert(!$('bundle-status').classList.contains('error-text'), 'measured report reconciles the saved bundle source');
  return { checks: [...checks], bundleLay: identity.lay, report: $('report-result').textContent };
}

export async function runIncompletePlanCheck() {
  screen(0);
  const job = structuredClone(fixture.job); job.fabric.width = 10;
  await upload('open-file', 'too-narrow.job.json', job);
  screen(1); set('quantity-0', 2); set('max-plies', 2); set('max-garments', 1); set('plan-time', 0.1);
  $('plan-form').requestSubmit();
  await until(() => $('plan-error').textContent.includes('Incomplete plan.'), 'partial plan status');
  const status = $('plan-error').textContent;
  assert(status.includes('M: 2') && status.includes('Do not cut'), 'real infeasible-width order reports its shortfall');
  return { status };
}

export async function runCompareChecks() {
  const response = await fetch('/api/export?format=plt', { method: 'POST', body: JSON.stringify({ job: fixture.job, result: fixture.result }) });
  if (!response.ok) throw new Error('Original fixture PLT export failed.');
  screen(2); set('compare-width', fixture.job.fabric.width); set('compare-time', 0.1);
  await upload('compare-file', 'original.plt', await response.text());
  await until(() => $('compare-status').textContent.includes('Current CAD marker loaded.'), 'PLT import in Compare');
  $('compare-run').click();
  await until(() => $('compare-status').textContent.includes('Comparison ready.'), 'Compare re-nest');
  const summary = $('compare-result').textContent;
  assert(summary.includes('Checks passed') && summary.includes('fabric saving per marker'), 'Compare validates both layouts before reporting savings');
  $('open-theirs').click();
  assert(!document.querySelector('.workspace').hidden, 'Compare opens its checked original marker in Editor');
  return { summary };
}
