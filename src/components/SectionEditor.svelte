<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { CrepeBuilder } from '@milkdown/crepe/builder';
  import { listItem } from '@milkdown/crepe/feature/list-item';
  import { linkTooltip } from '@milkdown/crepe/feature/link-tooltip';
  import { placeholder } from '@milkdown/crepe/feature/placeholder';
  import { table } from '@milkdown/crepe/feature/table';
  import { serializerCtx } from '@milkdown/kit/core';
  import { callCommand, replaceAll, $prose as prosePlugin } from '@milkdown/kit/utils';
  import { toggleStrongCommand, toggleEmphasisCommand } from '@milkdown/kit/preset/commonmark';
  import { Plugin } from '@milkdown/kit/prose/state';
  import type { Section } from '../lib/types';
  let {
    section,
    onchange,
    readonly = false,
    sourceMode = false,
    onmodechange,
    onready,
  }: {
    section: Section;
    onchange: (markdown: string) => void;
    readonly?: boolean;
    sourceMode?: boolean;
    onmodechange?: (source: boolean) => void;
    onready?: () => void;
  } = $props();
  let root: HTMLDivElement;
  let crepe: CrepeBuilder | undefined;
  let mode = $state(untrack(() => (!section.richSafe || sourceMode ? 'source' : 'rich')));
  let text = $state(untrack(() => section.markdown));
  let ready = $state(false);
  let editorError = $state('');
  $effect(() => {
    if (ready && crepe) crepe.setReadonly(readonly);
  });
  onMount(() => {
    if (!section.richSafe) {
      ready = true;
      onready?.();
      return;
    }
    let disposed = false;
    const instance = new CrepeBuilder({ root, defaultValue: section.markdown })
      .addFeature(listItem)
      .addFeature(linkTooltip)
      .addFeature(table)
      .addFeature(placeholder, {
        text: `Add ${section.heading.toLowerCase().replace(/^\d+ /, '')}…`,
      });
    crepe = instance;
    instance.setReadonly(true);

    // Install persistence before the view becomes editable. The library's debounced
    // listener can otherwise miss the final keystroke when switching tabs quickly.
    instance.editor.use(
      prosePlugin(
        (ctx) =>
          new Plugin({
            filterTransaction(transaction) {
              let valid = true;
              transaction.doc.descendants((node) => {
                if (
                  (node.type.name === 'heading' && Number(node.attrs.level) <= 3) ||
                  node.type.name === 'hr'
                )
                  valid = false;
              });
              return valid;
            },
            view() {
              return {
                update(view, previous) {
                  if (ready && !disposed && !readonly && !previous.doc.eq(view.state.doc)) {
                    const serialize = ctx.get(serializerCtx);
                    const markdown = serialize(view.state.doc);
                    // List/table plugins can append an empty paragraph when the view
                    // first receives a selection. That is not an authored source edit.
                    if (markdown.trim() === serialize(previous.doc).trim()) return;
                    text = markdown;
                    onchange(text);
                  }
                },
              };
            },
          }),
      ),
    );
    void instance
      .create()
      .then(() => {
        if (disposed) void instance.destroy();
        else {
          ready = true;
          instance.setReadonly(readonly);
          onready?.();
        }
      })
      .catch((error) => {
        editorError = String(error);
        mode = 'source';
        onready?.();
      });
    return () => {
      disposed = true;
      ready = false;
      if (instance.editor.status === 'Created') void instance.destroy();
    };
  });
  function toggle() {
    if (mode === 'rich') {
      mode = 'source';
    } else {
      crepe?.editor.action(replaceAll(text));
      mode = 'rich';
    }
    onmodechange?.(mode === 'source');
  }
  function sourceInput(value: string) {
    text = value;
    onchange(value);
  }
  function format(kind: 'bold' | 'italic') {
    crepe?.editor.action(
      callCommand(kind === 'bold' ? toggleStrongCommand.key : toggleEmphasisCommand.key),
    );
  }
</script>

<div class="section-editor" class:source-mode={mode === 'source'}>
  <div class="section-tools" aria-label={`${section.heading} formatting`}>
    {#if mode === 'rich'}<button
        title="Bold"
        aria-label={`Bold in ${section.heading}`}
        onmousedown={(e) => e.preventDefault()}
        disabled={!ready || readonly}
        onclick={() => format('bold')}><strong>B</strong></button
      ><button
        title="Italic"
        aria-label={`Italic in ${section.heading}`}
        onmousedown={(e) => e.preventDefault()}
        disabled={!ready || readonly}
        onclick={() => format('italic')}><em>I</em></button
      >{/if}
    <button
      class="source-toggle"
      onclick={toggle}
      disabled={!section.richSafe || readonly || !ready}
      >{mode === 'source' ? 'Markdown' : 'Source'}</button
    >
  </div>
  {#if !section.richSafe}<p class="source-hint">
      This section contains source syntax that is kept intact. Edit its Markdown below.
    </p>{/if}
  {#if editorError}<p class="source-hint">
      Rich editing could not load. Markdown editing remains available.
    </p>{/if}
  <div bind:this={root} class:hidden={mode === 'source'}></div>
  {#if mode === 'source'}<textarea
      aria-label={`${section.heading} Markdown`}
      value={text}
      oninput={(e) => sourceInput(e.currentTarget.value)}
      disabled={readonly}
      spellcheck="false"
      rows={Math.min(18, Math.max(4, text.split('\n').length + 1))}
    ></textarea>{/if}
</div>
