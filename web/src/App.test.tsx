import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";

afterEach(() => {
  vi.restoreAllMocks();
});

function mockFetch() {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "/api/health") {
        return Promise.resolve({ ok: true, status: 200, json: async () => ({ status: "ok" }) });
      }
      if (url === "/api/vault/index") {
        return Promise.resolve({
          ok: true,
          status: 200,
          json: async () => ({
            root: "fixture",
            notes: [
              {
                relative_path: "docs/concepts/agent-harness.md",
                title: "Agent Harness",
                note_type: "concept",
                tags: ["agent", "runtime"],
                links: [{ target: "LLM Harness", alias: null }]
              }
            ]
          })
        });
      }
      if (url === "/api/maintenance/scan") {
        return Promise.resolve({
          ok: true,
          status: 200,
          json: async () => ({
            items: [
              {
                priority: "P0",
                kind: "broken_wikilink",
                file: "docs/concepts/agent-harness.md",
                evidence: "Missing target [[LLM Harness]]",
                requires_confirmation: false
              }
            ]
          })
        });
      }
      if (url === "/api/ask") {
        return Promise.resolve({
          ok: true,
          status: 200,
          json: async () => ({
            answer: "已收到你的问题。当前提问通路已经连通，后续会接入知识库检索和 LLM Harness。",
            sources: [],
            requires_followup: false
          })
        });
      }
      return Promise.resolve({ ok: false, status: 404, json: async () => ({}) });
    })
  );
}

function mockAskFailure() {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "/api/health") {
        return Promise.resolve({ ok: true, status: 200, json: async () => ({ status: "ok" }) });
      }
      if (url === "/api/ask") {
        return Promise.resolve({ ok: false, status: 500, json: async () => ({}) });
      }
      return Promise.resolve({ ok: false, status: 404, json: async () => ({}) });
    })
  );
}

describe("App", () => {
  it("loads service status", async () => {
    mockFetch();
    render(<App />);

    await waitFor(() => expect(screen.getByText("服务在线")).toBeInTheDocument());
  });

  it("shows vault notes", async () => {
    mockFetch();
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "知识库" }));

    expect(await screen.findByText("Agent Harness")).toBeInTheDocument();
    expect(screen.getByText("docs/concepts/agent-harness.md")).toBeInTheDocument();
  });

  it("runs maintenance scan", async () => {
    mockFetch();
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "维护扫描" }));
    await userEvent.click(screen.getByRole("button", { name: "开始扫描" }));

    expect(await screen.findByText("broken_wikilink")).toBeInTheDocument();
    expect(screen.getByText("Missing target [[LLM Harness]]")).toBeInTheDocument();
  });

  it("asks a question and shows the assistant reply", async () => {
    mockFetch();
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "提问" }));
    await userEvent.type(screen.getByLabelText("问题"), "什么是 Agent Harness？");
    await userEvent.click(screen.getByRole("button", { name: "发送" }));

    expect(await screen.findByText("什么是 Agent Harness？")).toBeInTheDocument();
    expect(
      await screen.findByText("已收到你的问题。当前提问通路已经连通，后续会接入知识库检索和 LLM Harness。")
    ).toBeInTheDocument();
  });

  it("shows an error when asking fails", async () => {
    mockAskFailure();
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "提问" }));
    await userEvent.type(screen.getByLabelText("问题"), "测试失败");
    await userEvent.click(screen.getByRole("button", { name: "发送" }));

    expect(await screen.findByText("POST /api/ask failed with 500")).toBeInTheDocument();
    expect(screen.getByText("测试失败")).toBeInTheDocument();
  });
});
