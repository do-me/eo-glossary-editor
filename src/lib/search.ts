import type { Term, Workspace } from './types';
const normalize = (text: string) =>
  text
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase();
export function findTerms(terms: Term[], workspace: Workspace): Term[] {
  const query = normalize(workspace.query.trim());
  const words = query.split(/\s+/).filter(Boolean);
  const groups = [workspace.teams, workspace.classes, workspace.statuses];
  return terms
    .filter(
      (t) =>
        (!workspace.changedOnly || t.changed) &&
        groups.every((g) => !g.length || g.some((tag) => t.tags.includes(tag))) &&
        words.every((w) => normalize(t.searchText).includes(w)),
    )
    .map((t) => ({
      t,
      score: !query
        ? 3
        : normalize(t.title) === query
          ? 0
          : normalize(t.title).startsWith(query)
            ? 1
            : normalize(t.title).includes(query)
              ? 2
              : 3,
    }))
    .sort((a, b) => a.score - b.score || a.t.title.localeCompare(b.t.title))
    .map((x) => x.t);
}
