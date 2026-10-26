export function zoomShortcut(
  event: Pick<KeyboardEvent, 'ctrlKey' | 'metaKey' | 'key' | 'code'>,
  current: number,
): number | null {
  if (!event.ctrlKey && !event.metaKey) return null;
  if (['+', '='].includes(event.key) || ['Equal', 'NumpadAdd'].includes(event.code))
    return Math.min(200, current + 10);
  if (event.key === '-' || ['Minus', 'NumpadSubtract'].includes(event.code))
    return Math.max(75, current - 10);
  if (event.key === '0' || ['Digit0', 'Numpad0'].includes(event.code)) return 100;
  return null;
}
