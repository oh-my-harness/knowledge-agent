import type { AskResponse, HealthResponse, MaintenanceInbox, VaultScan } from "./types";

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

export function askVault(message: string): Promise<AskResponse> {
  return requestJson<AskResponse>("/api/ask", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ message, mode: "vault" })
  });
}
