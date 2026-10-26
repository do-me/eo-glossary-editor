export interface Account {
  id: number;
  login: string;
  name: string | null;
}
export interface Section {
  id: number;
  heading: string;
  level: number;
  markdown: string;
  editable: boolean;
  richSafe: boolean;
}
export interface Document {
  title: string;
  description: string;
  tags: string[];
  sections: Section[];
  warnings: string[];
  source: string;
}
export interface OpenTerm {
  path: string;
  document: Document;
  revision: number;
  isNew: boolean;
}
export type Edit =
  | { kind: 'section'; id: number; markdown: string }
  | { kind: 'metadata'; description: string; tags: string[] }
  | { kind: 'add_definition' };
export interface Submission {
  number: number;
  url: string;
  headRepo: string;
  headBranch: string;
  headSha: string;
  accountId: number;
  state: string;
}
export interface SessionSummary {
  repository: string;
  id: string;
  name: string;
  updatedAt: string;
  changed: number;
  pending: number;
  submission: Submission | null;
  archived: boolean;
  interrupted: boolean;
}
export type AppView = 'terms' | 'sessions' | 'review' | 'updates';
export interface TermUi {
  details: boolean;
  source: boolean;
  sourceSections: number[];
}
export interface Workspace {
  view?: AppView;
  filtersOpen?: boolean;
  viewScroll?: Record<string, number>;
  sidebarScroll?: number;
  tabsScroll?: number;
  termUi?: Record<string, TermUi>;
  reviewMode?: ReviewMode;
  allChanges?: boolean;
  contributionTitle?: string;
  contributionBody?: string;
  dialog?: string | null;
  newTermTitle?: string;
  newTermTeams?: string[];
  sessionName?: string;

  tabs: string[];
  pinned: string[];
  active: string | null;
  query: string;
  teams: string[];
  classes: string[];
  statuses: string[];
  changedOnly: boolean;
  scroll: Record<string, number>;
}
export interface Draft {
  starting: string;
  source: string;
  submitted: string | null;
  revision: number;
  isNew: boolean;
}
export interface Session {
  repository: string;
  id: string;
  name: string;
  baseSha: string;
  terms: Record<string, Draft>;
  workspace: Workspace;
  submission: Submission | null;
  operation: unknown | null;
  conflicts: Record<string, unknown>;
}
export interface Term {
  path: string;
  title: string;
  description: string;
  tags: string[];
  searchText: string;
  changed: boolean;
  pending: boolean;
  isNew: boolean;
}
export interface ReviewSection {
  key: string;
  heading: string;
  level: number;
  html: string;
}
export interface ReviewPage {
  title: string;
  description: string;
  tags: string[];
  sections: ReviewSection[];
}
export type ReviewMode = 'changes' | 'preview' | 'compare';
export interface Change {
  beforePage: ReviewPage | null;
  afterPage: ReviewPage;
  path: string;
  title: string;
  before: string;
  after: string;
  diff: string;
  warnings: string[];
  isNew: boolean;
}
export interface Bootstrap {
  preferences: { zoom: number };
  defaultRepository: string;
  sessions: SessionSummary[];
  activeSession: string | null;
  catalogue: { repository: string; sha: string; fetchedAt: string; count: number; branch: string };
  teams: string[];
  classes: string[];
  statuses: string[];
}
export interface PublicPr {
  number: number;
  title: string;
  html_url: string;
}

export interface SyncConflict {
  path: string;
  title: string;
  revision: number;
  githubSha: string;
  deleted: boolean;
  localDiff: string;
  githubDiff: string;
  comparison: Change;
}
export interface SyncReport {
  repository: string;
  sha: string;
  branch: string;
  checkedAt: string;
  updated: number;
  merged: number;
  deferred: boolean;
  conflicts: SyncConflict[];
}
