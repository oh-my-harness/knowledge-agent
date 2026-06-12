import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";

afterEach(() => {
  vi.restoreAllMocks();
});

function ok(body: unknown) {
  return Promise.resolve({ ok: true, status: 200, json: async () => body });
}

function notFound() {
  return Promise.resolve({ ok: false, status: 404, json: async () => ({}) });
}

function mockFetch(answer = "我已经收到你的问题。") {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "/api/health") {
        return ok({ status: "ok" });
      }
      if (url === "/api/vault/index") {
        return ok({
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
        });
      }
      if (url === "/api/maintenance/scan") {
        return ok({
          items: [
            {
              priority: "P0",
              kind: "broken_wikilink",
              file: "docs/concepts/agent-harness.md",
              evidence: "Missing target [[LLM Harness]]",
              requires_confirmation: false
            }
          ]
        });
      }
      if (url === "/api/ask/sessions") {
        return ok([{ id: "default", name: "默认会话", updated_at: null }]);
      }
      if (url === "/api/ask/sessions/default/messages") {
        return ok([]);
      }
      if (url === "/api/ask") {
        return ok({
          answer,
          sources: [],
          requires_followup: false
        });
      }
      return notFound();
    })
  );
}

function mockAskFailure() {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "/api/health") {
        return ok({ status: "ok" });
      }
      if (url === "/api/ask/sessions") {
        return ok([{ id: "default", name: "默认会话", updated_at: null }]);
      }
      if (url === "/api/ask/sessions/default/messages") {
        return ok([]);
      }
      if (url === "/api/ask") {
        return Promise.resolve({ ok: false, status: 500, json: async () => ({}) });
      }
      return notFound();
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

  it("asks a question with Enter and shows the assistant reply", async () => {
    mockFetch();
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "提问" }));
    await screen.findAllByText("默认会话");
    await userEvent.type(screen.getByLabelText("问题"), "什么是 Agent Harness？{enter}");

    expect(await screen.findByText("什么是 Agent Harness？")).toBeInTheDocument();
    expect(await screen.findByText("我已经收到你的问题。")).toBeInTheDocument();
    expect(fetch).toHaveBeenCalledWith("/api/ask", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ message: "什么是 Agent Harness？", session_id: "default", mode: "vault" })
    });
  });

  it("renders assistant markdown", async () => {
    mockFetch("## 结论\n\n- 第一条\n- 第二条\n\n```ts\nconst ok = true;\n```");
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "提问" }));
    await userEvent.type(screen.getByLabelText("问题"), "给我 Markdown{enter}");

    expect(await screen.findByRole("heading", { name: "结论", level: 2 })).toBeInTheDocument();
    const list = screen.getByRole("list");
    expect(within(list).getByText("第一条")).toBeInTheDocument();
    expect(screen.getByText("const ok = true;")).toBeInTheDocument();
  });

  it("keeps a newline for Shift+Enter", async () => {
    mockFetch();
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "提问" }));
    const input = await screen.findByLabelText("问题");
    await userEvent.type(input, "第一行{shift>}{enter}{/shift}第二行");

    expect(input).toHaveValue("第一行\n第二行");
    expect(screen.queryByText("我已经收到你的问题。")).not.toBeInTheDocument();
  });

  it("shows an error when asking fails", async () => {
    mockAskFailure();
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "提问" }));
    await screen.findAllByText("默认会话");
    await userEvent.type(screen.getByLabelText("问题"), "测试失败");
    await userEvent.click(screen.getByRole("button", { name: "发送" }));

    expect(await screen.findByText("POST /api/ask failed with 500")).toBeInTheDocument();
    expect(screen.getByText("测试失败")).toBeInTheDocument();
  });
});
