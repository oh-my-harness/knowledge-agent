import { afterEach, describe, expect, it, vi } from "vitest";
import {
  askEventsUrl,
  askVault,
  applyConfirmation,
  createAskSession,
  deleteAskSession,
  getAskSessionMessages,
  getLocalSettings,
  getVaultIndex,
  listConfirmations,
  listAskSessions,
  renameAskSession,
  rejectConfirmation,
  runMaintenanceScan,
  saveLocalSettings
} from "./api";
import type { LocalSettings } from "./types";

afterEach(() => {
  vi.restoreAllMocks();
});

function mockFetch(body: unknown, ok = true) {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue({
      ok,
      status: ok ? 200 : 500,
      text: async () => JSON.stringify(body),
      json: async () => body
    })
  );
}

const settings: LocalSettings = {
  llm: {
    provider: "deepseek",
    deepseek_api_key: null,
    deepseek_model: "deepseek-v4-flash"
  },
  web_search: {
    enabled: false,
    provider: "manual"
  }
};

describe("api client", () => {
  it("loads vault index", async () => {
    mockFetch({ root: "vault", notes: [] });
    await expect(getVaultIndex()).resolves.toEqual({ root: "vault", notes: [] });
  });

  it("loads and saves local settings", async () => {
    mockFetch(settings);
    await expect(getLocalSettings()).resolves.toEqual(settings);

    mockFetch({ ...settings, web_search: { enabled: true, provider: "manual" } });
    await expect(saveLocalSettings({ ...settings, web_search: { enabled: true, provider: "manual" } })).resolves.toEqual({
      ...settings,
      web_search: { enabled: true, provider: "manual" }
    });
    expect(fetch).toHaveBeenCalledWith("/api/settings/local", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ ...settings, web_search: { enabled: true, provider: "manual" } })
    });
  });

  it("runs maintenance scan", async () => {
    mockFetch({
      items: [
        {
          priority: "P0",
          kind: "broken_wikilink",
          file: "a.md",
          evidence: "Missing",
          requires_confirmation: false
        }
      ]
    });
    const inbox = await runMaintenanceScan();
    expect(inbox.items[0].kind).toBe("broken_wikilink");
  });

  it("manages confirmation queue", async () => {
    const item = {
      id: "item-1",
      kind: "replace_note",
      path: "a.md",
      reason: null,
      original_content: "old",
      proposed_content: "new",
      created_at: "1"
    };
    mockFetch({ items: [item] });
    await expect(listConfirmations()).resolves.toEqual({ items: [item] });

    mockFetch(item);
    await expect(applyConfirmation("item/1")).resolves.toEqual(item);
    expect(fetch).toHaveBeenCalledWith("/api/confirmations/item%2F1/apply", { method: "POST" });

    mockFetch(item);
    await expect(rejectConfirmation("item/1")).resolves.toEqual(item);
    expect(fetch).toHaveBeenLastCalledWith("/api/confirmations/item%2F1/reject", { method: "POST" });
  });

  it("asks the vault through the ask endpoint", async () => {
    mockFetch({ answer: "收到", sources: [], requires_followup: false });

    const response = await askVault("什么是 Agent Harness？", "research");

    expect(response.answer).toBe("收到");
    expect(fetch).toHaveBeenCalledWith("/api/ask", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ message: "什么是 Agent Harness？", session_id: "research", mode: "vault" })
    });
  });

  it("builds ask event stream urls", () => {
    expect(askEventsUrl("research/session")).toBe("/api/ask/events?session_id=research%2Fsession");
  });

  it("manages ask sessions", async () => {
    mockFetch([{ id: "default", name: "默认会话", updated_at: null }]);
    await expect(listAskSessions()).resolves.toEqual([{ id: "default", name: "默认会话", updated_at: null }]);

    mockFetch({ id: "research", name: "research", updated_at: null });
    await expect(createAskSession("research")).resolves.toEqual({
      id: "research",
      name: "research",
      updated_at: null
    });

    mockFetch([{ role: "user", content: "hello" }]);
    await expect(getAskSessionMessages("research/session")).resolves.toEqual([{ role: "user", content: "hello" }]);
    expect(fetch).toHaveBeenLastCalledWith("/api/ask/sessions/research%2Fsession/messages", undefined);
  });

  it("renames and deletes ask sessions", async () => {
    mockFetch({ id: "renamed", name: "renamed", updated_at: null });
    await expect(renameAskSession("research/session", "renamed")).resolves.toEqual({
      id: "renamed",
      name: "renamed",
      updated_at: null
    });
    expect(fetch).toHaveBeenCalledWith("/api/ask/sessions/research%2Fsession", {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ name: "renamed" })
    });

    vi.mocked(fetch).mockResolvedValueOnce({
      ok: true,
      status: 204,
      text: async () => "",
      json: async () => ({})
    } as Response);
    await expect(deleteAskSession("research/session")).resolves.toBeUndefined();
    expect(fetch).toHaveBeenLastCalledWith("/api/ask/sessions/research%2Fsession", { method: "DELETE" });
  });

  it("throws useful error for failed requests", async () => {
    mockFetch("failed", false);
    await expect(getVaultIndex()).rejects.toThrow("GET /api/vault/index failed with 500");
  });
});
