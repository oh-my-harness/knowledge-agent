import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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

function localSettings() {
  return {
    llm: {
      provider: "deepseek",
      deepseek_api_key: null,
      deepseek_model: "deepseek-v4-flash"
    },
    web_search: {
      enabled: false,
      provider: "duckduckgo"
    }
  };
}

function mockFetch(answer = "我已经收到你的问题。") {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
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
      if (url === "/api/confirmations") {
        return ok({ items: [] });
      }
      if (url === "/api/settings/local") {
        return ok(localSettings());
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

function mockDelayedAsk(answer = "我已经想好了。") {
  let resolveAsk: () => void = () => {};
  const askReady = new Promise<void>((resolve) => {
    resolveAsk = resolve;
  });

  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "/api/ask/sessions") {
        return ok([{ id: "default", name: "默认会话", updated_at: null }]);
      }
      if (url === "/api/ask/sessions/default/messages") {
        return ok([]);
      }
      if (url === "/api/ask") {
        return askReady.then(() =>
          ok({
            answer,
            sources: [],
            requires_followup: false
          })
        );
      }
      return notFound();
    })
  );

  return { resolveAsk };
}

function mockSessionScrollFetch() {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url === "/api/ask/sessions") {
        return ok([
          { id: "default", name: "默认会话", updated_at: null },
          { id: "research", name: "研究会话", updated_at: null }
        ]);
      }
      if (url === "/api/ask/sessions/default/messages") {
        return ok([
          { role: "user", content: "默认问题" },
          { role: "assistant", content: "默认回答" }
        ]);
      }
      if (url === "/api/ask/sessions/research/messages") {
        return ok([
          { role: "user", content: "研究问题" },
          { role: "assistant", content: "研究回答" }
        ]);
      }
      return notFound();
    })
  );
}

function mockSessionActionFetch() {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url === "/api/ask/sessions") {
        return ok([
          { id: "default", name: "默认会话", updated_at: null },
          { id: "research", name: "研究会话", updated_at: null }
        ]);
      }
      if (url === "/api/ask/sessions/default/messages") {
        return ok([]);
      }
      if (url === "/api/ask/sessions/research/messages") {
        return ok([]);
      }
      if (url === "/api/ask/sessions/research" && init?.method === "PATCH") {
        return ok({ id: "renamed", name: "renamed", updated_at: null });
      }
      if (url === "/api/ask/sessions/renamed/messages") {
        return ok([]);
      }
      if (url === "/api/ask/sessions/renamed" && init?.method === "DELETE") {
        return Promise.resolve({ ok: true, status: 204, json: async () => ({}) });
      }
      return notFound();
    })
  );
}

function mockConfirmationFetch() {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url === "/api/ask/sessions") {
        return ok([{ id: "default", name: "默认会话", updated_at: null }]);
      }
      if (url === "/api/ask/sessions/default/messages") {
        return ok([]);
      }
      if (url === "/api/confirmations") {
        return ok({
          items: [
            {
              id: "item-1",
              kind: "replace_note",
              path: "docs/note.md",
              reason: "补充说明",
              original_content: "# Old",
              proposed_content: "# New",
              created_at: "1"
            }
          ]
        });
      }
      if (url === "/api/confirmations/item-1/apply" && init?.method === "POST") {
        return ok({
          id: "item-1",
          kind: "replace_note",
          path: "docs/note.md",
          reason: "补充说明",
          original_content: "# Old",
          proposed_content: "# New",
          created_at: "1"
        });
      }
      return notFound();
    })
  );
}

