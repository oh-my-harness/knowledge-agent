import type {
  AskResponse,
  ChatMessage,
  ChatSession,
  ConfirmationItem,
  ConfirmationQueue,
  LocalSettings,
  MaintenanceInbox,
  PdfAsset,
  VaultScan
} from "./types";

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, init);
  if (!response.ok) {
    throw new Error(`${init?.method ?? "GET"} ${path} failed with ${response.status}`);
  }
  return response.json() as Promise<T>;
}

export function getVaultIndex(): Promise<VaultScan> {
  return requestJson<VaultScan>("/api/vault/index");
}

export function listPdfAssets(): Promise<PdfAsset[]> {
  return requestJson<PdfAsset[]>("/api/vault/pdfs");
}

export function getLocalSettings(): Promise<LocalSettings> {
  return requestJson<LocalSettings>("/api/settings/local");
}

export function saveLocalSettings(settings: LocalSettings): Promise<LocalSettings> {
  const { effective: _effective, ...payload } = settings;
  return requestJson<LocalSettings>("/api/settings/local", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload)
  });
}

export function runMaintenanceScan(): Promise<MaintenanceInbox> {
  return requestJson<MaintenanceInbox>("/api/maintenance/scan", { method: "POST" });
}

export function listConfirmations(): Promise<ConfirmationQueue> {
  return requestJson<ConfirmationQueue>("/api/confirmations");
}

export function applyConfirmation(id: string): Promise<ConfirmationItem> {
  return requestJson<ConfirmationItem>(`/api/confirmations/${encodeURIComponent(id)}/apply`, { method: "POST" });
}

export function rejectConfirmation(id: string): Promise<ConfirmationItem> {
  return requestJson<ConfirmationItem>(`/api/confirmations/${encodeURIComponent(id)}/reject`, { method: "POST" });
}

export function listAskSessions(): Promise<ChatSession[]> {
  return requestJson<ChatSession[]>("/api/ask/sessions");
}

export function createAskSession(name: string): Promise<ChatSession> {
  return requestJson<ChatSession>("/api/ask/sessions", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ name })
  });
}

export function renameAskSession(sessionId: string, name: string): Promise<ChatSession> {
  return requestJson<ChatSession>(`/api/ask/sessions/${encodeURIComponent(sessionId)}`, {
    method: "PATCH",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ name })
  });
}

export async function deleteAskSession(sessionId: string): Promise<void> {
  const response = await fetch(`/api/ask/sessions/${encodeURIComponent(sessionId)}`, { method: "DELETE" });
  if (!response.ok) {
    throw new Error(`DELETE /api/ask/sessions/${encodeURIComponent(sessionId)} failed with ${response.status}`);
  }
}

export function getAskSessionMessages(sessionId: string): Promise<ChatMessage[]> {
  return requestJson<ChatMessage[]>(`/api/ask/sessions/${encodeURIComponent(sessionId)}/messages`);
}

export function askVault(message: string, sessionId = "default"): Promise<AskResponse> {
  return requestJson<AskResponse>("/api/ask", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ message, session_id: sessionId, mode: "vault" })
  });
}

export function askEventsUrl(sessionId = "default"): string {
  return `/api/ask/events?session_id=${encodeURIComponent(sessionId)}`;
}
