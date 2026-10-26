import { it, expect } from 'vitest';
import { zoomShortcut } from './zoom';
const key = (key: string, ctrlKey = true, metaKey = false, code = '') => ({
  key,
  ctrlKey,
  metaKey,
  code,
});
it('supports control and command plus/minus/reset with bounded persisted percentages', () => {
  expect(zoomShortcut(key('+'), 100)).toBe(110);
  expect(zoomShortcut(key('='), 100)).toBe(110);
  expect(zoomShortcut(key('-', false, true), 100)).toBe(90);
  expect(zoomShortcut(key('0'), 145)).toBe(100);
  expect(zoomShortcut(key('+'), 200)).toBe(200);
  expect(zoomShortcut(key('-'), 75)).toBe(75);
  expect(zoomShortcut(key('+', false), 100)).toBeNull();
  expect(zoomShortcut(key('Add', true, false, 'NumpadAdd'), 100)).toBe(110);
  expect(zoomShortcut(key('Dead', true, false, 'Equal'), 100)).toBe(110);
  expect(zoomShortcut(key('_', true, false, 'Minus'), 100)).toBe(90);
  expect(zoomShortcut(key(')', true, false, 'Digit0'), 140)).toBe(100);
});
