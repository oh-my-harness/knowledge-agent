import { afterEach, describe, expect, it, vi } from "vitest";
import { askVault, getHealth, getVaultIndex, runMaintenanceScan } from "./api";

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

describe("api client", () => {
  it("loads health", async () => {
    mockFetch({ status: "ok" });
    await expect(getHealth()).resolves.toEqual({ status: "ok" });
  });

  it("loads vault index", async () => {
    mockFetch({ root: "vault", notes: [] });
    await expect(getVaultIndex()).resolves.toEqual({ root: "vault", notes: [] });
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

  it("asks the vault through the ask endpoint", async () => {
    mockFetch({ answer: "收到", sources: [], requires_followup: false });

    const response = await askVault("什么是 Agent Harness？");

    expect(response.answer).toBe("收到");
    expect(fetch).toHaveBeenCalledWith("/api/ask", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ message: "什么是 Agent Harness？", mode: "vault" })
    });
  });

  it("throws useful error for failed requests", async () => {
    mockFetch("failed", false);
    await expect(getHealth()).rejects.toThrow("GET /api/health failed with 500");
  });
});
