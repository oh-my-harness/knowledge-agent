import type { AskResponse, ChatMessage, ChatSession, HealthResponse, MaintenanceInbox, VaultScan } from "./types";

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, init);
  if (!response.ok) {
    throw new Error(`${init?.method ?? "GET"} ${path} failed with ${response.status}`);
  }
  return response.json() as Promise<T>;
}

export function getHealth(): Promise<HealthResponse> {
  return requestJson<HealthResponse>("/api/health");
}

export function getVaultIndex(): Promise<VaultScan> {
  return requestJson<VaultScan>("/api/vault/index");
}

export function runMaintenanceScan(): Promise<MaintenanceInbox> {
  return requestJson<MaintenanceInbox>("/api/maintenance/scan", { method: "POST" });
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
