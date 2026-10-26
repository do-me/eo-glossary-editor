import { it, expect } from 'vitest';
import { findTerms } from './search';
import type { Term, Workspace } from './types';
const workspace: Workspace = {
  tabs: [],
  pinned: [],
  active: null,
  query: '',
  teams: [],
  classes: [],
  statuses: [],
  changedOnly: false,
  scroll: {},
};
const term = (title: string, tags: string[], searchText = title): Term => ({
  title,
  tags,
  searchText,
  path: title,
  description: '',
  changed: false,
  pending: false,
  isNew: false,
});
it('ranks exact titles and ANDs groups while ORing teams', () => {
  const terms = [
    term('Accuracy', ['WGCV', 'core', 'to be discussed']),
    term('Angular accuracy', ['LSI-VC', 'core', 'to be discussed']),
    term('Radiometric quality', ['WGCV', 'base', 'approved'], 'Accuracy synonym'),
  ];
  expect(findTerms(terms, { ...workspace, query: 'accuracy' }).map((t) => t.title)).toEqual([
    'Accuracy',
    'Angular accuracy',
    'Radiometric quality',
  ]);
  expect(
    findTerms(terms, {
      ...workspace,
      teams: ['WGCV', 'LSI-VC'],
      classes: ['core'],
      statuses: ['to be discussed'],
    }).length,
  ).toBe(2);
  expect(findTerms(terms, { ...workspace, changedOnly: true })).toEqual([]);
});
