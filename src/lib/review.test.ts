// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { cleanReviewHtml, compareHtml, compareSections } from './review';
const context = {
  repository: 'do-me/eo-glossary',
  sha: 'a'.repeat(40),
  path: 'docs/terms/accuracy.md',
};
const doc = (html: string) => new DOMParser().parseFromString(html, 'text/html');
describe('rendered terminology review', () => {
  it('highlights changed words without flattening emphasis, links or lists', () => {
    const result = compareHtml(
      '<p>A <strong>precise</strong> definition of <a href="https://example.org">accuracy</a>.</p><ul><li>Unchanged source</li></ul>',
      '<p>A <strong>clearer</strong> definition of <a href="https://example.org">accuracy</a>.</p><ul><li>Unchanged source</li></ul>',
      context,
    );
    expect(result.changed).toBe(true);
    expect(doc(result.before).querySelector('strong mark.diff-removed')?.textContent).toBe(
      'precise',
    );
    expect(doc(result.after).querySelector('strong mark.diff-added')?.textContent).toBe('clearer');
    expect(doc(result.after).querySelector('li')?.textContent).toBe('Unchanged source');
    expect(doc(result.after).querySelector('a')?.getAttribute('href')).toBe('https://example.org/');
  });
  it('rejects active HTML, unsafe URLs and UI styling while preserving scientific markup', () => {
    const safe = cleanReviewHtml(
      '<script>alert(1)</script><img src=x onerror=alert(2)><iframe src="https://evil.example"></iframe><form><input name=location></form><p class="modal" style="position:fixed" onclick="steal()">H<sub>2</sub>O</p><a href="jav&#97;script:alert(3)">bad</a><a href="../reference.md">citation</a><svg onload=alert(4)></svg><table><tr><td>25 °C</td></tr></table>',
      context,
    );
    const rendered = doc(safe);
    expect(
      rendered.querySelector('script,img,iframe,form,input,svg,[style],[onclick],.modal'),
    ).toBeNull();
    expect(rendered.querySelector('sub')?.textContent).toBe('2');
    expect(rendered.querySelector('a')?.hasAttribute('href')).toBe(false);
    expect(rendered.querySelectorAll('a')[1].getAttribute('href')).toBe(
      `https://github.com/do-me/eo-glossary/blob/${context.sha}/docs/reference.md`,
    );
    expect(rendered.querySelector('td')?.textContent).toBe('25 °C');
  });
  it('retains Unicode and newly added or removed definition sections', () => {
    const before = {
      title: 'Term',
      description: '',
      tags: [],
      sections: [
        { key: 'one', heading: '1 Definition', level: 2, html: '<p>λ = 2 μm 🌍</p>' },
        { key: 'old', heading: 'Sources', level: 3, html: '<p>Old source</p>' },
      ],
    };
    const after = {
      ...before,
      sections: [
        { ...before.sections[0], html: '<p>λ = 3 μm 🌍</p>' },
        { key: 'new', heading: '2 Definition', level: 2, html: '<p>An alternative</p>' },
      ],
    };
    const result = compareSections(before, after, context);
    expect(result).toHaveLength(3);
    expect(doc(result[0].after).body.textContent).toBe('λ = 3 μm 🌍');
    expect(result.find((s) => s.key === 'old')?.removed).toBe(true);
    expect(
      doc(result.find((s) => s.key === 'new')!.after).querySelector('mark')?.textContent,
    ).toContain('An');
  });
  it('recognizes added empty skeleton sections as structural changes', () => {
    const before = { title: 'Term', description: '', tags: [], sections: [] };
    const after = {
      ...before,
      sections: [{ key: '2:Definition', heading: '2 Definition', level: 2, html: '' }],
    };
    const sections = compareSections(before, after, context);
    expect(sections[0].changed).toBe(true);
    expect(sections[0].added).toBe(true);
  });
});