describe("App", () => {
  it("starts on the ask page without service status navigation", async () => {
    mockFetch();
    render(<App />);

    expect(screen.queryByRole("button", { name: "服务状态" })).not.toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "提问" })).toBeInTheDocument();
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

  it("shows and applies confirmation queue items", async () => {
    mockConfirmationFetch();
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "维护扫描" }));

    expect(await screen.findByText("docs/note.md")).toBeInTheDocument();
    expect(screen.getByLabelText("修改预览 docs/note.md")).toBeInTheDocument();
    expect(screen.getByText("# Old")).toBeInTheDocument();
    expect(screen.getByText("# New")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "确认应用" }));

    expect(fetch).toHaveBeenCalledWith("/api/confirmations/item-1/apply", { method: "POST" });
  });

  it("loads and saves local settings", async () => {
    mockFetch();
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "设置" }));
    expect(await screen.findByText("当前配置")).toBeInTheDocument();
    expect(screen.getAllByText("deepseek-v4-flash").length).toBeGreaterThan(0);
    expect(screen.getByText("未配置")).toBeInTheDocument();
    expect(screen.getByText("未启用")).toBeInTheDocument();

    await userEvent.clear(await screen.findByLabelText("模型名"));
    await userEvent.type(screen.getByLabelText("模型名"), "deepseek-chat");
    await userEvent.click(screen.getByRole("checkbox", { name: "启用网页搜索工具" }));
    await userEvent.click(screen.getByRole("button", { name: "保存设置" }));

    expect(fetch).toHaveBeenCalledWith("/api/settings/local", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        llm: {
          provider: "deepseek",
          deepseek_api_key: null,
          deepseek_model: "deepseek-chat"
        },
        web_search: {
          enabled: true,
          provider: "duckduckgo"
        }
      })
    });
    expect(await screen.findByText("设置已保存。LLM 和网页搜索配置会在服务重启后用于新 runner。")).toBeInTheDocument();
  });

  it("asks a question with Enter and shows the assistant reply", async () => {
    mockFetch();
    render(<App />);

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

  it("shows a thinking state while the assistant is working", async () => {
    const { resolveAsk } = mockDelayedAsk();
    render(<App />);

    await screen.findAllByText("默认会话");
    await userEvent.type(screen.getByLabelText("问题"), "继续研究{enter}");

    expect(await screen.findByRole("status", { name: "助手正在思考" })).toBeInTheDocument();

    resolveAsk();

    expect(await screen.findByText("我已经想好了。")).toBeInTheDocument();
    expect(screen.queryByRole("status", { name: "助手正在思考" })).not.toBeInTheDocument();
  });

  it("renames and deletes sessions from the session list", async () => {
    mockSessionActionFetch();
    vi.spyOn(window, "prompt").mockReturnValue("renamed");
    vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<App />);

    await screen.findByRole("button", { name: "研究会话" });
    await userEvent.click(screen.getByRole("button", { name: "重命名会话 研究会话" }));

    expect(await screen.findByRole("button", { name: "renamed" })).toBeInTheDocument();
    expect(fetch).toHaveBeenCalledWith("/api/ask/sessions/research", {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ name: "renamed" })
    });

    await userEvent.click(screen.getByRole("button", { name: "删除会话 renamed" }));

    expect(screen.queryByRole("button", { name: "renamed" })).not.toBeInTheDocument();
    expect(fetch).toHaveBeenCalledWith("/api/ask/sessions/renamed", { method: "DELETE" });
  });

  it("scrolls sessions to the latest message when opened", async () => {
    const scrollHeight = vi.spyOn(HTMLElement.prototype, "scrollHeight", "get").mockReturnValue(1000);
    const clientHeight = vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(400);
    const scrollIntoView = vi.fn();
    const originalScrollIntoView = HTMLElement.prototype.scrollIntoView;
    HTMLElement.prototype.scrollIntoView = scrollIntoView;
    mockSessionScrollFetch();
    render(<App />);

    await screen.findByText("默认回答");
    const messageList = screen.getByLabelText("消息");
    await waitFor(() => expect(messageList.scrollTop).toBe(1000));

    messageList.scrollTop = 120;
    fireEvent.scroll(messageList);

    await userEvent.click(screen.getByRole("button", { name: "研究会话" }));
    await screen.findByText("研究回答");
    await waitFor(() => expect(messageList.scrollTop).toBe(1000));

    messageList.scrollTop = 240;
    fireEvent.scroll(messageList);

    await userEvent.click(screen.getByRole("button", { name: "默认会话" }));
    await screen.findByText("默认回答");
    await waitFor(() => expect(messageList.scrollTop).toBe(1000));
    expect(scrollIntoView).toHaveBeenCalled();

    clientHeight.mockRestore();
    scrollHeight.mockRestore();
    HTMLElement.prototype.scrollIntoView = originalScrollIntoView;
  });

  it("renders assistant markdown", async () => {
    mockFetch("## 结论\n\n- 第一条\n- 第二条\n\n```ts\nconst ok = true;\n```");
    render(<App />);

    await userEvent.type(screen.getByLabelText("问题"), "给我 Markdown{enter}");

    expect(await screen.findByRole("heading", { name: "结论", level: 2 })).toBeInTheDocument();
    const list = screen.getByRole("list");
    expect(within(list).getByText("第一条")).toBeInTheDocument();
    expect(screen.getByText("const ok = true;")).toBeInTheDocument();
  });

  it("keeps a newline for Shift+Enter", async () => {
    mockFetch();
    render(<App />);

    const input = await screen.findByLabelText("问题");
    await userEvent.type(input, "第一行{shift>}{enter}{/shift}第二行");

    expect(input).toHaveValue("第一行\n第二行");
    expect(screen.queryByText("我已经收到你的问题。")).not.toBeInTheDocument();
  });

  it("shows an error when asking fails", async () => {
    mockAskFailure();
    render(<App />);

    await screen.findAllByText("默认会话");
    await userEvent.type(screen.getByLabelText("问题"), "测试失败");
    await userEvent.click(screen.getByRole("button", { name: "发送" }));

    expect(await screen.findByText("POST /api/ask failed with 500")).toBeInTheDocument();
    expect(screen.getByText("测试失败")).toBeInTheDocument();
  });
});
