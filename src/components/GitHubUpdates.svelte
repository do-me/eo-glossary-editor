<script lang="ts">
  import { onMount } from 'svelte';
  import { ArrowLeft, RefreshCw, Download, Check, GitCompareArrows } from '@lucide/svelte';
  import RenderedChange from './RenderedChange.svelte';
  import type { SyncReport, SyncConflict } from '../lib/types';
  let {
    report,
    busy,
    onback,
    oncheck,
    onresolve,
    onexport,
    scrollTop = 0,
    onscroll,
  }: {
    report: SyncReport | null;
    busy: boolean;
    onback: () => void;
    oncheck: () => void;
    onresolve: (conflict: SyncConflict, choice: 'github' | 'local') => void;
    onexport: () => void;
    scrollTop?: number;
    onscroll?: (value: number) => void;
  } = $props();
  let root: HTMLElement;
  onMount(() => {
    root.scrollTop = scrollTop;
  });
</script>

<main class="updates-page" bind:this={root} onscroll={() => onscroll?.(root.scrollTop)}>
  <button class="back-button" onclick={onback}><ArrowLeft size={15} />Back to editing</button>
  <div class="page-title">
    <div>
      <p class="eyebrow">GITHUB UPDATES</p>
      <h1>Review changes from GitHub</h1>
      <p class="muted">
        {report?.repository} · {report?.branch ?? 'main'} · {report?.sha.slice(0, 7)}
      </p>
    </div>
    <div class="update-actions">
      <button onclick={onexport}><Download size={14} />Export a backup</button><button
        onclick={oncheck}
        disabled={busy}><RefreshCw size={14} />Check again</button
      >
    </div>
  </div>
  {#if report?.deferred}<p class="notice banner">
      An interrupted submission is retained. Recover it before applying updates to this session.
    </p>{/if}
  {#if report?.conflicts.length}<p class="updates-intro">
      Another contribution changed the same content. Compare the GitHub version with your saved
      draft, then choose which version to keep for each term. Nothing is submitted by these choices.
    </p>
    {#each report.conflicts as conflict (conflict.path)}<section
        class="conflict-card"
        aria-label={`Conflict: ${conflict.title}`}
      >
        <header>
          <GitCompareArrows size={16} /><strong>{conflict.title}</strong><code
            >{conflict.path.replace('docs/terms/', '')}</code
          >
        </header>
        {#if conflict.deleted}<p class="notice banner">
            This term was removed from GitHub. Keeping your version proposes restoring it.
          </p>{/if}
        <RenderedChange
          change={conflict.comparison}
          mode="compare"
          repository={report.repository}
          sha={conflict.githubSha}
          beforeLabel={`GitHub ${report.branch}`}
          afterLabel="Your version"
          emptyBefore="This term was removed from GitHub."
        />
        <details class="source-diff">
          <summary>See concrete source diffs from the common version</summary>
          <div class="comparison">
            <div>
              <h3>Changes merged into GitHub</h3>
              <pre>{conflict.githubDiff}</pre>
            </div>
            <div>
              <h3>Your local changes</h3>
              <pre>{conflict.localDiff}</pre>
            </div>
          </div>
        </details>
        <footer class="conflict-actions">
          <div>
            <button onclick={() => onresolve(conflict, 'github')} disabled={busy}
              >Use GitHub version</button
            ><small>Pull this term and overwrite my local edits.</small>
          </div>
          <div>
            <button class="primary" onclick={() => onresolve(conflict, 'local')} disabled={busy}
              >Keep my version</button
            ><small>Keep my saved draft for the next contribution.</small>
          </div>
        </footer>
      </section>{/each}
  {:else}<div class="review-empty">
      <Check size={25} />
      <h2>Your session is up to date</h2>
      <p class="muted">Local edits and the latest GitHub version are reconciled.</p>
    </div>{/if}
</main>
