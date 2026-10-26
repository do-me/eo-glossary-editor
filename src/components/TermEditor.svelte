<script lang="ts">
  import { untrack } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { LockKeyhole, Plus, Info, RotateCcw, Code, ChevronDown } from '@lucide/svelte';
  import SectionEditor from './SectionEditor.svelte';
  import type { OpenTerm, Edit, TermUi } from '../lib/types';
  let {
    term,
    teams,
    classes,
    statuses,
    onedit,
    onadd,
    onrevert,
    readonly = false,
    locked = false,
    ui = { details: false, source: false, sourceSections: [] },
    onuichange,
    onready,
  }: {
    term: OpenTerm;
    teams: string[];
    classes: string[];
    statuses: string[];
    onedit: (edit: Edit) => void;
    onadd: () => void;
    onrevert: () => void;
    readonly?: boolean;
    locked?: boolean;
    ui?: TermUi;
    onuichange?: (ui: TermUi) => void;
    onready?: () => void;
  } = $props();
  let description = $state(untrack(() => term.document.description));
  let tags = $state(untrack(() => [...term.document.tags]));
  let details = $state(untrack(() => ui.details));
  let source = $state(untrack(() => ui.source));
  const disabled = $derived(readonly || locked);
  let sourceSections = $state(untrack(() => [...ui.sourceSections]));
  function uiChanged() {
    onuichange?.({ details, source, sourceSections: [...sourceSections] });
  }
  const readySections = new SvelteSet<number>();
  function sectionReady(id: number) {
    readySections.add(id);
    if (readySections.size === term.document.sections.filter((s) => s.editable).length) onready?.();
  }
  function metadata() {
    onedit({ kind: 'metadata', description, tags: [...tags] });
  }
  function groupChange(group: string[], value: string) {
    tags = tags.filter((t) => !group.includes(t));
    if (value) tags.push(value);
    metadata();
  }
  function toggleTeam(team: string) {
    tags = tags.includes(team) ? tags.filter((t) => t !== team) : [...tags, team];
    metadata();
  }
</script>

<article class="term-document">
  <div class="document-eyebrow">
    {#if term.isNew}<span>NEW TERM</span>{/if}<span class="document-actions"
      ><button
        class:active={details}
        onclick={() => {
          details = !details;
          uiChanged();
        }}><Info size={14} /> Details <ChevronDown size={12} /></button
      ><button
        title="Read complete Markdown"
        aria-label="Read complete Markdown"
        onclick={() => {
          source = !source;
          uiChanged();
        }}><Code size={15} /></button
      ></span
    >
  </div>
  <h1 class="term-title" data-protected-heading>{term.document.title}</h1>
  <div class="document-tags">
    {#each tags as tag, i (i)}<span class:team-tag={teams.includes(tag)}>{tag}</span>{/each}
  </div>
  {#if readonly}<p class="notice">
      This PR is {term.isNew ? 'closed' : 'closed or merged'}. Start a new session to make another
      contribution.
    </p>{/if}
  {#if details}
    <section class="details-panel" aria-label="Term details">
      <label
        >Description<textarea
          bind:value={description}
          oninput={metadata}
          {disabled}
          rows="2"
          placeholder="Short description for search results"
        ></textarea></label
      >
      <div class="form-row">
        <label
          >Term class<select
            {disabled}
            value={tags.find((t) => classes.includes(t)) ?? ''}
            onchange={(e) => groupChange(classes, e.currentTarget.value)}
            ><option value="">Select class</option>{#each classes as value (value)}<option
                >{value}</option
              >{/each}</select
          ></label
        ><label
          >Discussion status<select
            {disabled}
            value={tags.find((t) => statuses.includes(t)) ?? ''}
            onchange={(e) => groupChange(statuses, e.currentTarget.value)}
            ><option value="">Select status</option>{#each statuses as value (value)}<option
                >{value}</option
              >{/each}</select
          ></label
        >
      </div>
      <fieldset {disabled}>
        <legend>Review teams</legend>
        <div class="team-options">
          {#each teams as team (team)}<label
              ><input
                type="checkbox"
                checked={tags.includes(team)}
                onchange={() => toggleTeam(team)}
              />{team}</label
            >{/each}
        </div>
      </fieldset>
      <p class="muted small">
        Existing custom tags are preserved. Discussion status is independent of this editing
        session.
      </p>
      <div class="detail-footer">
        <code>{term.path}</code><button
          onclick={onrevert}
          disabled={disabled || term.revision === 0}><RotateCcw size={13} /> Revert term…</button
        >
      </div>
    </section>
  {/if}
  {#if source}<details class="whole-source" open>
      <summary>Complete source · read only</summary>
      <pre>{term.document.source}</pre>
    </details>{/if}
  {#each term.document.sections as section (section.id)}
    {#if section.editable}
      <section class="content-section" aria-label={section.heading}>
        {#if section.level === 2}<h2 data-protected-heading>
            {section.heading}<LockKeyhole size={12} aria-label="Protected heading" />
          </h2>{:else}<h3 data-protected-heading>
            {section.heading}<LockKeyhole size={11} aria-label="Protected heading" />
          </h3>{/if}
        <SectionEditor
          {section}
          onready={() => sectionReady(section.id)}
          readonly={disabled}
          sourceMode={sourceSections.includes(section.id)}
          onmodechange={(source) => {
            sourceSections = source
              ? [...new Set([...sourceSections, section.id])]
              : sourceSections.filter((id) => id !== section.id);
            uiChanged();
          }}
          onchange={(markdown) => onedit({ kind: 'section', id: section.id, markdown })}
        />
      </section>
    {/if}
  {/each}
  <div class="document-end">
    <button class="text-button" onclick={onadd} {disabled}
      ><Plus size={15} /> Add alternative definition</button
    ><span><LockKeyhole size={11} /> Required headings are protected</span>
  </div>
  {#if term.document.warnings.length}<details class="document-warnings">
      <summary
        >{term.document.warnings.length} content {term.document.warnings.length === 1
          ? 'note'
          : 'notes'}</summary
      >
      <ul>
        {#each term.document.warnings as warning, i (i)}<li>{warning}</li>{/each}
      </ul>
    </details>{/if}
</article>
