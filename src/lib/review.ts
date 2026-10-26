import DOMPurify from 'dompurify';
import { diffWordsWithSpace } from 'diff';
import type { ReviewPage } from './types';
export interface ReviewContext {
  repository: string;
  sha: string;
  path: string;
}
/** One sanitization policy for both source HTML and diff markup, including the final HTML sink. */
export function cleanReviewHtml(html: string, context?: ReviewContext): string {
  const clean = DOMPurify.sanitize(html, {
    ALLOWED_TAGS: [
      'p',
      'h1',
      'h2',
      'h3',
      'h4',
      'h5',
      'h6',
      'a',
      'strong',
      'b',
      'em',
      'i',
      'u',
      's',
      'del',
      'ins',
      'blockquote',
      'code',
      'pre',
      'ul',
      'ol',
      'li',
      'table',
      'thead',
      'tbody',
      'tfoot',
      'tr',
      'th',
      'td',
      'caption',
      'br',
      'hr',
      'sup',
      'sub',
      'span',
      'div',
      'mark',
    ],
    ALLOWED_ATTR: ['href', 'title', 'colspan', 'rowspan', 'start', 'align', 'class'],
    ALLOW_DATA_ATTR: false,
    ALLOW_ARIA_ATTR: false,
  });
  const document = new DOMParser().parseFromString(clean, 'text/html');
  for (const element of document.body.querySelectorAll('[class]')) {
    const classes =
      element.tagName === 'MARK'
        ? [...element.classList].filter((c) => c === 'diff-added' || c === 'diff-removed')
        : [];
    element.removeAttribute('class');
    if (classes.length) element.setAttribute('class', classes.join(' '));
  }
  for (const anchor of document.body.querySelectorAll('a[href]')) {
    try {
      const url = new URL(
        anchor.getAttribute('href')!,
        context
          ? `https://github.com/${context.repository}/blob/${context.sha}/${context.path}`
          : 'https://github.com/',
      );
      if (!['https:', 'http:'].includes(url.protocol)) {
        anchor.removeAttribute('href');
        continue;
      }
      anchor.setAttribute('href', url.href);
      anchor.setAttribute('rel', 'noopener noreferrer');
    } catch {
      anchor.removeAttribute('href');
    }
  }
  return document.body.innerHTML;
}
interface Interval {
  start: number;
  end: number;
}
function markText(html: string, intervals: Interval[], kind: 'added' | 'removed'): string {
  const document = new DOMParser().parseFromString(html, 'text/html');
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  const nodes: Text[] = [];
  while (walker.nextNode()) nodes.push(walker.currentNode as Text);
  let position = 0;
  for (const node of nodes) {
    const value = node.data;
    const end = position + value.length;
    const hits = intervals.filter((r) => r.end > position && r.start < end);
    if (hits.length) {
      const fragment = document.createDocumentFragment();
      let cursor = 0;
      for (const hit of hits) {
        const start = Math.max(hit.start - position, 0);
        const stop = Math.min(hit.end - position, value.length);
        fragment.append(value.slice(cursor, start));
        const mark = document.createElement('mark');
        mark.className = `diff-${kind}`;
        mark.textContent = value.slice(start, stop);
        fragment.append(mark);
        cursor = stop;
      }
      fragment.append(value.slice(cursor));
      node.replaceWith(fragment);
    }
    position = end;
  }
  return document.body.innerHTML;
}
export function compareHtml(
  before: string,
  after: string,
  context?: ReviewContext,
): { before: string; after: string; changed: boolean } {
  const cleanBefore = cleanReviewHtml(before, context);
  const cleanAfter = cleanReviewHtml(after, context);
  if (cleanBefore === cleanAfter) return { before: cleanBefore, after: cleanAfter, changed: false };
  const text = (html: string) =>
    new DOMParser().parseFromString(html, 'text/html').body.textContent ?? '';
  const oldText = text(cleanBefore);
  const newText = text(cleanAfter);
  const parts = diffWordsWithSpace(oldText, newText, { timeout: 60 });
  const removed: Interval[] = [];
  const added: Interval[] = [];
  let oldPosition = 0;
  let newPosition = 0;
  if (parts) {
    for (const part of parts) {
      if (part.removed) removed.push({ start: oldPosition, end: oldPosition + part.value.length });
      if (part.added) added.push({ start: newPosition, end: newPosition + part.value.length });
      if (!part.added) oldPosition += part.value.length;
      if (!part.removed) newPosition += part.value.length;
    }
  } else {
    removed.push({ start: 0, end: oldText.length });
    added.push({ start: 0, end: newText.length });
  }
  return {
    before: markText(cleanBefore, removed, 'removed'),
    after: markText(cleanAfter, added, 'added'),
    changed: true,
  };
}
export function compareSections(
  before: ReviewPage | null,
  after: ReviewPage,
  context: ReviewContext,
) {
  const old = new Map(before?.sections.map((s) => [s.key, s]) ?? []);
  const sections = after.sections.map((section) => {
    const previous = old.get(section.key);
    old.delete(section.key);
    const comparison = compareHtml(previous?.html ?? '', section.html, context);
    return {
      ...section,
      ...comparison,
      changed: !previous || comparison.changed,
      added: !previous,
      removed: false,
    };
  });
  // Heading removal is uncommon (headings are protected), but imported PRs must still display it.
  for (const section of old.values())
    sections.push({
      ...section,
      ...compareHtml(section.html, '', context),
      changed: true,
      added: false,
      removed: true,
    });
  return sections;
}
