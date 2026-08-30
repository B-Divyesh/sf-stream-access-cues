export type Settings = { obs_host: string; obs_port: number; configured: boolean; password_saved: boolean };
export type ChecklistItem = { id: string; text: string; done: boolean };
export type Cue = { id: string; label: string; scene_name: string };
export type PlatformLink = { id: string; label: string; url: string };
export type ObsStatus = { connected: boolean; message: string; scenes: string[]; current_scene: string | null };
export type Runtime = { build_sha: string; deployment_mode: 'local' | 'hosted'; obs_control_available: boolean };

export type WorkspaceApi = {
  settings: () => Promise<Settings>;
  saveSettings: (value: { obs_host: string; obs_port: number; obs_password?: string }) => Promise<Settings>;
  checklist: () => Promise<ChecklistItem[]>;
  saveChecklist: (items: ChecklistItem[]) => Promise<ChecklistItem[]>;
  cues: () => Promise<Cue[]>;
  saveCues: (items: Cue[]) => Promise<Cue[]>;
  links: () => Promise<PlatformLink[]>;
  saveLinks: (items: PlatformLink[]) => Promise<PlatformLink[]>;
};

export class ApiFailure extends Error {
  constructor(message: string, public status: number) { super(message); }
}

const workspaceKeyName = 'stream-access-cues.operator-key';

export function operatorKey(): string {
  const existing = localStorage.getItem(workspaceKeyName);
  if (existing && /^[A-Za-z0-9_-]{43}$/.test(existing)) return existing;
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  const key = btoa(String.fromCharCode(...bytes))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '');
  localStorage.setItem(workspaceKeyName, key);
  return key;
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`/api${path}`, {
      ...options,
      headers: { 'Content-Type': 'application/json', 'X-Operator-Key': operatorKey(), ...options?.headers }
    });
  } catch {
    throw new ApiFailure('The private cue service is unavailable. Start Stream Access Cues, then retry.', 0);
  }
  if (!response.ok) {
    const body = await response.json().catch(() => ({})) as { error?: string };
    throw new ApiFailure(body.error || `The request failed (${response.status}).`, response.status);
  }
  return response.json() as Promise<T>;
}

async function publicRequest<T>(path: string): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`/api${path}`);
  } catch {
    throw new ApiFailure('The cue service is unavailable. Start Stream Access Cues, then retry.', 0);
  }
  if (!response.ok) {
    throw new ApiFailure(`The request failed (${response.status}).`, response.status);
  }
  return response.json() as Promise<T>;
}

export const api: WorkspaceApi & {
  runtime: () => Promise<Runtime>;
  obsStatus: () => Promise<ObsStatus>;
  testObs: () => Promise<ObsStatus>;
  setScene: (scene_name: string) => Promise<ObsStatus>;
} = {
  runtime: () => publicRequest<Runtime>('/runtime'),
  settings: () => request<Settings>('/settings'),
  saveSettings: (value: { obs_host: string; obs_port: number; obs_password?: string }) => request<Settings>('/settings', { method: 'PUT', body: JSON.stringify(value) }),
  checklist: () => request<ChecklistItem[]>('/checklist'),
  saveChecklist: (items: ChecklistItem[]) => request<ChecklistItem[]>('/checklist', { method: 'PUT', body: JSON.stringify(items) }),
  cues: () => request<Cue[]>('/cues'),
  saveCues: (items: Cue[]) => request<Cue[]>('/cues', { method: 'PUT', body: JSON.stringify(items) }),
  links: () => request<PlatformLink[]>('/links'),
  saveLinks: (items: PlatformLink[]) => request<PlatformLink[]>('/links', { method: 'PUT', body: JSON.stringify(items) }),
  obsStatus: () => request<ObsStatus>('/obs/status'),
  testObs: () => request<ObsStatus>('/obs/test', { method: 'POST' }),
  setScene: (scene_name: string) => request<ObsStatus>('/obs/scene', { method: 'POST', body: JSON.stringify({ scene_name }) })
};

type BrowserWorkspace = {
  version: 1;
  checklist: ChecklistItem[];
  cues: Cue[];
  links: PlatformLink[];
};

export type BrowserWorkspaceKind = 'hosted' | 'demo';

const browserWorkspaceKeys: Record<BrowserWorkspaceKind, string> = {
  hosted: 'stream-access-cues.hosted.workspace.v1',
  demo: 'demo:stream-access-cues.workspace.v1'
};

const hostedStarter: BrowserWorkspace = {
  version: 1,
  checklist: [
    { id: 'starter-1', text: 'Set stream title and category', done: false },
    { id: 'starter-2', text: 'Check microphone level', done: false },
    { id: 'starter-3', text: 'Confirm recording path', done: false },
    { id: 'starter-4', text: 'Test scene cues', done: false },
    { id: 'starter-5', text: 'Start broadcast', done: false }
  ],
  cues: [],
  links: [
    { id: 'twitch', label: 'Open Twitch dashboard', url: 'https://dashboard.twitch.tv/' },
    { id: 'youtube', label: 'Open YouTube Studio', url: 'https://studio.youtube.com/' }
  ]
};

