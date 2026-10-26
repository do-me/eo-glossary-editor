<script lang="ts">
  import { SvelteMap } from 'svelte/reactivity';
  import { onMount, tick } from 'svelte';
  import {
    Search,
    ListFilter,
    BookOpen,
    Layers,
    Settings,
    Plus,
    X,
    ArrowUpRight,
    Check,
    Circle,
    CloudUpload,
    RefreshCw,
    Download,
    Upload,
    Pin,
    ChevronLeft,
    ChevronRight,
    AlertCircle,
    Github,
    FileText,
    ArrowLeft,
    Ellipsis,
  } from '@lucide/svelte';
  import GitHubUpdates from './components/GitHubUpdates.svelte';
  import ZoomControl from './components/ZoomControl.svelte';
  import { zoomShortcut } from './lib/zoom';
  import RenderedChange from './components/RenderedChange.svelte';
  import TermEditor from './components/TermEditor.svelte';
  import { api, message, native, openLink } from './lib/api';
  import { SaveQueue } from './lib/save-queue';
  import { findTerms } from './lib/search';
  import type {
    Account,
    Bootstrap,
    Session,
    SessionSummary,
    Term,
    OpenTerm,
    Workspace,
    Edit,
    Change,
    PublicPr,
    ReviewMode,
    AppView,
    TermUi,
    SyncReport,
    SyncConflict,
  } from './lib/types';
  let boot = $state<Bootstrap>();
  let session = $state<Session>();
  let sessions = $state<SessionSummary[]>([]);
  let terms = $state<Term[]>([]);
  let document = $state<OpenTerm>();
  let documentReady = $state(false);
  let view = $state<AppView>('terms');
  let error = $state('');
  let notice = $state('');
  let busy = $state('');
  let loading = $state(true);
  let filters = $state(false);
  let epoch = $state(0);
  let account = $state<Account | null>(null);
  let saveState = $state<'saved' | 'saving' | 'error'>('saved');
  let modal = $state<'new-term' | 'new-session' | 'account' | 'rename' | 'revert' | null>(null);
  let search = $state<HTMLInputElement>();
  let title = $state('');
  let sessionName = $state('');
  let newTeams = $state<string[]>([]);
  let changes = $state<Change[]>([]);
  let allChanges = $state(false);
  let restoring = $state(false);
  let syncApplying = $state(false);
  let syncing = $state(false);
  let syncState = $state<'idle' | 'checking' | 'current' | 'offline'>('idle');
  let syncReport = $state<SyncReport | null>(null);
  let zoom = $state(100);
  let zoomWrite = Promise.resolve();
  let syncTask: Promise<void> | null = null;
  let pageArea = $state<HTMLElement>();
  let sidebarArea = $state<HTMLDivElement>();
  let tabsArea = $state<HTMLDivElement>();

  let reviewMode = $state<ReviewMode>('changes');
  let repositoryUrl = $state('');
  let contributionTitle = $state('');
  let contributionBody = $state('');
  let submitResult = $state('');
  let pat = $state('');
  let clientId = $state('');
  let authKind = $state<'github_app' | 'oauth_app'>('github_app');
  let remember = $state(true);
  let device = $state<{ userCode: string; verificationUrl: string; interval: number } | null>(null);
  let prs = $state<PublicPr[]>([]);
  let clientConfigured = $state(false);
  let workspace = $state<Workspace>({
    tabs: [],
    pinned: [],
    active: null,
    query: '',
    teams: [],
    classes: [],
    statuses: [],
    changedOnly: false,
    scroll: {},
  });
  const statuses = new SvelteMap<string, 'saving' | 'saved' | 'error'>();
  const queues = new SvelteMap<string, SaveQueue>();
  const documents = new SvelteMap<string, OpenTerm>();
  let workspaceTimer: ReturnType<typeof setTimeout>;
  let authTimer: ReturnType<typeof setTimeout>;
  let lastClosed: string | undefined;
  let scrollArea = $state<HTMLDivElement>();
  let shuttingDown = false;
  let openingSequence = 0;
  const results = $derived(findTerms(terms, workspace));
  const pending = $derived(terms.filter((t) => t.pending).length);
  const changed = $derived(terms.filter((t) => t.changed).length);
  const activeFilters = $derived(
    workspace.teams.length +
      workspace.classes.length +
      workspace.statuses.length +
      Number(workspace.changedOnly),
  );
  const currentReadonly = $derived(
    Boolean(session?.submission && session.submission.state !== 'open'),
  );
  const skeleton = $derived(
    `---\ntitle: ${title.trim() || 'New term'}\ndescription: ""\ntags:\n  - core\n  - to be defined${newTeams.map((t) => '\n  - ' + t).join('')}\n---\n\n# ${title.trim() || 'New term'}\n\n## 1 Definition\n\n### Notes\n\n### Examples\n\n### Sources\n`,
  );
  function fail(e: unknown) {
    error = message(e);
  }
  async function perform(label: string, action: () => Promise<void>) {
    if (busy || syncApplying) return;
    busy = label;
    error = '';
    try {
      await action();
    } catch (e) {
      fail(e);
    } finally {
      busy = '';
    }
  }
  async function startup() {
    try {
      boot = await api<Bootstrap>('bootstrap');
      repositoryUrl = `https://github.com/${boot.catalogue.repository}`;
      zoom = boot.preferences.zoom;
      applyPreviewZoom();
      sessions = boot.sessions;
      const auth = await api<{
        account: Account | null;
        clientIdConfigured: boolean;
        kind?: 'github_app' | 'oauth_app';
      }>('account');
      account = auth.account;
      clientConfigured = auth.clientIdConfigured;
      authKind = auth.kind ?? 'github_app';
      const id =
        boot.activeSession && sessions.some((s) => s.id === boot?.activeSession)
          ? boot.activeSession
          : sessions[0]?.id;
      if (id) await loadSession(id);
      else {
        const s = await api<Session>('new_session', { name: 'Terminology review' });
        await loadSession(s.id);
      }
    } catch (e) {
      fail(e);
    } finally {
      loading = false;
    }
    if (session) void syncNow();
  }
  onMount(() => {
    void startup();
    let unlisten: (() => void) | undefined;
    let unlistenQuit: (() => void) | undefined;
    let disposed = false;
    async function quit() {
      if (shuttingDown) return;
      shuttingDown = true;
      try {
        await flush();
        await zoomWrite;
        await api('quit');
      } catch (e) {
        shuttingDown = false;
        fail(e);
      }
    }
    if (native)
      void Promise.all([import('@tauri-apps/api/window'), import('@tauri-apps/api/event')])
        .then(async ([{ getCurrentWindow }, { listen }]) => {
          unlisten = await getCurrentWindow().onCloseRequested((event) => {
            event.preventDefault();
            void quit();
          });
          unlistenQuit = await listen('quit-requested', () => void quit());
          if (disposed) {
            unlisten();
            unlistenQuit();
          }
        })
        .catch(fail);
    return () => {
      disposed = true;
      unlisten?.();
      unlistenQuit?.();
      clearTimeout(workspaceTimer);
      clearTimeout(authTimer);
    };
  });
  async function loadSession(id: string, preserved?: Workspace) {
    clearTimeout(workspaceTimer);
    restoring = true;
    openingSequence++;
    try {
      const next = await api<Session>('session', { session: id });
      session = next;
      workspace = preserved
        ? {
            ...preserved,
            tabs: preserved.tabs.filter((p) => next.workspace.tabs.includes(p)),
            active:
              preserved.active && next.workspace.tabs.includes(preserved.active)
                ? preserved.active
                : next.workspace.active,
          }
        : next.workspace;
      const savedView = workspace.view ?? 'terms';
      filters = workspace.filtersOpen ?? false;
      reviewMode = ['changes', 'preview', 'compare'].includes(workspace.reviewMode ?? '')
        ? workspace.reviewMode!
        : 'changes';
      allChanges = workspace.allChanges ?? false;
      contributionTitle = workspace.contributionTitle ?? '';
      contributionBody = workspace.contributionBody ?? '';
      title = workspace.newTermTitle ?? '';
      newTeams = workspace.newTermTeams ?? [];
      sessionName = workspace.sessionName ?? '';
      modal = ['new-term', 'new-session', 'rename', 'account'].includes(workspace.dialog ?? '')
        ? (workspace.dialog as typeof modal)
        : null;
      documents.clear();
      queues.clear();
      statuses.clear();
      originalSources.clear();
      document = undefined;
      saveState = 'saved';
      view = 'terms';
      terms = await api<Term[]>('terms', { session: id });
      if (workspace.active && terms.some((t) => t.path === workspace.active))
        await openTerm(workspace.active, false);
      syncReport = await api<SyncReport>('sync_report', { session: id });
      if (savedView === 'review') {
        changes = await api<Change[]>('changes', { session: id, all: allChanges });
      }
      view = ['terms', 'sessions', 'review', 'updates'].includes(savedView) ? savedView : 'terms';
      sessions = await api<SessionSummary[]>('sessions');
      await tick();
      restoreScroll();
      await saveWorkspace();
    } finally {
      restoring = false;
    }
  }
  function captureWorkspace(): Workspace {
    const saved = $state.snapshot(workspace);
    if (documentReady && document && scrollArea) saved.scroll[document.path] = scrollArea.scrollTop;
    return {
      ...saved,
      view,
      filtersOpen: filters,
      reviewMode,
      allChanges,
      contributionTitle,
      contributionBody,
      dialog: modal === 'revert' ? null : modal,
      newTermTitle: title,
      newTermTeams: [...newTeams],
      sessionName,
    };
  }
  function restoreScroll() {
    if (scrollArea && workspace.active)
      scrollArea.scrollTop = workspace.scroll[workspace.active] ?? 0;
    if (sidebarArea) sidebarArea.scrollTop = workspace.sidebarScroll ?? 0;
    if (tabsArea) tabsArea.scrollLeft = workspace.tabsScroll ?? 0;
    if (pageArea) pageArea.scrollTop = workspace.viewScroll?.[view] ?? 0;
  }
  function savePageScroll() {
    if (pageArea) {
      workspace.viewScroll ??= {};
      workspace.viewScroll[view] = pageArea.scrollTop;
      workspaceChanged();
    }
  }
  $effect(() => {
    const stateKey = JSON.stringify([
      view,
      filters,
      reviewMode,
      allChanges,
      contributionTitle,
      contributionBody,
      modal,
      title,
      newTeams,
      sessionName,
    ]);
    if (stateKey && !loading && !restoring && session) workspaceChanged();
  });
  $effect(() => {
    const route = view;
    if (!loading && !restoring)
      void tick().then(() => {
        if (view === route) restoreScroll();
      });
  });
  function termUi(path: string): TermUi {
    return workspace.termUi?.[path] ?? { details: false, source: false, sourceSections: [] };
  }
  function updateTermUi(path: string, state: TermUi) {
    workspace.termUi ??= {};
    workspace.termUi[path] = state;
    workspaceChanged();
  }
  function applyPreviewZoom() {
    if (!native) {
      window.document.body.classList.add('browser-zoom');
      window.document.documentElement.style.setProperty('--preview-zoom', String(zoom / 100));
    }
  }
  function setZoom(value: number) {
    zoom = Math.max(75, Math.min(200, Math.round(value)));
    applyPreviewZoom();
    const selected = zoom;
    zoomWrite = zoomWrite
      .then(async () => {
        await api('save_preferences', { preferences: { zoom: selected } });
      })
      .catch(fail);
  }
  async function syncNow(force = false): Promise<void> {
    if (syncTask) {
      await syncTask;
      if (force) return syncNow(true);
      return;
    }
    if (!session) return;
    const id = session.id;
    syncTask = (async () => {
      syncing = true;
      syncState = 'checking';
      try {
        const latest = await api<{ sha: string }>('check_updates', { session: id });
        if (session?.id !== id) return;
        if (busy && !force) {
          syncState = 'idle';
          return;
        }
        const changedBase = latest.sha !== session.baseSha;
        if (changedBase) {
          syncApplying = true;
          await tick();
        }
        await flush();
        if (session?.id !== id) return;
        const report = await api<SyncReport>('apply_updates', { session: id, sha: latest.sha });
        if (session?.id !== id) return;
        if (changedBase) {
          await loadSession(id, captureWorkspace());
        } else {
          const fresh = await api<Session>('session', { session: id });
          if (session?.id === id) session = fresh;
        }
        syncReport = report;
        boot = await api<Bootstrap>('bootstrap');
        syncState = 'current';
        if (report.conflicts.length)
          notice = `GitHub ${report.branch} changed. ${report.conflicts.length} ${report.conflicts.length === 1 ? 'term needs' : 'terms need'} your choice. Your drafts are retained.`;
        else if (report.merged || report.updated)
          notice = `Latest ${report.branch} loaded. ${report.merged} local ${report.merged === 1 ? 'draft was' : 'drafts were'} combined with non-overlapping GitHub changes.`;
        if (report.deferred)
          notice =
            'Latest main downloaded. Recover the interrupted submission before applying it to this session.';
      } catch (e) {
        syncState = 'offline';
        notice = `Could not check the latest GitHub version. Your cached terms and local edits are available. ${message(e)}`;
      } finally {
        syncApplying = false;
        syncing = false;
      }
    })();
    try {
      await syncTask;
    } finally {
      syncTask = null;
    }
  }
  async function resolveUpdate(conflict: SyncConflict, choice: 'github' | 'local') {
    await perform('Saving your choice', async () => {
      syncApplying = true;
      try {
        await tick();
        await flush();
        const report = await api<SyncReport>('resolve_update', {
          session: session!.id,
          path: conflict.path,
          revision: conflict.revision,
          sha: conflict.githubSha,
          choice,
        });
        const id = session!.id;
        await loadSession(id);
        syncReport = report;
        view = report.conflicts.length ? 'updates' : 'terms';
        notice =
          choice === 'github'
            ? 'GitHub version applied. Your earlier draft is retained in local revision history.'
            : 'Your version is kept. It will be reviewed against the latest main when you submit.';
      } catch (e) {
        if (session) syncReport = await api<SyncReport>('sync_report', { session: session.id });
        throw e;
      } finally {
        syncApplying = false;
      }
    });
  }
  async function switchSession(id: string) {
    await perform('Opening session', async () => {
      await flush();
      await loadSession(id);
      // Selecting a session is an explicit request to resume editing. Startup alone
      // restores the exact previous page and dialog.
      view = 'terms';
      modal = null;
      await saveWorkspace();
      void syncNow();
    });
  }
  async function saveWorkspace() {
    clearTimeout(workspaceTimer);
    if (!session) return;
    await api('workspace', { session: session.id, workspace: captureWorkspace() });
  }
  function workspaceChanged() {
    clearTimeout(workspaceTimer);
    workspaceTimer = setTimeout(() => void saveWorkspace().catch(fail), 250);
  }
  async function flush() {
    clearTimeout(workspaceTimer);
    await Promise.all([...queues.values()].map((q) => q.flush()));
    await saveWorkspace();
  }
  function queueFor(term: OpenTerm): SaveQueue {
    const existing = queues.get(term.path);
    if (existing) return existing;
    const id = session!.id;
    const path = term.path;
    const q = new SaveQueue(
      term.revision,
      (revision, edit) => api<OpenTerm>('edit', { session: id, path, revision, edit }),
      (saved) => {
        documents.set(path, saved);
        if (document?.path === path) document = saved;
        const old = terms.find((t) => t.path === path);
        const initial = session!.terms[path];
        // Compute changed/pending from acknowledged source, never from a timer or input event.
        if (old) {
          old.title = saved.document.title;
          old.tags = saved.document.tags;
          old.description = saved.document.description;
          old.searchText = saved.document.source;
          old.changed =
            saved.isNew ||
            saved.document.source !== (initial?.starting ?? originalSources.get(path));
          old.pending =
            saved.document.source !==
            (initial?.submitted ?? initial?.starting ?? originalSources.get(path));
          terms = [...terms];
        }
      },
      (state, e) => {
        statuses.set(path, state);
        saveState = [...statuses.values()].includes('error')
          ? 'error'
          : [...statuses.values()].includes('saving')
            ? 'saving'
            : 'saved';
        if (e) fail(e);
      },
    );
    queues.set(path, q);
    return q;
  }
  const originalSources = new SvelteMap<string, string>();
  async function openTerm(path: string, persist = true) {
    if (syncApplying && !restoring) return;
    const sequence = ++openingSequence;
    const sessionId = session!.id;
    try {
      if (document) {
        await queues.get(document.path)?.flush();
        if (documentReady && scrollArea) workspace.scroll[document.path] = scrollArea.scrollTop;
      }
      let term = documents.get(path);
      if (!term) {
        term = await api<OpenTerm>('open', { session: sessionId, path });
        if (sequence !== openingSequence || session?.id !== sessionId) return;
        documents.set(path, term);
        if (!session!.terms[path]) originalSources.set(path, term.document.source);
      }
      if (sequence !== openingSequence || session?.id !== sessionId) return;
      if (!workspace.tabs.includes(path)) workspace.tabs.push(path);
      workspace.active = path;
      documentReady = false;
      document = term;
      view = 'terms';
      epoch++;
      if (persist) workspaceChanged();
      await tick();
      if (scrollArea) scrollArea.scrollTop = workspace.scroll[path] ?? 0;
    } catch (e) {
      fail(e);
    }
  }
  function edit(edit: Edit) {
    if (document && !currentReadonly) queueFor(document).enqueue(edit);
  }
  async function closeTab(path: string) {
    try {
      await queues.get(path)?.flush();
      lastClosed = path;
      const i = workspace.tabs.indexOf(path);
      workspace.tabs = workspace.tabs.filter((p) => p !== path);
      workspace.pinned = workspace.pinned.filter((p) => p !== path);
      if (workspace.active === path) {
        const next = workspace.tabs[Math.min(i, workspace.tabs.length - 1)];
        if (next) await openTerm(next);
        else {
          workspace.active = null;
          document = undefined;
        }
      }
      workspaceChanged();
    } catch (e) {
      fail(e);
    }
  }
  function pin(path: string) {
    workspace.pinned = workspace.pinned.includes(path)
      ? workspace.pinned.filter((p) => p !== path)
      : [...workspace.pinned, path];
    workspaceChanged();
  }
  function moveTab(path: string, delta: number) {
    const i = workspace.tabs.indexOf(path);
    const j = i + delta;
    if (j < 0 || j >= workspace.tabs.length) return;
    const tabs = [...workspace.tabs];
    [tabs[i], tabs[j]] = [tabs[j], tabs[i]];
    workspace.tabs = tabs;
    workspaceChanged();
  }
  function setFilter(group: 'teams' | 'classes' | 'statuses', tag: string) {
    workspace[group] = workspace[group].includes(tag)
      ? workspace[group].filter((t) => t !== tag)
      : [...workspace[group], tag];
    workspaceChanged();
  }
  async function newTerm() {
    await perform('Creating term', async () => {
      await flush();
      const term = await api<OpenTerm>('new_term', {
        session: session!.id,
        title,
        tags: ['core', 'to be defined', ...newTeams],
      });
      terms = await api<Term[]>('terms', { session: session!.id });
      await openTerm(term.path);
      modal = null;
    });
  }
  async function newSession() {
    await perform('Creating session', async () => {
      await flush();
      const s = await api<Session>('new_session', { name: sessionName });
      modal = null;
      await loadSession(s.id);
    });
  }
  async function addDefinition() {
    await perform('Adding definition', async () => {
      await flush();
      const term = document!;
      document = await api<OpenTerm>('edit', {
        session: session!.id,
        path: term.path,
        revision: term.revision,
        edit: { kind: 'add_definition' },
      });
      documents.set(term.path, document);
      queues.delete(term.path);
      epoch++;
      terms = await api<Term[]>('terms', { session: session!.id });
    });
  }
  async function revert() {
    await perform('Reverting term', async () => {
      await flush();
      const path = document!.path;
      await api('revert', { session: session!.id, path, revision: document!.revision });
      documents.delete(path);
      queues.delete(path);
      terms = await api<Term[]>('terms', { session: session!.id });
      modal = null;
      if (terms.some((t) => t.path === path)) await openTerm(path);
      else await closeTab(path);
    });
  }
  async function review() {
    await perform('Preparing review', async () => {
      await flush();
      changes = await api<Change[]>('changes', { session: session!.id, all: allChanges });
      contributionTitle ||= session!.name;
      view = 'review';
    });
  }
  async function submit() {
    await perform('Submitting to GitHub', async () => {
      await syncNow(true);
      if (syncReport?.conflicts.length) {
        view = 'updates';
        return;
      }
      await flush();
      const link = await api<{ url: string }>('submit', {
        session: session!.id,
        title: contributionTitle,
        body: contributionBody,
      });
      submitResult = link.url;
      await flush();
      const id = session!.id;
      await loadSession(id);
      changes = await api<Change[]>('changes', { session: id, all: allChanges });
      view = 'review';
    });
  }
  async function changeRepository() {
    await perform('Opening repository', async () => {
      await flush();
      const result = await api<{ session: Session; bootstrap: Bootstrap }>('configure_repository', {
        url: repositoryUrl,
      });
      await flush();
      boot = result.bootstrap;
      repositoryUrl = `https://github.com/${boot.catalogue.repository}`;
      prs = [];
      changes = [];
      contributionTitle = '';
      contributionBody = '';
      submitResult = '';
      await loadSession(result.session.id);
      modal = null;
      notice = `Now using ${result.session.repository}. Your other sessions are preserved with their original repositories.`;
    });
  }
  async function refresh() {
    await syncNow();
  }
  async function exportSession() {
    await perform('Exporting session', async () => {
      await flush();
      if (native) await api('export_file', { session: session!.id });
      else {
        const content = await api<string>('export', { session: session!.id });
        const blob = new Blob([content], { type: 'application/json' });
        const a = documentElement('a');
        a.href = URL.createObjectURL(blob);
        a.download = 'glossary-session.json';
        a.click();
        setTimeout(() => URL.revokeObjectURL(a.href), 1000);
      }
      notice = 'Session exported without credentials.';
    });
  }
  function documentElement(tag: 'a') {
    return window.document.createElement(tag);
  }
  async function importSession() {
    await perform('Importing session', async () => {
      await flush();
      if (native) {
        const s = await api<Session | null>('import_file');
        if (s) await loadSession(s.id);
      } else notice = 'Use the local CLI to import a session in the development harness.';
    });
  }
  async function tokenSignIn() {
    const secret = pat;
    pat = '';
    await perform('Authenticating', async () => {
      account = await api<Account>('sign_in_token', { token: secret, remember });
      modal = null;
      notice = `Signed in as ${account.login}.`;
    });
  }
  async function beginAuth() {
    await perform('Starting GitHub authentication', async () => {
      device = await api('auth_begin', { remember });
      await openLink(device!.verificationUrl);
      authTimer = setTimeout(() => void pollAuth(), device!.interval * 1000);
    });
  }
  async function pollAuth() {
    try {
      const result = await api<{ account?: Account; pending?: boolean }>('auth_poll');
      if (result.account) {
        account = result.account;
        device = null;
        modal = null;
        notice = `Signed in as ${account.login}.`;
      } else if (device) authTimer = setTimeout(() => void pollAuth(), device.interval * 1000);
    } catch (e) {
      fail(e);
      device = null;
    }
  }
  async function closeModal() {
    if (device) {
      clearTimeout(authTimer);
      await api('auth_cancel').catch(fail);
      device = null;
    }
    pat = '';
    modal = null;
  }
  async function followUp() {
    await perform('Starting follow-up', async () => {
      await flush();
      const s = await api<Session>('follow_up', { session: session!.id });
      await loadSession(s.id);
      notice = 'Follow-up session created. The original session is retained.';
    });
  }
  async function resumePr(number: number) {
    await perform('Loading GitHub contribution', async () => {
      await flush();
      const s = await api<Session>('import_pr', { number });
      await loadSession(s.id);
    });
  }
  async function keydown(event: KeyboardEvent) {
    const nextZoom = zoomShortcut(event, zoom);
    if (nextZoom !== null) {
      event.preventDefault();
      setZoom(nextZoom);
      return;
    }
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      view = 'terms';
      await tick();
      search?.focus();
      search?.select();
    }
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
      event.preventDefault();
      await flush().catch(fail);
    }
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'w' && document) {
      event.preventDefault();
      await closeTab(document.path);
    }
    if (
      (event.metaKey || event.ctrlKey) &&
      event.shiftKey &&
      event.key.toLowerCase() === 't' &&
      lastClosed
    ) {
      event.preventDefault();
      await openTerm(lastClosed);
    }
    if (event.key === 'Escape' && modal) await closeModal();
  }
  function focusScope(node: HTMLElement) {
    const previous = window.document.activeElement as HTMLElement | null;
    const focusable = () =>
      [
        ...node.querySelectorAll<HTMLElement>(
          'button:not(:disabled),input:not(:disabled),textarea:not(:disabled),select:not(:disabled),summary,[tabindex="0"]',
        ),
      ].filter((e) => e.getClientRects().length);
    queueMicrotask(() => {
      const preferred = node.querySelector<HTMLElement>('input:not([type=checkbox])');
      (preferred ?? focusable()[0] ?? node).focus();
    });
    const trap = (e: KeyboardEvent) => {
      if (e.key !== 'Tab') return;
      const elements = focusable();
      const first = elements[0];
      const last = elements.at(-1);
      if (e.shiftKey && window.document.activeElement === first) {
        e.preventDefault();
        last?.focus();
      } else if (!e.shiftKey && window.document.activeElement === last) {
        e.preventDefault();
        first?.focus();
      }
    };
    node.addEventListener('keydown', trap);
    return {
      destroy() {
        node.removeEventListener('keydown', trap);
        previous?.focus();
      },
    };
  }
  function interceptLink(event: MouseEvent) {
    const target = event.target;
    if (target instanceof Element) {
      const anchor = target.closest('a');
      if (anchor?.href) {
        event.preventDefault();
        void openLink(anchor.href).catch(fail);
      }
    }
  }
