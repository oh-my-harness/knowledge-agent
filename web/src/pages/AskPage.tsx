import { FormEvent, KeyboardEvent, useEffect, useMemo, useState } from "react";
import { askVault, createAskSession, getAskSessionMessages, listAskSessions } from "../api";
import type { ChatMessage, ChatSession } from "../types";

export function AskPage() {
  const [input, setInput] = useState("");
  const [sessionName, setSessionName] = useState("");
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState("default");
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [isLoadingMessages, setIsLoadingMessages] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeSession = useMemo(
    () => sessions.find((session) => session.id === activeSessionId),
    [activeSessionId, sessions]
  );

  useEffect(() => {
    let isCurrent = true;
    listAskSessions()
      .then((loadedSessions) => {
        if (!isCurrent) {
          return;
        }
        const nextSessions =
          loadedSessions.length > 0
            ? loadedSessions
            : [{ id: "default", name: "默认会话", updated_at: null }];
        setSessions(nextSessions);
        setActiveSessionId((current) =>
          nextSessions.some((session) => session.id === current) ? current : nextSessions[0].id
        );
      })
      .catch((err) => {
        if (isCurrent) {
          setError(err instanceof Error ? err.message : "加载会话失败");
        }
      });

    return () => {
      isCurrent = false;
    };
  }, []);

  useEffect(() => {
    let isCurrent = true;
    setIsLoadingMessages(true);
    getAskSessionMessages(activeSessionId)
      .then((loadedMessages) => {
        if (isCurrent) {
          setMessages(loadedMessages);
          setError(null);
        }
      })
      .catch((err) => {
        if (isCurrent) {
          setMessages([]);
          setError(err instanceof Error ? err.message : "加载消息失败");
        }
      })
      .finally(() => {
        if (isCurrent) {
          setIsLoadingMessages(false);
        }
      });

    return () => {
      isCurrent = false;
    };
  }, [activeSessionId]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await sendMessage();
  }

  async function sendMessage() {
    const message = input.trim();
    if (!message || isSending) {
      return;
    }

    setMessages((current) => [...current, { role: "user", content: message }]);
    setInput("");
    setIsSending(true);
    setError(null);

    try {
      const response = await askVault(message, activeSessionId);
      setMessages((current) => [...current, { role: "assistant", content: response.answer }]);
      setSessions((current) =>
        current.map((session) =>
          session.id === activeSessionId ? { ...session, updated_at: new Date().toISOString() } : session
        )
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : "发送失败");
    } finally {
      setIsSending(false);
    }
  }

  async function handleCreateSession(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const name = sessionName.trim();
    if (!name) {
      return;
    }

    try {
      const session = await createAskSession(name);
      setSessions((current) => {
        const withoutDuplicate = current.filter((item) => item.id !== session.id);
        return [session, ...withoutDuplicate];
      });
      setActiveSessionId(session.id);
      setSessionName("");
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "创建会话失败");
    }
  }

  function handleInputKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) {
      return;
    }

    event.preventDefault();
    void sendMessage();
  }

  return (
    <section className="page ask-page">
      <header className="page-header">
        <h2>提问</h2>
        <span>{activeSession?.name ?? activeSessionId}</span>
      </header>
      <div className="chat-layout">
        <aside className="session-panel" aria-label="会话">
          <form className="session-form" onSubmit={handleCreateSession}>
            <label className="sr-only" htmlFor="session-name">
              新会话名称
            </label>
            <input
              id="session-name"
              value={sessionName}
              onChange={(event) => setSessionName(event.target.value)}
              placeholder="新会话名称"
            />
            <button className="primary-button" disabled={sessionName.trim().length === 0} type="submit">
              新建
            </button>
          </form>
          <div className="session-list">
            {sessions.map((session) => (
              <button
                className={`session-item ${session.id === activeSessionId ? "active" : ""}`}
                key={session.id}
                onClick={() => setActiveSessionId(session.id)}
                type="button"
              >
                <span>{session.name}</span>
                {session.updated_at && <small>{new Date(session.updated_at).toLocaleString()}</small>}
              </button>
            ))}
          </div>
        </aside>
        <div className="message-list" aria-live="polite">
          {isLoadingMessages ? (
            <p className="muted">加载消息中</p>
          ) : (
            messages.map((message, index) => (
              <article className={`message ${message.role}`} key={`${message.role}-${index}`}>
                <span>{message.role === "user" ? "你" : "助手"}</span>
                <p>{message.content}</p>
              </article>
            ))
          )}
        </div>
      </div>
      {error && <p className="error-text">{error}</p>}
      <form className="ask-form" onSubmit={handleSubmit}>
        <label className="sr-only" htmlFor="ask-message">
          问题
        </label>
        <textarea
          id="ask-message"
          value={input}
          onChange={(event) => setInput(event.target.value)}
          onKeyDown={handleInputKeyDown}
          placeholder="向知识库提问"
          rows={3}
        />
        <button className="primary-button" disabled={isSending || input.trim().length === 0} type="submit">
          {isSending ? "发送中" : "发送"}
        </button>
      </form>
    </section>
  );
}