const demoStarter: BrowserWorkspace = {
  version: 1,
  checklist: [
    { id: 'demo-title', text: 'Set the Friday community stream title', done: true },
    { id: 'demo-audio', text: 'Confirm the USB microphone level', done: true },
    { id: 'demo-recording', text: 'Check the recording folder has space', done: false },
    { id: 'demo-cues', text: 'Preview the starting-soon scene cue', done: false },
    { id: 'demo-go-live', text: 'Start the broadcast when the checklist is complete', done: false }
  ],
  cues: [
    { id: 'demo-starting', label: 'Starting soon', scene_name: 'Starting Soon' },
    { id: 'demo-live', label: 'Go live', scene_name: 'Camera + Game' },
    { id: 'demo-break', label: 'Take a break', scene_name: 'Be Right Back' }
  ],
  links: [
    { id: 'demo-twitch', label: 'Open Twitch dashboard', url: 'https://dashboard.twitch.tv/' },
    { id: 'demo-youtube', label: 'Open YouTube Studio', url: 'https://studio.youtube.com/' }
  ]
};

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function seedFor(kind: BrowserWorkspaceKind): BrowserWorkspace {
  return clone(kind === 'demo' ? demoStarter : hostedStarter);
}

function loadBrowserWorkspace(kind: BrowserWorkspaceKind): BrowserWorkspace {
  const key = browserWorkspaceKeys[kind];
  try {
    const parsed = JSON.parse(localStorage.getItem(key) || 'null') as Partial<BrowserWorkspace> | null;
    if (parsed?.version === 1 && Array.isArray(parsed.checklist) && Array.isArray(parsed.cues) && Array.isArray(parsed.links)) {
      return parsed as BrowserWorkspace;
    }
  } catch {
    // A malformed browser value is safely replaced by the bundled seed.
  }
  const seeded = seedFor(kind);
  localStorage.setItem(key, JSON.stringify(seeded));
  return seeded;
}

function saveBrowserWorkspace(kind: BrowserWorkspaceKind, value: BrowserWorkspace): void {
  localStorage.setItem(browserWorkspaceKeys[kind], JSON.stringify(value));
}

function browserSettings(): Settings {
  return { obs_host: '127.0.0.1', obs_port: 4455, configured: false, password_saved: false };
}

function validateBrowserId(id: string): void {
  if (!/^[A-Za-z0-9_-]{1,80}$/.test(id)) throw new ApiFailure('An item identifier was invalid.', 400);
}

function validateBrowserText(name: string, value: string, maximum: number): void {
  const length = value.trim().length;
  if (length === 0 || length > maximum) throw new ApiFailure(`${name} must be between 1 and ${maximum} characters.`, 400);
}

function validateBrowserIds(ids: string[], message: string): void {
  if (new Set(ids).size !== ids.length) throw new ApiFailure(message, 400);
  ids.forEach(validateBrowserId);
}

function validateBrowserChecklist(items: ChecklistItem[]): void {
  if (items.length > 50) throw new ApiFailure('A checklist can contain at most 50 items.', 400);
  validateBrowserIds(items.map((item) => item.id), 'Each checklist item needs a unique identifier.');
  items.forEach((item) => validateBrowserText('Checklist item', item.text, 200));
}

function validateBrowserCues(cues: Cue[]): void {
  if (cues.length > 9) throw new ApiFailure('You can assign at most nine keyboard cues.', 400);
  validateBrowserIds(cues.map((cue) => cue.id), 'Each cue needs a unique identifier.');
  cues.forEach((cue) => {
    validateBrowserText('Cue label', cue.label, 60);
    validateBrowserText('Scene name', cue.scene_name, 128);
  });
}

function validateBrowserLinks(links: PlatformLink[]): void {
  if (links.length > 8) throw new ApiFailure('You can save at most eight metadata links.', 400);
  validateBrowserIds(links.map((link) => link.id), 'Each metadata link needs a unique identifier.');
  links.forEach((link) => {
    validateBrowserText('Link label', link.label, 80);
    let url: URL;
    try { url = new URL(link.url); } catch { throw new ApiFailure('Each metadata link needs a complete web address.', 400); }
    if (!['http:', 'https:'].includes(url.protocol)) throw new ApiFailure('Metadata links must use http or https.', 400);
  });
}

/**
 * The public guide and demo never put workspace data in a container. This
 * keeps the guide private and deterministic even when the factory scales it.
 */
export function browserWorkspace(kind: BrowserWorkspaceKind): WorkspaceApi {
  return {
    settings: async () => browserSettings(),
    saveSettings: async () => {
      throw new ApiFailure('OBS connection settings are available only in the local service.', 403);
    },
    checklist: async () => clone(loadBrowserWorkspace(kind).checklist),
    saveChecklist: async (checklist) => {
      validateBrowserChecklist(checklist);
      const workspace = loadBrowserWorkspace(kind);
      workspace.checklist = clone(checklist);
      saveBrowserWorkspace(kind, workspace);
      return clone(workspace.checklist);
    },
    cues: async () => clone(loadBrowserWorkspace(kind).cues),
    saveCues: async (cues) => {
      validateBrowserCues(cues);
      const workspace = loadBrowserWorkspace(kind);
      workspace.cues = clone(cues);
      saveBrowserWorkspace(kind, workspace);
      return clone(workspace.cues);
    },
    links: async () => clone(loadBrowserWorkspace(kind).links),
    saveLinks: async (links) => {
      validateBrowserLinks(links);
      const workspace = loadBrowserWorkspace(kind);
      workspace.links = clone(links);
      saveBrowserWorkspace(kind, workspace);
      return clone(workspace.links);
    }
  };
}

export function resetBrowserWorkspace(kind: BrowserWorkspaceKind): void {
  localStorage.removeItem(browserWorkspaceKeys[kind]);
}
