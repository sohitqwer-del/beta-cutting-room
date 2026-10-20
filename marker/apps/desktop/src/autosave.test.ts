import { describe, expect, it, vi } from 'vitest';
import { projectAutosave } from './autosave';

describe('desktop recovery autosave', () => {
  it('preserves recovery and its backup across idle ticks and another restart', async () => {
    let recovery = { name: 'Factory order' }, backup = { name: 'Previous factory order' };
    const hooks = {
      read: () => ({ name: 'Demo' }), busy: () => false, saved: vi.fn(),
      write: vi.fn(async (value: { name: string }) => { backup = recovery; recovery = value; }),
    };
    for (let session = 0; session < 2; session++) {
      const autosave = projectAutosave(hooks, true);
      await autosave.tick(); await autosave.tick(); await autosave.tick();
    }
    expect(hooks.write).not.toHaveBeenCalled();
    expect(recovery.name).toBe('Factory order');
    expect(backup.name).toBe('Previous factory order');
    const autosave = projectAutosave(hooks, true);
    autosave.useCurrent(); await autosave.tick(); await autosave.tick();
    expect(hooks.write).toHaveBeenCalledTimes(1);
    expect(recovery.name).toBe('Demo');
    expect(backup.name).toBe('Factory order');
  });

  it('saves a restored project and keeps later edits dirty during an in-flight write', async () => {
    let current = { name: 'Recovered order' }, finish!: () => void;
    const saved = vi.fn(), writes: { name: string }[] = [];
    const autosave = projectAutosave({
      read: () => structuredClone(current), busy: () => false, saved,
      write: async value => { writes.push(value); await new Promise<void>(resolve => { finish = resolve; }); },
    }, false);
    const first = autosave.tick();
    current = { name: 'Edited recovered order' };
    await autosave.tick();
    expect(writes).toEqual([{ name: 'Recovered order' }]);
    finish(); await first;
    expect(saved).not.toHaveBeenCalled();
    const second = autosave.tick(); finish(); await second;
    expect(writes[1]).toEqual(current);
    expect(saved).toHaveBeenCalledTimes(1);
  });

  it('waits for operations and retries a failed disk write', async () => {
    let busy = true;
    const write = vi.fn().mockRejectedValueOnce(new Error('Disk full')).mockResolvedValue(undefined);
    const autosave = projectAutosave({ read: () => ({ name: 'Order' }), busy: () => busy, write, saved: vi.fn() }, false);
    await autosave.tick(); expect(write).not.toHaveBeenCalled();
    busy = false;
    await expect(autosave.tick()).rejects.toThrow('Disk full');
    await autosave.tick(); expect(write).toHaveBeenCalledTimes(2);
  });
});
