<script lang="ts">
  import { ArrowUpRight, Check, GitPullRequest, GitMerge } from '@lucide/svelte';
  import type { Submission } from '../lib/types';
  let {
    submission,
    pending,
    submitted,
    busy,
    onopen,
    oncontinue,
    onfollowup,
  }: {
    submission: Submission;
    pending: number;
    submitted: number;
    busy: boolean;
    onopen: () => void;
    oncontinue: () => void;
    onfollowup: () => void;
  } = $props();
  const closed = $derived(submission.state !== 'open');
  const heading = $derived(
    submission.state === 'merged'
      ? 'Contribution merged'
      : closed
        ? 'Pull request closed'
        : pending
          ? 'New edits to send'
          : 'Contribution submitted',
  );
</script>

<section class="submission-summary" aria-label="Pull request status">
  <div class="submission-summary-icon" aria-hidden="true">
    {#if submission.state === 'merged'}<GitMerge size={22} />{:else if !closed && !pending}<Check
        size={22}
      />{:else}<GitPullRequest size={22} />{/if}
  </div>
  <div class="submission-summary-content">
    <div class="submission-summary-heading">
      <h2 id="contribution-status-heading" tabindex="-1">{heading}</h2>
      <span class="pill">PR #{submission.number} · {submission.state}</span>
    </div>
    <p>
      {#if submission.state === 'merged'}Your contribution is part of the glossary. Start a
        follow-up for further changes.
      {:else if closed}This pull request is closed. Start a follow-up to continue your work.
      {:else if pending}{pending}
        {pending === 1 ? 'term has' : 'terms have'} saved local edits that are not in this pull request
        yet.
      {:else}All your saved edits are in the pull request. You can continue editing and update it
        later.{/if}
    </p>
    <span class="small muted"
      >{submitted}
      {submitted === 1 ? 'term' : 'terms'} submitted to GitHub{#if closed && pending}
        · {pending} with newer local edits{/if}</span
    >
    <div class="submission-summary-actions">
      <button class="primary" onclick={onopen}
        >View PR #{submission.number}<ArrowUpRight size={14} /></button
      >
      {#if closed}<button onclick={onfollowup} disabled={busy}>Start follow-up</button
        >{:else}<button onclick={oncontinue}>Continue editing</button>{/if}
    </div>
  </div>
</section>