</script>

<svelte:window
  onkeydown={keydown}
  ononline={() => (notice = 'Connection restored.')}
  onoffline={() => (notice = 'Offline · your edits continue saving locally.')}
/>
<div class="app-shell" onclick={interceptLink} role="presentation">
  <nav class="rail" aria-label="Main navigation">
    <div class="brand" title="EO Glossary Editor">eo<span>·</span></div>
    <button
      class:chosen={view === 'terms'}
      disabled={loading || syncApplying || restoring}
      aria-label="Terms"
      title="Terms"
      onclick={() => (view = 'terms')}><BookOpen size={20} /></button
    ><button
      class:chosen={view === 'sessions'}
      disabled={loading || syncApplying || restoring}
      aria-label="Sessions"
      title="Sessions"
      onclick={() => {
        view = 'sessions';
        void api<SessionSummary[]>('sessions')
          .then((v) => (sessions = v))
          .catch(fail);
      }}><Layers size={20} /></button
    >
    <div class="rail-spacer"></div>
    <button
      disabled={loading || syncApplying || restoring}
      aria-label="Account and settings"
      title="Account and settings"
      onclick={() => (modal = 'account')}
      ><Settings size={19} />{#if account}<span class="account-dot"></span>{/if}</button
    >
  </nav>
  <div class="workspace" inert={syncApplying}>
    <header class="topbar">
      <div class="topbar-title">
        <span>EO Glossary</span>{#if session}<button
            class="repository-indicator"
            title="Session repository"
            onclick={() => void openLink(`https://github.com/${session!.repository}`).catch(fail)}
            >{session.repository}<ArrowUpRight size={11} /></button
          >{/if}<span class="slash">/</span><button
          class="session-title"
          onclick={() => {
            sessionName = session?.name ?? '';
            modal = 'rename';
          }}>{session?.name ?? 'Opening workspace'}</button
        >{#if session?.submission}<button
            class="pr-link"
            onclick={() => void openLink(session!.submission!.url).catch(fail)}
            >PR #{session.submission.number}<ArrowUpRight size={12} /></button
          >{/if}
      </div>
      <div class="topbar-actions">
        <span class="save-state" class:save-error={saveState === 'error'} aria-live="polite"
          >{#if saveState === 'saved'}<Check size={13} /> Saved locally{:else if saveState === 'saving'}<Circle
              size={10}
            /> Saving…{:else}<AlertCircle size={13} /> Not saved
            <button onclick={() => void flush().catch(fail)}>Retry</button>{/if}</span
        ><button class="primary" onclick={review} disabled={loading || !!busy}
          ><CloudUpload size={14} /> Review changes <span class="count">{pending}</span></button
        >
      </div>
    </header>
    {#if error}<div class="banner error" role="alert">
        <AlertCircle size={15} /><span>{error}</span><button
          aria-label="Dismiss error"
          onclick={() => (error = '')}><X size={15} /></button
        >
      </div>{/if}
    {#if notice}<div class="banner notice" role="status">
        <span>{notice}</span><button aria-label="Dismiss notification" onclick={() => (notice = '')}
          ><X size={14} /></button
        >
      </div>{/if}
    {#if syncReport?.conflicts.length && view !== 'updates'}<div
        class="banner update-banner"
        role="status"
      >
        <span
          >GitHub {syncReport.branch} has conflicting changes in {syncReport.conflicts.length}
          {syncReport.conflicts.length === 1 ? 'term' : 'terms'}.</span
        ><button onclick={() => (view = 'updates')}>Review differences</button>
      </div>{/if}
    {#if busy}<div class="busy-line" role="status">{busy}…</div>{/if}
    {#if loading}<main class="empty-state">
        <div class="empty-symbol"><BookOpen size={26} /></div>
        <h1>Opening your glossary</h1>
        <p>Your local sessions will be restored.</p>
      </main>
    {:else if view === 'sessions'}
      <main class="session-page" bind:this={pageArea} onscroll={savePageScroll}>
        <div class="page-title">
          <div>
            <p class="eyebrow">YOUR WORKSPACE</p>
            <h1>Sessions</h1>
            <p class="muted">Keep a contribution together, from first draft to review.</p>
          </div>
          <button
            class="primary"
            onclick={() => {
              sessionName = '';
              modal = 'new-session';
            }}><Plus size={15} /> New session</button
          >
        </div>
        <div class="section-label">
          <span>ON THIS DEVICE · {sessions.length}</span>
          <div>
            <button onclick={importSession}><Upload size={14} /> Import</button><button
              onclick={exportSession}><Download size={14} /> Export active</button
            >
          </div>
        </div>
        <div class="session-list">
          {#each sessions as s (s.id)}<button
              class="session-row"
              class:current={s.id === session?.id}
              onclick={() => switchSession(s.id)}
              ><Layers size={18} /><span
                ><strong>{s.name}</strong><small
                  >{s.repository} · {s.changed} changed terms · {s.pending} pending · {new Date(
                    s.updatedAt,
                  ).toLocaleDateString()}{s.interrupted ? ' · Submission interrupted' : ''}</small
                ></span
              >{#if s.submission}<span class="pill"
                  >PR #{s.submission.number} · {s.submission.state}</span
                >{:else}<span class="pill">Local draft</span>{/if}<ChevronRight size={16} /></button
            >{/each}
        </div>
        <div class="public-sessions">
          <div>
            <h2>Continue a GitHub contribution</h2>
            <p class="muted">Resume an open PR you authored in {boot?.catalogue.repository}.</p>
          </div>
          <button
            onclick={() =>
              perform('Finding your contributions', async () => {
                prs = await api<PublicPr[]>('authored_prs');
                if (!prs.length) notice = 'No open glossary PRs found for your account.';
              })}
            disabled={!account || !!busy}><Github size={15} /> Find my PRs</button
          >
        </div>
        {#each prs as pr (pr.number)}<button class="session-row" onclick={() => resumePr(pr.number)}
            ><Github size={18} /><span
              ><strong>{pr.title}</strong><small>#{pr.number} · {boot?.catalogue.repository}</small
              ></span
            ><ChevronRight size={16} /></button
          >{/each}
      </main>
    {:else if view === 'updates'}<GitHubUpdates
        report={syncReport}
        busy={!!busy || syncing}
        onback={() => (view = 'terms')}
        oncheck={() => void syncNow()}
        onresolve={resolveUpdate}
        onexport={exportSession}
        scrollTop={workspace.viewScroll?.updates ?? 0}
        onscroll={(value) => {
          workspace.viewScroll ??= {};
          workspace.viewScroll.updates = value;
          workspaceChanged();
        }}
      />
    {:else if view === 'review'}
      <main class="review-page" bind:this={pageArea} onscroll={savePageScroll}>
        <button class="back-button" onclick={() => (view = 'terms')}
          ><ArrowLeft size={15} /> Back to editing</button
        >
        <div class="page-title">
          <div>
            <p class="eyebrow">SESSION REVIEW</p>
            <h1>{session?.name}</h1>
            <p class="muted">
              {changes.length}
              {changes.length === 1 ? 'term' : 'terms'} in this review. Destination: {session?.repository}.
            </p>
          </div>
          <button onclick={exportSession}><Download size={14} /> Export</button>
        </div>
        {#if currentReadonly || session?.operation}<div class="notice banner">
            <span
              >{session?.operation
                ? 'An interrupted submission is retained for recovery. Retry it, or move pending work to a follow-up session.'
                : 'This contribution has closed. Continue pending work in a fresh session.'}</span
            ><button onclick={followUp} disabled={!!busy}>Start follow-up</button>
          </div>{/if}
        {#if session?.submission}<div class="review-options">
            <label
              ><input type="checkbox" bind:checked={allChanges} onchange={review} /> Show all changes
              since the session started</label
            ><button onclick={() => void openLink(session!.submission!.url).catch(fail)}
              >Open PR #{session.submission.number}<ArrowUpRight size={13} /></button
            >
          </div>{/if}
        {#if changes.length}<div class="review-toolbar">
            <div class="review-mode" role="group" aria-label="Review display">
              {#each [{ id: 'changes', label: 'Changes' }, { id: 'preview', label: 'Preview' }, { id: 'compare', label: 'Side by side' }] as choice (choice.id)}<button
                  aria-pressed={reviewMode === choice.id}
                  class:selected={reviewMode === choice.id}
                  onclick={() => (reviewMode = choice.id as ReviewMode)}>{choice.label}</button
                >{/each}
            </div>
            <span class="muted small">Green highlights additions · rose highlights removals</span>
          </div>{/if}
        {#each changes as change (change.path)}<details class="change-card" open>
            <summary
              ><FileText size={15} /><strong>{change.title}</strong><span class="pill"
                >{change.isNew ? 'New term' : 'Edited'}</span
              ><code>{change.path.replace('docs/terms/', '')}</code></summary
            >
            <RenderedChange
              {change}
              mode={reviewMode}
              repository={session!.repository}
              sha={session!.baseSha}
            />
            <details class="source-diff">
              <summary>View source diff</summary>
              <pre>{#each change.diff.split('\n') as line, i (i)}<span
                    class:added={line.startsWith('+')}
                    class:removed={line.startsWith('-')}>{line}&#10;</span
                  >{/each}</pre>
            </details>
            {#if change.warnings.length}<ul class="review-warnings">
                {#each change.warnings as warning, i (i)}<li>{warning}</li>{/each}
              </ul>{/if}
          </details>{/each}
        {#if !changes.length}<div class="review-empty">
            <Check size={24} />
            <h2>Everything is saved</h2>
            <p class="muted">
              {session?.submission
                ? 'No new local changes to submit.'
                : 'Edit a term to start your contribution.'}
            </p>
          </div>{/if}
        {#if pending || session?.operation}<section class="submission-form">
            <h2>{session?.submission ? 'Update your contribution' : 'Submit for review'}</h2>
            <p class="muted">
              {session?.submission
                ? `New changes will update PR #${session.submission.number}.`
                : `All changed terms go into one pull request in ${session?.repository}.`}
            </p>
            <label>Contribution title<input bind:value={contributionTitle} maxlength="250" /></label
            ><label
              >Description<textarea
                bind:value={contributionBody}
                rows="4"
                placeholder="Explain the terminology changes and the sources you used."
              ></textarea></label
            >{#if !account}<button class="primary" onclick={() => (modal = 'account')}
                ><Github size={15} /> Authenticate with GitHub</button
              >{:else}<div class="submit-footer">
                <span class="muted small">Submitting as @{account.login}</span><button
                  class="primary"
                  onclick={submit}
                  disabled={!!busy || !contributionTitle.trim()}
                  ><CloudUpload size={15} />{session?.operation
                    ? 'Recover submission'
                    : session?.submission
                      ? 'Update submission'
                      : 'Submit for review'}</button
                >
              </div>{/if}
          </section>{/if}
        {#if submitResult}<div class="success-message">
            <Check size={16} /> Contribution submitted.<button
              onclick={() => void openLink(submitResult).catch(fail)}
              >View on GitHub<ArrowUpRight size={13} /></button
            >
          </div>{/if}
      </main>
    {:else}
      <div class="editing-layout">
        <aside class="term-sidebar" aria-label="Find terms">
          <div class="sidebar-heading">
            <strong>Terms</strong><span>{boot?.catalogue.count}</span><button
              aria-label="New term"
              title="New term"
              onclick={() => {
                title = '';
                newTeams = [];
                modal = 'new-term';
              }}
              disabled={currentReadonly}><Plus size={17} /></button
            >
          </div>
          <div class="search-field">
            <Search size={15} /><input
              bind:this={search}
              bind:value={workspace.query}
              oninput={workspaceChanged}
              aria-label="Search terms"
              placeholder="Search terms…"
            /><kbd>⌘ K</kbd>
          </div>
          <div class="filter-bar">
            <button class:active={filters || activeFilters > 0} onclick={() => (filters = !filters)}
              ><ListFilter size={14} /> Filters{#if activeFilters}<span class="count"
                  >{activeFilters}</span
                >{/if}</button
            ><span>{results.length} results</span>
          </div>
          {#if filters}<div class="filters">
              <fieldset>
                <legend>Review team</legend>
                <div class="filter-tags">
                  {#each boot?.teams ?? [] as tag (tag)}<button
                      class:selected={workspace.teams.includes(tag)}
                      aria-pressed={workspace.teams.includes(tag)}
                      onclick={() => setFilter('teams', tag)}>{tag}</button
                    >{/each}
                </div>
              </fieldset>
              <fieldset>
                <legend>Discussion status</legend>{#each boot?.statuses ?? [] as tag (tag)}<label
                    ><input
                      type="checkbox"
                      checked={workspace.statuses.includes(tag)}
                      onchange={() => setFilter('statuses', tag)}
                    />{tag}</label
                  >{/each}
              </fieldset>
              <fieldset>
                <legend>Term class</legend>
                <div class="filter-tags">
                  {#each boot?.classes ?? [] as tag (tag)}<button
                      class:selected={workspace.classes.includes(tag)}
                      aria-pressed={workspace.classes.includes(tag)}
                      onclick={() => setFilter('classes', tag)}>{tag}</button
                    >{/each}
                </div>
              </fieldset>
              <label
                ><input
                  type="checkbox"
                  bind:checked={workspace.changedOnly}
                  onchange={workspaceChanged}
                /> Changed in this session</label
              ><button
                class="text-button"
                onclick={() => {
                  workspace.teams = [];
                  workspace.classes = [];
                  workspace.statuses = [];
                  workspace.changedOnly = false;
                  workspaceChanged();
                }}>Clear filters</button
              >
            </div>{/if}
          <div
            class="term-results"
            aria-label="Search results"
            bind:this={sidebarArea}
            onscroll={() => {
              if (sidebarArea) {
                workspace.sidebarScroll = sidebarArea.scrollTop;
                workspaceChanged();
              }
            }}
          >
            {#each results as term (term.path)}<button
                class="term-result"
                class:current={workspace.active === term.path}
                onclick={() => openTerm(term.path)}
                ><span class="result-title"
                  >{term.title}{#if term.changed}<span
                      class="change-dot"
                      title="Changed in this session"
                    ></span>{/if}</span
                ><span class="result-description"
                  >{term.description ||
                    term.tags.filter((t) => boot?.teams.includes(t)).join(' · ') ||
                    'Earth observation terminology'}</span
                ></button
              >{/each}{#if !results.length}<p class="no-results">
                No matching terms. Try another search or clear the filters.
              </p>{/if}
          </div>
          <div class="sidebar-footer">
            <span title={session?.baseSha}
              >Snapshot {session?.baseSha.slice(0, 7)}<small
                >{boot ? new Date(boot.catalogue.fetchedAt).toLocaleDateString() : ''}</small
              ></span
            ><button
              title="Download latest main"
              aria-label="Refresh glossary"
              disabled={!!busy}
              onclick={refresh}><RefreshCw size={14} /></button
            >
          </div>
        </aside>
        <main class="editor-pane">
          <div
            class="tabs"
            bind:this={tabsArea}
            onscroll={() => {
              if (tabsArea) {
                workspace.tabsScroll = tabsArea.scrollLeft;
                workspaceChanged();
              }
            }}
            role="tablist"
            aria-label="Open terms"
          >
            {#each [...workspace.tabs.filter( (p) => workspace.pinned.includes(p) ), ...workspace.tabs.filter((p) => !workspace.pinned.includes(p))] as path (path)}<div
                class="tab"
                class:active={workspace.active === path}
              >
                <button
                  role="tab"
                  aria-selected={workspace.active === path}
                  onclick={() => openTerm(path)}
                  >{#if workspace.pinned.includes(path)}<Pin size={11} />{:else}<FileText
                      size={13}
                    />{/if}{terms.find((t) => t.path === path)?.title ??
                    path.split('/').pop()}{#if terms.find((t) => t.path === path)?.pending}<span
                      class="change-dot"
                    ></span>{/if}</button
                >
                <details class="tab-menu">
                  <summary aria-label={`Options for ${terms.find((t) => t.path === path)?.title}`}
                    ><Ellipsis size={12} /></summary
                  >
                  <div>
                    <button onclick={() => pin(path)}
                      ><Pin size={12} />{workspace.pinned.includes(path) ? 'Unpin' : 'Pin'} tab</button
                    ><button onclick={() => moveTab(path, -1)}
                      ><ChevronLeft size={12} />Move left</button
                    ><button onclick={() => moveTab(path, 1)}
                      ><ChevronRight size={12} />Move right</button
                    >
                  </div>
                </details>
                <button
                  class="close-tab"
                  aria-label={`Close ${terms.find((t) => t.path === path)?.title}`}
                  onclick={() => closeTab(path)}><X size={12} /></button
                >
              </div>{/each}
          </div>
          <div
            class="editor-scroll"
            bind:this={scrollArea}
            onscroll={() => {
              if (documentReady && document && scrollArea) {
                workspace.scroll[document.path] = scrollArea.scrollTop;
                workspaceChanged();
              }
            }}
          >
            {#if document}{#key `${session?.id}:${document.path}:${epoch}`}<TermEditor
                  term={document}
                  onready={() =>
                    requestAnimationFrame(() => {
                      restoreScroll();
                      documentReady = true;
                    })}
                  teams={boot?.teams ?? []}
                  classes={boot?.classes ?? []}
                  statuses={boot?.statuses ?? []}
                  onedit={edit}
                  onadd={addDefinition}
                  onrevert={() => (modal = 'revert')}
                  readonly={currentReadonly}
                  locked={syncApplying}
                  ui={termUi(document.path)}
                  onuichange={(state) => document && updateTermUi(document.path, state)}
                />{/key}{:else}<div class="empty-state">
                <div class="empty-symbol"><BookOpen size={27} /></div>
                <p class="eyebrow">A SHARED LANGUAGE FOR EARTH OBSERVATION</p>
                <h1>Make the definition clearer.</h1>
                <p>
                  Find a term, open a few tabs, and work at your own pace.<br />Your edits save
                  locally as you go.
                </p>
                <div class="empty-actions">
                  <button class="primary" onclick={() => search?.focus()}
                    ><Search size={14} /> Find a term</button
                  ><button
                    onclick={() => {
                      title = '';
                      newTeams = [];
                      modal = 'new-term';
                    }}><Plus size={14} /> Create a term</button
                  >
                </div>
                <span class="empty-footnote">One session. Multiple terms. One contribution.</span>
              </div>{/if}
          </div>
        </main>
      </div>
    {/if}
    <footer class="editor-status app-status">
      <span class="sync-status"
        >{#if syncing}Checking GitHub…{:else if syncState === 'offline'}Using cached version <button
            onclick={() => void syncNow()}>Retry</button
          >{:else if syncReport?.conflicts.length}<button onclick={() => (view = 'updates')}
            >{syncReport.conflicts.length} conflicts to review</button
          >{:else if syncState === 'current'}GitHub {syncReport?.branch ?? 'main'} checked{:else}Local
          workspace{/if}</span
      ><span>{changed} changed · {pending} pending</span><ZoomControl {zoom} onchange={setZoom} />
    </footer>
  </div>
</div>
{#if modal}<div class="modal-backdrop" role="presentation">
    <div
      class="modal"
      use:focusScope
      class:wide={modal === 'new-term'}
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
      tabindex="-1"
    >
      <button class="modal-close" aria-label="Close dialog" onclick={closeModal}
        ><X size={18} /></button
      >
      {#if modal === 'new-term'}<p class="eyebrow">ADD TO THE GLOSSARY</p>
        <h1 id="modal-title">New term</h1>
        <p class="muted">
          The full term structure is ready. Start with a name, then fill in the definition.
        </p>
        <form
          onsubmit={(e) => {
            e.preventDefault();
            void newTerm();
          }}
        >
          <label
            >Term name<input
              bind:value={title}
              placeholder="e.g. Surface reflectance"
              maxlength="200"
              required
            /></label
          >
          <fieldset>
            <legend>Review teams <span class="muted">· optional now</span></legend>
            <div class="team-options">
              {#each boot?.teams ?? [] as team (team)}<label
                  ><input
                    type="checkbox"
                    checked={newTeams.includes(team)}
                    onchange={() =>
                      (newTeams = newTeams.includes(team)
                        ? newTeams.filter((t) => t !== team)
                        : [...newTeams, team])}
                  />{team}</label
                >{/each}
            </div>
          </fieldset>
          <details class="skeleton-preview" open>
            <summary>Term skeleton · included automatically</summary>
            <pre data-testid="new-term-skeleton">{skeleton}</pre>
          </details>
          <div class="modal-footer">
            <button type="button" onclick={closeModal}>Cancel</button><button
              type="submit"
              class="primary"
              disabled={!title.trim() || !!busy}>Create term<ChevronRight size={14} /></button
            >
          </div>
        </form>
      {:else if modal === 'new-session' || modal === 'rename'}<h1 id="modal-title">
          {modal === 'rename' ? 'Rename session' : 'New session'}
        </h1>
        <p class="muted">A session keeps related term edits together in one contribution.</p>
        <form
          onsubmit={(e) => {
            e.preventDefault();
            if (modal === 'new-session') void newSession();
            else
              void perform('Renaming session', async () => {
                await api('rename_session', { session: session!.id, name: sessionName });
                session!.name = sessionName;
                modal = null;
              });
          }}
        >
          <label
            >Session name<input
              bind:value={sessionName}
              required
              maxlength="200"
              placeholder="e.g. WGCV terminology review"
            /></label
          >
          <div class="modal-footer">
            <button type="button" onclick={closeModal}>Cancel</button><button
              class="primary"
              type="submit"
              disabled={!sessionName.trim() || !!busy}
              >{modal === 'rename' ? 'Save name' : 'Create session'}</button
            >
          </div>
        </form>
      {:else if modal === 'revert'}<h1 id="modal-title">Revert this term?</h1>
        <p class="muted">
          {document?.isNew
            ? 'This removes the new term from this local session.'
            : 'This restores the last submitted version, or the session’s starting version if it has not been submitted.'}
        </p>
        <div class="modal-footer">
          <button onclick={closeModal}>Keep editing</button><button
            class="danger"
            onclick={revert}
            disabled={!!busy}>Revert term</button
          >
        </div>
      {:else}<p class="eyebrow">ACCOUNT & SETTINGS</p>
        <h1 id="modal-title">Settings</h1>
        <section class="repository-settings" aria-labelledby="repository-settings-title">
          <h2 id="repository-settings-title">Repository</h2>
          <p class="small muted">
            Choose the repository for new sessions and PRs. Existing sessions keep their original
            destination.
          </p>
          <form
            onsubmit={(e) => {
              e.preventDefault();
              void changeRepository();
            }}
          >
            <label
              >Repository URL<input
                bind:value={repositoryUrl}
                placeholder="https://github.com/ceos-org/eo-glossary"
                required
                spellcheck="false"
                autocomplete="off"
              /></label
            >
            <div class="repository-setting-footer">
              <span class="small muted">New sessions: {boot?.catalogue.repository}</span><button
                type="submit"
                disabled={!!busy ||
                  !repositoryUrl.trim() ||
                  repositoryUrl === `https://github.com/${boot?.catalogue.repository}`}
                >Use repository</button
              >
            </div>
          </form>
        </section>
        <h2 class="account-settings-title">GitHub account</h2>
        {#if account}<div class="signed-in">
            <Github size={26} />
            <div>
              <strong>{account.name ?? account.login}</strong>
              <p>@{account.login}</p>
            </div>
            <button
              onclick={() =>
                perform('Signing out', async () => {
                  await api('sign_out');
                  account = null;
                  notice = 'Signed out. Your local sessions are kept.';
                })}>Sign out</button
            >
          </div>{:else}<p class="muted">
            Sign in when you’re ready to submit. Browsing and editing work offline.
          </p>
          <label class="checkbox-label"
            ><input type="checkbox" bind:checked={remember} /> Remember me in the system credential store</label
          >{#if device}<div class="device-code">
              <p>Enter this code on GitHub</p>
              <strong>{device.userCode}</strong><button
                onclick={() => void openLink(device!.verificationUrl).catch(fail)}
                >Open GitHub<ArrowUpRight size={13} /></button
              ><small>Waiting for authorization…</small>
            </div>{:else}<button
              class="primary auth-button"
              onclick={beginAuth}
              disabled={!clientConfigured || !!busy}
              ><Github size={16} /> Authenticate with GitHub</button
            >{#if !clientConfigured}<p class="small muted">
                Device sign-in needs the editor’s registered public client ID. Configure it below or
                use a token.
              </p>{/if}{/if}
          <details class="token-settings">
            <summary>Use a personal access token</summary>
            <form
              onsubmit={(e) => {
                e.preventDefault();
                void tokenSignIn();
              }}
            >
              <label
                >GitHub token<input
                  type="password"
                  bind:value={pat}
                  autocomplete="off"
                  placeholder="Token with public repository access"
                  required
                /></label
              >
              <p class="small muted">
                Classic token: public_repo scope. Fine-grained token: repository Contents and Pull
                requests write access. External contributors may need a classic token for
                cross-owner forks.
              </p>
              <button type="submit" disabled={!pat || !!busy}>Sign in with token</button>
            </form>
          </details>{/if}
        <details class="token-settings">
          <summary>Configure GitHub authentication</summary>
          <p class="small muted">
            Register a GitHub App (recommended) with Device Flow enabled, or use an existing OAuth
            app. A GitHub App uses selected-repository access with Contents and Pull requests write
            permissions. Disable webhooks; no hosted service, private key, or client secret is
            needed for device sign-in.
          </p>
          <label class="auth-kind"
            >Application type<select bind:value={authKind}
              ><option value="github_app">GitHub App (recommended)</option><option value="oauth_app"
                >OAuth app</option
              ></select
            ></label
          >
          <label
            >Public client ID<input
              bind:value={clientId}
              placeholder="Ov23…"
              autocomplete="off"
            /></label
          ><button
            disabled={!clientId || !!busy}
            onclick={() =>
              perform('Saving authentication settings', async () => {
                await api('configure_auth', { clientId, kind: authKind });
                clientConfigured = true;
                notice = 'GitHub authentication configured.';
              })}>Save client ID</button
          ><button
            class="text-button"
            onclick={() => void openLink('https://github.com/settings/developers').catch(fail)}
            >GitHub developer settings<ArrowUpRight size={12} /></button
          >
        </details>
        <div class="settings-note">
          <p><strong>EO Glossary Editor</strong> · 0.1.0 · Local development build</p>
          <p>{boot?.catalogue.count} cached terms · Snapshot {boot?.catalogue.sha.slice(0, 7)}</p>
          <p>Your sessions are stored locally in SQLite. Exports contain no account credentials.</p>
        </div>{/if}
    </div>
  </div>{/if}
