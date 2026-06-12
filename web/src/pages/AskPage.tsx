import { FormEvent, KeyboardEvent, UIEvent, useEffect, useMemo, useRef, useState } from "react";
import { Pencil, Trash2 } from "lucide-react";
import ReactMarkdown from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import remarkGfm from "remark-gfm";
import {
  askEventsUrl,
  askVault,
  createAskSession,
  deleteAskSession,
  getAskSessionMessages,
  listAskSessions,
  renameAskSession
} from "../api";
import type { AskActivityEvent, ChatMessage, ChatSession } from "../types";

export function AskPage() {
  const [input, setInput] = useState("");
  const [sessionName, setSessionName] = useState("");
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState("default");
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [isLoadingMessages, setIsLoadingMessages] = useState(false);
  const [agentActivity, setAgentActivity] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const messageListRef = useRef<HTMLDivElement | null>(null);
  const bottomAnchorRef = useRef<HTMLDivElement | null>(null);
  const pendingScrollSession = useRef<string | null>(null);
  const restoreScrollFrames = useRef<number[]>([]);
  const shouldStickToBottom = useRef(true);
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
    pendingScrollSession.current = activeSessionId;
    shouldStickToBottom.current = true;
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

  useEffect(() => {
    const shouldScroll =
      !isLoadingMessages &&
      (pendingScrollSession.current === activeSessionId || shouldStickToBottom.current);
    if (!shouldScroll) {
      return;
    }

    const messageList = messageListRef.current;
    const bottomAnchor = bottomAnchorRef.current;
    if (!messageList || !bottomAnchor) {
      return;
    }

    restoreScrollFrames.current.forEach((frame) => cancelAnimationFrame(frame));
    restoreScrollFrames.current = [];

    const firstFrame = requestAnimationFrame(() => {
      const secondFrame = requestAnimationFrame(() => {
        bottomAnchor.scrollIntoView?.({ block: "end" });
        messageList.scrollTop = messageList.scrollHeight;
        pendingScrollSession.current = null;
        restoreScrollFrames.current = [];
      });
      restoreScrollFrames.current = [secondFrame];
    });
    restoreScrollFrames.current = [firstFrame];

    return () => {
      restoreScrollFrames.current.forEach((frame) => cancelAnimationFrame(frame));
      restoreScrollFrames.current = [];
    };
  }, [activeSessionId, agentActivity, isLoadingMessages, isSending, messages]);

  useEffect(() => {
    if (typeof EventSource === "undefined") {
      return;
    }

    const events = new EventSource(askEventsUrl(activeSessionId));
    events.addEventListener("agent", (event) => {
      const activity = JSON.parse((event as MessageEvent).data) as AskActivityEvent;
      if (activity.kind === "agent_end" || activity.kind === "error") {
        setAgentActivity(null);
        return;
      }
      setAgentActivity(activity.label);
    });

    return () => {
      events.close();
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
    shouldStickToBottom.current = true;
    setAgentActivity("正在思考");
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
      setAgentActivity(null);
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

  async function handleRenameSession(session: ChatSession) {
    const name = window.prompt("新的会话名称", session.name)?.trim();
    if (!name || name === session.name) {
      return;
    }

    try {
      const renamed = await renameAskSession(session.id, name);
      setSessions((current) => current.map((item) => (item.id === session.id ? renamed : item)));
      if (activeSessionId === session.id) {
        setActiveSessionId(renamed.id);
      }
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "重命名会话失败");
    }
  }

  async function handleDeleteSession(session: ChatSession) {
    if (!window.confirm(`删除会话“${session.name}”？`)) {
      return;
    }

    try {
      await deleteAskSession(session.id);
      const nextSessions = sessions.filter((item) => item.id !== session.id);
      setSessions(nextSessions);
      if (activeSessionId === session.id) {
        setActiveSessionId(nextSessions[0]?.id ?? "default");
      }
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "删除会话失败");
    }
  }

  function handleInputKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) {
      return;
    }

    event.preventDefault();
    void sendMessage();
  }

  function handleMessageListScroll(event: UIEvent<HTMLDivElement>) {
    if (pendingScrollSession.current === activeSessionId) {
      return;
    }

    const messageList = event.currentTarget;
    const distanceFromBottom =
      messageList.scrollHeight - messageList.scrollTop - messageList.clientHeight;
    shouldStickToBottom.current = distanceFromBottom < 80;
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
              <div
                className={`session-item ${session.id === activeSessionId ? "active" : ""}`}
                key={session.id}
              >
                <button className="session-select" onClick={() => setActiveSessionId(session.id)} type="button">
                  <span>{session.name}</span>
                  {session.updated_at && <small>{new Date(session.updated_at).toLocaleString()}</small>}
                </button>
                <div className="session-actions">
                  <button
                    aria-label={`重命名会话 ${session.name}`}
                    className="icon-button"
                    onClick={() => void handleRenameSession(session)}
                    title="重命名"
                    type="button"
                  >
                    <Pencil aria-hidden="true" size={15} />
                  </button>
                  <button
                    aria-label={`删除会话 ${session.name}`}
                    className="icon-button danger"
                    onClick={() => void handleDeleteSession(session)}
                    title="删除"
                    type="button"
                  >
                    <Trash2 aria-hidden="true" size={15} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </aside>
        <div
          className="message-list"
          aria-label="消息"
          aria-live="polite"
          onScroll={handleMessageListScroll}
          ref={messageListRef}
        >
          {isLoadingMessages ? (
            <p className="muted">加载消息中</p>
          ) : (
            messages.map((message, index) => (
              <article className={`message ${message.role}`} key={`${message.role}-${index}`}>
                <span>{message.role === "user" ? "你" : "助手"}</span>
                <MessageBody message={message} />
              </article>
            ))
          )}
          {!isLoadingMessages && isSending && (
            <article className="message assistant thinking" aria-label="助手正在思考" role="status">
              <span>助手</span>
              <div className="thinking-indicator">
                <span aria-hidden="true" />
                <span aria-hidden="true" />
                <span aria-hidden="true" />
                <p>{agentActivity ?? "正在思考"}</p>
              </div>
            </article>
          )}
          <div aria-hidden="true" ref={bottomAnchorRef} />
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

function MessageBody({ message }: { message: ChatMessage }) {
  if (message.role === "assistant") {
    return (
      <div className="markdown-body">
        <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeSanitize]}>
          {message.content}
        </ReactMarkdown>
      </div>
    );
  }

  return <p className="plain-message">{message.content}</p>;
}
