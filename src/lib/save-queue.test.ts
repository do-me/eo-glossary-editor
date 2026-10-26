import { describe, it, expect, vi } from 'vitest';
import { SaveQueue } from './save-queue';
import type { OpenTerm } from './types';
const term = (revision: number) => ({ revision }) as OpenTerm;
describe('durable save sequencing', () => {
  it('waits for acknowledgements and uses the returned revision', async () => {
    let release!: (value: OpenTerm) => void;
    const revisions: number[] = [];
    const write = vi.fn((revision: number) => {
      revisions.push(revision);
      return revisions.length === 1
        ? new Promise<OpenTerm>((resolve) => (release = resolve))
        : Promise.resolve(term(revision + 1));
    });
    const status = vi.fn();
    const q = new SaveQueue(7, write, () => {}, status);
    q.enqueue({ kind: 'section', id: 1, markdown: 'First' });
    const pending = q.flush();
    q.enqueue({ kind: 'section', id: 2, markdown: 'Second' });
    expect(write).toHaveBeenCalledTimes(1);
    release(term(8));
    await pending;
    expect(revisions).toEqual([7, 8]);
    expect(q.revision).toBe(9);
    expect(status).toHaveBeenLastCalledWith('saved');
  });
  it('coalesces idle edits and retains a failed write for retry', async () => {
    const write = vi
      .fn()
      .mockRejectedValueOnce(new Error('Disk full'))
      .mockResolvedValueOnce(term(1));
    const ack = vi.fn();
    const status = vi.fn();
    const q = new SaveQueue(0, write, ack, status);
    q.enqueue({ kind: 'section', id: 1, markdown: 'old' });
    q.enqueue({ kind: 'section', id: 1, markdown: 'new' });
    await expect(q.flush()).rejects.toThrow('Disk full');
    expect(ack).not.toHaveBeenCalled();
    expect(status).toHaveBeenLastCalledWith('error', expect.any(Error));
    await q.flush();
    expect(write).toHaveBeenLastCalledWith(0, { kind: 'section', id: 1, markdown: 'new' });
    expect(ack).toHaveBeenCalledWith(term(1));
  });
});
