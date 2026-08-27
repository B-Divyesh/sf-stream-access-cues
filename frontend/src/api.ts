export type Settings = { obs_host: string; obs_port: number; configured: boolean; password_saved: boolean };
export type ChecklistItem = { id: string; text: string; done: boolean };
export type Cue = { id: string; label: string; scene_name: string };
export type PlatformLink = { id: string; label: string; url: string };
export type ObsStatus = { connected: boolean; message: string; scenes: string[]; current_scene: string | null };

export class ApiFailure extends Error {
  constructor(message: string, public status: number) { super(message); }
}

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`/api${path}`, {
      ...options,
      headers: { 'Content-Type': 'application/json', ...options?.headers }
    });
  } catch {
    throw new ApiFailure('The local service is unavailable. Start the Stream Access Cues server, then retry.', 0);
  }
  if (!response.ok) {
    const body = await response.json().catch(() => ({})) as { error?: string };
    throw new ApiFailure(body.error || `The request failed (${response.status}).`, response.status);
  }
  return response.json() as Promise<T>;
}

export const api = {
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
