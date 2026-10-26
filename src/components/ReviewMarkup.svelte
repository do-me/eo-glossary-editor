<script lang="ts">
  import { cleanReviewHtml, type ReviewContext } from '../lib/review';
  let { html, context }: { html: string; context: ReviewContext } = $props();
  const safe = $derived(cleanReviewHtml(html, context));
</script>

<div class="review-prose">
  <!-- The sole HTML sink is always sanitized, including generated diff markup. -->
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html safe}
</div>
