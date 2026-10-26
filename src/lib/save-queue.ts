import type { Edit, OpenTerm } from './types';
/** One ordered writer per term. Revision acknowledgements precede subsequent edits. */
export class SaveQueue {
  private pending = new Map<string, Edit>();
  private running: Promise<void> | null = null;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private firstPending = 0;
  private failure: unknown = null;
  revision: number;
  constructor(
    initial: number,
    private write: (revision: number, edit: Edit) => Promise<OpenTerm>,
    private acknowledged: (term: OpenTerm) => void,
    private status: (state: 'saving' | 'saved' | 'error', error?: unknown) => void,
  ) {
    this.revision = initial;
  }
  enqueue(edit: Edit): void {
    const key = edit.kind === 'section' ? `section:${edit.id}` : edit.kind;
    this.pending.set(key, edit);
    this.failure = null;
    this.status('saving');
    if (!this.firstPending) this.firstPending = Date.now();
    clearTimeout(this.timer);
    this.timer = setTimeout(
      () => {
        void this.flush().catch(() => {});
      },
      Math.min(350, Math.max(0, 1800 - (Date.now() - this.firstPending))),
    );
  }
  async flush(): Promise<void> {
    clearTimeout(this.timer);
    if (this.running) {
      await this.running;
      if (this.pending.size) return this.flush();
      return;
    }
    if (!this.pending.size) {
      if (this.failure) throw this.failure;
      return;
    }
    this.running = this.drain();
    try {
      await this.running;
    } finally {
      this.running = null;
    }
  }
  private async drain(): Promise<void> {
    while (this.pending.size) {
      const entry = this.pending.entries().next().value as [string, Edit];
      const [key, edit] = entry;
      this.pending.delete(key);
      try {
        const term = await this.write(this.revision, edit);
        this.revision = term.revision;
        this.acknowledged(term);
        this.failure = null;
      } catch (error) {
        if (!this.pending.has(key)) this.pending.set(key, edit);
        this.failure = error;
        this.status('error', error);
        throw error;
      }
    }
    this.firstPending = 0;
    this.status('saved');
  }
}
