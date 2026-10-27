import { test, expect, type Page } from '@playwright/test';

async function freshSession(page: Page, name: string) {
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Sessions', exact: true })).toBeEnabled();
  if (await page.getByRole('dialog').count())
    await page.getByRole('button', { name: 'Close dialog', exact: true }).click();
  await page.getByRole('button', { name: 'Sessions', exact: true }).click();
  await page.getByRole('button', { name: 'New session', exact: true }).click();
  await page.getByLabel('Session name').fill(name);
  await page.getByRole('button', { name: 'Create session', exact: true }).click();
  await expect(page.getByRole('button', { name, exact: true })).toBeVisible();
}
test('personal token sign-in is primary, handles failure visibly, and retains optional app setup', async ({
  page,
}) => {
  await freshSession(page, `Token sign-in ${Date.now()}`);
  await page.getByRole('button', { name: 'Account and settings', exact: true }).click();
  const form = page.getByRole('form', { name: 'Personal access token sign-in' });
  await expect(form).toBeVisible();
  await expect(form.getByRole('button', { name: 'Create a token on GitHub' })).toBeVisible();
  const token = form.getByLabel('Personal access token', { exact: true });
  await expect(token).toHaveAttribute('type', 'password');
  await expect(page.getByLabel('Public client ID')).toBeHidden();
  await expect(page.getByRole('button', { name: 'Sign in with registered app' })).toHaveCount(0);
  await token.fill('invalid-fixture-token');
  await form.getByRole('button', { name: 'Sign in with token', exact: true }).click();
  await expect(form.getByRole('alert')).toContainText('The token was not accepted');
  await expect(token).toHaveValue('');
  await token.fill('fixture-token-do-not-use');
  await form.getByRole('button', { name: 'Sign in with token', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await page.getByRole('button', { name: 'Account and settings', exact: true }).click();
  await expect(page.getByText('@fixture-expert', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Sign out', exact: true }).click();
  await expect(form).toBeVisible();
  await expect(token).toHaveValue('');
  await page.screenshot({ path: '.cache/token-sign-in.png', fullPage: true });
  await page.getByText('Advanced: optional app sign-in', { exact: true }).click();
  await expect(page.getByLabel('Public client ID')).toBeVisible();
  await page.getByRole('button', { name: 'Close dialog', exact: true }).click();
});

test('expert workflow: skeleton, protected structure, real SQLite autosave, tabs and session recovery', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(e.message));
  const originalSession = `Expert review ${Date.now()}`;
  await freshSession(page, originalSession);
  await expect(page.getByRole('button', { name: 'New term', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'New term', exact: true }).click();
  const skeleton = page.getByTestId('new-term-skeleton');
  await expect(skeleton).toContainText('## 1 Definition');
  await expect(skeleton).toContainText('### Sources');
  const title = `Test radiometry ${Date.now()}`;
  await page.getByLabel('Term name').fill(title);
  await page.getByRole('dialog').getByLabel('WGCV', { exact: true }).check();
  await page.getByRole('button', { name: 'Create term', exact: true }).click();
  await expect(page.getByRole('heading', { name: title, exact: true })).toBeVisible();
  await expect(page.locator('[data-protected-heading]')).toHaveCount(5);
  expect(
    await page
      .locator('[data-protected-heading]')
      .evaluateAll((nodes) => nodes.every((n) => !(n as HTMLElement).isContentEditable)),
  ).toBe(true);
  const definition = page
    .getByRole('region', { name: '1 Definition', exact: true })
    .locator('[contenteditable="true"]');
  await expect(definition).toBeVisible();
  await definition.fill('An expert definition with preserved scientific meaning.');
  await expect(page.getByText('Saved locally', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Details', exact: true }).click();
  await page.getByLabel('Description', { exact: true }).fill('Locally durable test term');
  await expect(page.getByText('Saved locally', { exact: true })).toBeVisible();
  await page.getByRole('textbox', { name: 'Search terms', exact: true }).fill('accuracy');
  await page
    .locator('.term-result')
    .filter({ has: page.locator('.result-title', { hasText: /^Accuracy$/ }) })
    .click();
  await expect(page.getByRole('tab')).toHaveCount(2);
  await page.getByRole('tab', { name: title }).click();
  await expect(definition).toContainText('An expert definition');
  await page.reload();
  await expect(page.getByRole('heading', { name: title, exact: true })).toBeVisible();
  await expect(definition).toContainText('An expert definition');
  await page.getByRole('button', { name: `Close ${title}`, exact: true }).click();
  await page.getByRole('textbox', { name: 'Search terms', exact: true }).fill(title);
  await page.locator('.term-result').click();
  await expect(definition).toContainText('An expert definition');
  await page.getByRole('button', { name: 'Review changes' }).click();
  await expect(page.locator('.change-card').filter({ hasText: title })).toContainText(
    'An expert definition',
  );
  await page.getByRole('button', { name: 'Sessions', exact: true }).click();
  await page.getByRole('button', { name: 'New session', exact: true }).click();
  const sessionName = `Independent ${Date.now()}`;
  await page.getByLabel('Session name').fill(sessionName);
  await page.getByRole('button', { name: 'Create session', exact: true }).click();
  await expect(page.getByRole('button', { name: sessionName, exact: true })).toBeVisible();
  await page.getByRole('textbox', { name: 'Search terms', exact: true }).fill(title);
  await expect(page.locator('.term-result')).toHaveCount(0);
  await page.getByRole('button', { name: 'Sessions', exact: true }).click();
  await page.locator('.session-row').filter({ hasText: originalSession }).click();
  await expect(page.getByRole('heading', { name: title, exact: true })).toBeVisible();
  await expect(definition).toContainText('An expert definition');
  await page.screenshot({ path: '.cache/editor-workspace.png', fullPage: true });
  expect(errors).toEqual([]);
});

test('rapid tab switching flushes the latest keystroke and source cannot replace headings', async ({
  page,
}) => {
  await freshSession(page, `Rapid session ${Date.now()}`);
  await expect(page.getByRole('button', { name: 'New term', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'New term', exact: true }).click();
  const title = `Rapid edit ${Date.now()}`;
  await page.getByLabel('Term name').fill(title);
  await page.getByRole('button', { name: 'Create term', exact: true }).click();
  await expect(page.getByRole('heading', { name: title, exact: true })).toBeVisible();
  const region = page.getByRole('region', { name: '1 Definition', exact: true });
  const definition = region.locator('[contenteditable=true]');
  await definition.fill('The latest keystroke must survive.');
  // Navigate without waiting for the autosave debounce.
  await page.getByRole('textbox', { name: 'Search terms', exact: true }).fill('precision');
  await page
    .locator('.term-result')
    .filter({ has: page.locator('.result-title', { hasText: /^Precision$/ }) })
    .click();
  await page.getByRole('tab', { name: title }).click();
  await expect(definition).toContainText('The latest keystroke must survive.');
  await region.hover();
  await region.getByRole('button', { name: 'Source', exact: true }).click();
  const source = page.getByLabel('1 Definition Markdown', { exact: true });
  await source.fill('## A forbidden replacement heading');
  await expect(page.getByRole('alert')).toContainText('headings');
  await expect(region.locator('h2')).toHaveText('1 Definition');
  await source.fill('A valid source edit with **scientific precision**.');
  await expect(page.getByText('Saved locally', { exact: true })).toBeVisible();
  await page.reload();
  await expect(page.getByRole('heading', { name: title, exact: true })).toBeVisible();
  await expect(definition).toContainText('A valid source edit');
  expect(
    await page
      .locator('[data-protected-heading]')
      .evaluateAll((nodes) => nodes.every((n) => !(n as HTMLElement).isContentEditable)),
  ).toBe(true);
});

test('browsing representative real terms produces no saved changes', async ({ page }) => {
  await freshSession(page, `Read-only session ${Date.now()}`);
  for (const title of ['Accuracy', 'Resolution', 'Analysis Ready Data']) {
    await page.getByRole('textbox', { name: 'Search terms', exact: true }).fill(title);
    await page
      .locator('.term-result')
      .filter({ has: page.locator('.result-title', { hasText: new RegExp('^' + title + '$') }) })
      .click();
    await expect(page.getByRole('heading', { name: title, exact: true })).toBeVisible();
    await expect(
      page.locator('.content-section').first().locator('[contenteditable],textarea'),
    ).toBeVisible();
  }
  await page.getByRole('button', { name: 'Review changes' }).click();
  await expect(page.locator('.change-card')).toHaveCount(0);
  await expect(
    page.getByRole('heading', { name: 'Everything is saved', exact: true }),
  ).toBeVisible();
});

async function chooseRepository(page: Page, repository: string) {
  await page.getByRole('button', { name: 'Account and settings', exact: true }).click();
  const input = page.getByLabel('Repository URL', { exact: true });
  const target = `https://github.com/${repository}`;
  if ((await input.inputValue()) === target) {
    await page.getByRole('button', { name: 'Close dialog', exact: true }).click();
    return;
  }
  await input.fill(target);
  await page.getByRole('button', { name: 'Use repository', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.locator('.repository-indicator')).toHaveText(repository);
}

test('repository settings preserve earlier drafts and pin submission destinations', async ({
  page,
}) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Sessions', exact: true })).toBeEnabled();
  if (await page.getByRole('dialog').count())
    await page.getByRole('button', { name: 'Close dialog', exact: true }).click();
  await chooseRepository(page, 'ceos-org/eo-glossary');
  const originalName = `Pinned repository ${Date.now()}`;
  await freshSession(page, originalName);
  await page.getByRole('textbox', { name: 'Search terms', exact: true }).fill('accuracy');
  await page
    .locator('.term-result')
    .filter({ has: page.locator('.result-title', { hasText: /^Accuracy$/ }) })
    .click();
  const definition = page
    .getByRole('region', { name: '1 Definition', exact: true })
    .locator('[contenteditable=true]');
  await definition.fill('A saved draft for the original repository.');
  await expect(page.getByText('Saved locally', { exact: true })).toBeVisible();
  await chooseRepository(page, 'do-me/eo-glossary');
  await page.getByRole('textbox', { name: 'Search terms', exact: true }).fill('accuracy');
  await page
    .locator('.term-result')
    .filter({ has: page.locator('.result-title', { hasText: /^Accuracy$/ }) })
    .click();
  await expect(definition).not.toContainText('A saved draft for the original repository.');
  await page.getByRole('button', { name: 'Sessions', exact: true }).click();
  await page.locator('.session-row').filter({ hasText: originalName }).click();
  await expect(definition).toContainText('A saved draft for the original repository.');
  await expect(page.locator('.repository-indicator')).toHaveText('ceos-org/eo-glossary');
  await page.getByRole('button', { name: 'Review changes' }).click();
  await expect(page.locator('.page-title')).toContainText('Destination: ceos-org/eo-glossary');
  await freshSession(page, `New fork work ${Date.now()}`);
  await expect(page.locator('.repository-indicator')).toHaveText('do-me/eo-glossary');
  await page.reload();
  await expect(page.locator('.repository-indicator')).toHaveText('do-me/eo-glossary');
});

test('review renders documents with word highlights, clean preview and side-by-side comparison', async ({
  page,
}) => {
  await freshSession(page, `Rendered review ${Date.now()}`);
  await page.getByRole('textbox', { name: 'Search terms', exact: true }).fill('accuracy');
  await page
    .locator('.term-result')
    .filter({ has: page.locator('.result-title', { hasText: /^Accuracy$/ }) })
    .click();
  await expect(page.getByRole('heading', { name: 'Accuracy', exact: true })).toBeVisible();
  await expect(page.getByText('CEOS · EARTH OBSERVATION GLOSSARY', { exact: true })).toHaveCount(0);
  const region = page.getByRole('region', { name: '1 Definition', exact: true });
  await region.hover();
  await region.getByRole('button', { name: 'Source', exact: true }).click();
  await page
    .getByLabel('1 Definition Markdown', { exact: true })
    .fill('A **precise** definition with a [citation](https://example.org).');
  await expect(page.getByText('Saved locally', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Review changes' }).click();
  const rendered = page.locator('.rendered-change');
  await expect(rendered).toBeVisible();
  await expect(rendered.locator('.revision-after strong')).toHaveText('precise');
  await expect(rendered.locator('.diff-added').first()).toBeVisible();
  await expect(rendered.locator('.revision-before .diff-removed').first()).toBeVisible();
  await expect(rendered.locator('.revision-after a')).toHaveAttribute(
    'href',
    'https://example.org/',
  );
  await expect(rendered).not.toContainText('**precise**');
  await expect(rendered).not.toContainText('tags:');
  await page
    .getByRole('group', { name: 'Review display' })
    .getByRole('button', { name: 'Preview', exact: true })
    .click();
  await expect(rendered.locator('.revision-before')).toHaveCount(0);
  await expect(rendered.locator('strong').filter({ hasText: 'precise' })).toBeVisible();
  await expect(rendered.locator('mark')).toHaveCount(0);
  await page.getByRole('button', { name: 'Side by side', exact: true }).click();
  await expect(rendered.locator('article')).toHaveCount(2);
  await expect(page.getByRole('article', { name: 'Accuracy Before' })).toBeVisible();
  await expect(
    page
      .getByRole('article', { name: 'Accuracy After' })
      .locator('strong')
      .filter({ hasText: 'precise' }),
  ).toBeVisible();
  await page.screenshot({ path: '.cache/review-rendered.png', fullPage: true });
});

async function acknowledgeWorkspace(page: Page) {
  const written = page.waitForResponse((r) => {
    try {
      return r.request().postDataJSON().name === 'workspace' && r.ok();
    } catch {
      return false;
    }
  });
  await page.keyboard.press('Control+s');
  await written;
}
async function remoteMainEdit(page: Page, repository: string, id: number, markdown: string) {
  const response = await page.request.post('http://127.0.0.1:1421', {
    headers: { Origin: 'http://127.0.0.1:1420', 'X-Test-Token': 'local-e2e-only' },
    data: {
      name: 'fixture_main_update',
      data: { repository, path: 'docs/terms/accuracy.md', edit: { kind: 'section', id, markdown } },
    },
  });
  expect(response.ok()).toBe(true);
}
async function openAccuracy(page: Page) {
  await page.getByRole('textbox', { name: 'Search terms', exact: true }).fill('accuracy');
  await page
    .locator('.term-result')
    .filter({ has: page.locator('.result-title', { hasText: /^Accuracy$/ }) })
    .click();
  await expect(page.getByRole('heading', { name: 'Accuracy', exact: true })).toBeVisible();
}

test('zoom, metadata/source panels, filters and review state survive reopening', async ({
  page,
}) => {
  await freshSession(page, `Restore state ${Date.now()}`);
  await openAccuracy(page);
  await page.getByRole('button', { name: 'Details', exact: true }).click();
  await page.getByRole('button', { name: 'Filters', exact: true }).click();
  const region = page.getByRole('region', { name: '1 Definition', exact: true });
  await region.hover();
  await region.getByRole('button', { name: 'Source', exact: true }).click();
  await expect(page.getByLabel('1 Definition Markdown', { exact: true })).toBeVisible();
  await page.keyboard.press('Control+=');
  await page.keyboard.press('Control+=');
  await expect(page.getByRole('button', { name: 'Reset zoom to 100%' })).toHaveText('120%');
  const editor = page.locator('.editor-scroll');
  await editor.evaluate((node) => {
    node.scrollTop = 420;
  });
  await expect.poll(() => editor.evaluate((node) => node.scrollTop)).toBeGreaterThan(400);
  await acknowledgeWorkspace(page);
  await page.reload();
  await expect.poll(() => editor.evaluate((node) => node.scrollTop)).toBeGreaterThan(400);
  await expect(page.getByRole('button', { name: 'Reset zoom to 100%' })).toHaveText('120%');
  await expect(page.getByLabel('1 Definition Markdown', { exact: true })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Term details' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'WGCV', exact: true })).toBeVisible();
  await page.getByLabel('1 Definition Markdown', { exact: true }).fill('A source edit to review.');
  await expect(page.getByText('Saved locally', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Review changes' }).click();
  await page.getByRole('button', { name: 'Preview', exact: true }).click();
  await page.getByLabel('Contribution title', { exact: true }).fill('My saved contribution title');
  await page.getByLabel('Description', { exact: true }).fill('My saved rationale.');
  await acknowledgeWorkspace(page);
  await page.reload();
  await expect(page.getByRole('button', { name: 'Preview', exact: true })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await expect(page.getByLabel('Contribution title', { exact: true })).toHaveValue(
    'My saved contribution title',
  );
  await expect(page.getByLabel('Description', { exact: true })).toHaveValue('My saved rationale.');
  await page.getByRole('button', { name: 'Reset zoom to 100%' }).click();
});

test('startup fetch detects main conflicts and both explicit resolution choices persist', async ({
  page,
}) => {
  await freshSession(page, `Conflicting main ${Date.now()}`);
  await openAccuracy(page);
  const repository = (await page.locator('.repository-indicator').innerText()).trim();
  const definition = page
    .getByRole('region', { name: '1 Definition', exact: true })
    .locator('[contenteditable=true]');
  await definition.fill('My saved scientific definition.');
  await expect(page.getByText('Saved locally', { exact: true })).toBeVisible();
  await acknowledgeWorkspace(page);
  await remoteMainEdit(page, repository, 1, 'The definition merged from another PR.');
  await page.reload();
  await expect(page.getByRole('button', { name: 'Review differences', exact: true })).toBeVisible();
  await expect(definition).toContainText('My saved scientific definition.');
  await page.getByRole('button', { name: 'Review differences', exact: true }).click();
  const conflict = page.getByRole('region', { name: 'Conflict: Accuracy' });
  await expect(conflict).toContainText('The definition merged from another PR.');
  await expect(conflict).toContainText('My saved scientific definition.');
  await conflict.getByRole('button', { name: 'Use GitHub version', exact: true }).click();
  await expect(definition).toContainText('The definition merged from another PR.');
  await definition.fill('My second local version.');
  await expect(page.getByText('Saved locally', { exact: true })).toBeVisible();
  await acknowledgeWorkspace(page);
  await remoteMainEdit(page, repository, 1, 'A second approved main version.');
  await page.reload();
  await page.getByRole('button', { name: 'Review differences', exact: true }).click();
  await page.getByRole('button', { name: 'Keep my version', exact: true }).click();
  await expect(definition).toContainText('My second local version.');
  await acknowledgeWorkspace(page);
  await page.reload();
  await expect(definition).toContainText('My second local version.');
  await expect(page.getByRole('button', { name: 'Review differences', exact: true })).toHaveCount(
    0,
  );
});

test('startup combines another merged PR with a non-overlapping local draft', async ({ page }) => {
  await freshSession(page, `Main reconciliation ${Date.now()}`);
  await openAccuracy(page);
  const repository = (await page.locator('.repository-indicator').innerText()).trim();
  const definition = page
    .getByRole('region', { name: '1 Definition', exact: true })
    .locator('[contenteditable=true]');
  await definition.fill('My independent definition edit.');
  await expect(page.getByText('Saved locally', { exact: true })).toBeVisible();
  await acknowledgeWorkspace(page);
  await remoteMainEdit(page, repository, 2, 'An independent note merged on GitHub.');
  await page.reload();
  await expect(definition).toContainText('My independent definition edit.');
  await expect(page.getByRole('region', { name: 'Notes', exact: true }).first()).toContainText(
    'An independent note merged on GitHub.',
  );
  await expect(page.getByRole('button', { name: 'Review differences', exact: true })).toHaveCount(
    0,
  );
});

async function fixtureCommand(page: Page, name: string, data = {}) {
  const response = await page.request.post('http://127.0.0.1:1421', {
    headers: { Origin: 'http://127.0.0.1:1420', 'X-Test-Token': 'local-e2e-only' },
    data: { name, data },
  });
  expect(response.ok()).toBe(true);
  return response.json();
}

test('submitted contribution has one durable summary, full review, and separate unsent edits', async ({
  page,
}) => {
  await freshSession(page, `Submitted review ${Date.now()}`);
  await openAccuracy(page);
  const definition = page
    .getByRole('region', { name: '1 Definition', exact: true })
    .locator('[contenteditable=true]');
  await definition.fill('A submitted scientific definition.');
  await expect(page.getByText('Saved locally', { exact: true })).toBeVisible();
  await acknowledgeWorkspace(page);
  await fixtureCommand(page, 'fixture_sign_in');
  await page.reload();
  await page.getByRole('button', { name: 'Review changes' }).click();
  await page.getByRole('button', { name: 'Submit for review', exact: true }).click();
  const summary = page.getByRole('region', { name: 'Pull request status' });
  await expect(summary.getByRole('heading')).toHaveText('Contribution submitted');
  await expect(summary).toBeInViewport();
  await expect(summary).toContainText('1 term submitted to GitHub');
  await expect(summary.getByRole('button', { name: 'View PR #9001' })).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Full contribution 1', exact: true }),
  ).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('.change-card')).toContainText('A submitted scientific definition.');
  await expect(page.getByRole('heading', { name: 'Everything is saved' })).toHaveCount(0);
  await expect(page.locator('.submission-form')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Export', exact: true })).toBeVisible();
  await acknowledgeWorkspace(page);
  await page.reload();
  await expect(summary.getByRole('heading')).toHaveText('Contribution submitted');
  await expect(summary).toBeInViewport();
  await expect(page.locator('.change-card')).toHaveCount(1);
  await page.getByRole('button', { name: 'Not submitted 0', exact: true }).click();
  await expect(page.locator('.change-card')).toHaveCount(0);
  await expect(page.getByRole('status').filter({ hasText: 'No edits waiting' })).toBeVisible();
  await summary.getByRole('button', { name: 'Continue editing' }).click();
  await definition.fill('A newer local definition, not submitted yet.');
  await expect(page.getByText('Saved locally', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Review changes' }).click();
  await expect(summary.getByRole('heading')).toHaveText('New edits to send');
  await expect(page.getByRole('button', { name: 'Not submitted 1', exact: true })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await expect(page.getByRole('button', { name: 'Update submission', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Update submission', exact: true }).click();
  await expect(summary.getByRole('heading')).toHaveText('Contribution submitted');
  await expect(summary).toBeInViewport();
  await expect(page.locator('.submission-form')).toHaveCount(0);
  await page.screenshot({ path: '.cache/submission-complete.png', fullPage: true });

  // A merged contribution remains reviewable, while further work moves to a follow-up.
  const boot = await fixtureCommand(page, 'bootstrap');
  await fixtureCommand(page, 'fixture_pr_state', { session: boot.activeSession, state: 'merged' });
  await page.reload();
  await expect(summary.getByRole('heading')).toHaveText('Contribution merged');
  await expect(summary.getByRole('button', { name: 'Start follow-up' })).toBeVisible();
  await expect(page.locator('.submission-form')).toHaveCount(0);
});
