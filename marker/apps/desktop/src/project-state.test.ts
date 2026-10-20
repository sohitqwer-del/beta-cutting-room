import { describe, expect, it } from 'vitest';
import { projectPanels } from './project-state';
describe('native project state', () => {
  it('keeps a live draft over a stale browser cache', () => {
    const live = { document: { name: 'Accepted new pattern' }, ledger: [{ id: 'D2' }] };
    const restored = projectPanels({ planning: { plan: { name: 'Lay plan' }, cuttingRoom: live }, storage: { 'beta-cutting-room-v1': '{"document":{"name":"Old cache"}}' } });
    expect(restored).toEqual({ plan: { name: 'Lay plan' }, cuttingRoom: live });
  });
  it('round-trips a project larger than browser session storage without using it', () => {
    const large = 'x'.repeat(6 * 1024 * 1024);
    const project = JSON.parse(JSON.stringify({ planning: { cuttingRoom: { document: { meta: { note: large } } } }, storage: {} }));
    expect((projectPanels(project) as typeof project.planning).cuttingRoom.document.meta.note).toHaveLength(large.length);
  });
  it('reads the first project layout for saved drafts', () => {
    expect(projectPanels({ planning: { planEnd: 10 }, storage: { 'beta-cutting-room-v1': '{"document":{"name":"Legacy"}}' } })).toEqual({ planEnd: 10, cuttingRoom: { document: { name: 'Legacy' } } });
  });
});
