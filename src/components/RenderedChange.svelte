<script lang="ts">
  import type { Change, ReviewMode } from '../lib/types';
  import { compareSections, type ReviewContext } from '../lib/review';
  import ReviewMarkup from './ReviewMarkup.svelte';
  let {
    change,
    mode,
    repository,
    sha,
    beforeLabel = 'Before',
    afterLabel = 'After',
    emptyBefore = 'This term is new.',
  }: {
    change: Change;
    mode: ReviewMode;
    repository: string;
    sha: string;
    beforeLabel?: string;
    afterLabel?: string;
    emptyBefore?: string;
  } = $props();
  const context = $derived<ReviewContext>({ repository, sha, path: change.path });
  const sections = $derived(compareSections(change.beforePage, change.afterPage, context));
  const oldTags = $derived(change.beforePage?.tags ?? []);
  const newTags = $derived(change.afterPage.tags);
  const metadataChanged = $derived(
    change.beforePage &&
      (change.beforePage.title !== change.afterPage.title ||
        change.beforePage.description !== change.afterPage.description ||
        JSON.stringify(oldTags) !== JSON.stringify(newTags)),
  );
  const onlySourceChanged = $derived(
    !change.isNew && !metadataChanged && sections.every((s) => !s.changed),
  );
</script>

<div class="rendered-change" class:two-pages={mode === 'compare'}>
  {#if mode === 'compare'}
    {#each [{ label: beforeLabel, page: change.beforePage, side: 'before' }, { label: afterLabel, page: change.afterPage, side: 'after' }] as version (version.side)}
      <article class="review-document" aria-label={`${change.title} ${version.label}`}>
        <div class="preview-version">{version.label}</div>
        {#if version.page}<h2 class="preview-title">{version.page.title}</h2>
          <div class="document-tags">
            {#each version.page.tags as tag, i (i)}<span>{tag}</span>{/each}
          </div>
          {#if metadataChanged && change.beforePage?.description !== change.afterPage.description}<div
              class="description-change"
            >
              <span class="preview-version">Description</span>
              <p>{version.page.description || 'No description'}</p>
            </div>{/if}
          {#each sections.filter( (s) => (version.side === 'before' ? change.beforePage?.sections.some((p) => p.key === s.key) : !s.removed) ) as section (section.key)}
            <section class="review-section" class:section-changed={section.changed}>
              {#if section.heading}<svelte:element this={section.level === 2 ? 'h3' : 'h4'}
                  >{section.heading}</svelte:element
                >{/if}
              <ReviewMarkup
                html={version.side === 'before' ? section.before : section.after}
                {context}
              />
            </section>
          {/each}
        {:else}<p class="muted">{emptyBefore}</p>{/if}
      </article>
    {/each}
  {:else}
    <article
      class="review-document"
      aria-label={`${change.title} ${mode === 'preview' ? 'preview' : 'changes'}`}
    >
      <h2 class="preview-title">{change.afterPage.title}</h2>
      <div class="document-tags">
        {#each newTags as tag, i (i)}<span
            class:tag-added={mode === 'changes' && !oldTags.includes(tag)}>{tag}</span
          >{/each}{#if mode === 'changes'}{#each oldTags.filter((t) => !newTags.includes(t)) as tag, i (i)}<span
              class="tag-removed">{tag}</span
            >{/each}{/if}
      </div>
      {#if metadataChanged && change.beforePage?.description !== change.afterPage.description}<div
          class="description-change"
        >
          <span class="preview-version">Description</span>{#if mode === 'changes'}<p
              class="old-description"
            >
              {change.beforePage?.description || 'No description'}
            </p>{/if}
          <p>{change.afterPage.description || 'No description'}</p>
        </div>{/if}
      {#if onlySourceChanged && mode === 'changes'}<p class="formatting-note">
          The rendered content is unchanged. See the source diff for formatting changes.
        </p>{/if}
      {#each sections.filter((s) => mode === 'changes' || !s.removed) as section (section.key)}
        <section class="review-section" aria-label={section.heading || 'Introduction'}>
          {#if section.heading}<svelte:element this={section.level === 2 ? 'h3' : 'h4'}
              >{section.heading}{#if mode === 'changes' && !change.isNew && (section.added || section.removed)}<span
                  class="section-change-label"
                  >{section.removed ? 'Removed section' : 'Added section'}</span
                >{/if}</svelte:element
            >{/if}
          {#if mode === 'changes' && section.changed}
            {#if section.before.trim()}<div class="revision-before">
                <div class="preview-version">{section.removed ? 'Removed' : 'Before'}</div>
                <ReviewMarkup html={section.before} {context} />
              </div>{/if}
            {#if section.after.trim()}<div class="revision-after">
                <div class="preview-version">{section.before.trim() ? 'After' : 'Added'}</div>
                <ReviewMarkup html={section.after} {context} />
              </div>{/if}
          {:else}<ReviewMarkup html={section.html} {context} />{/if}
        </section>
      {/each}
    </article>
  {/if}
</div>
