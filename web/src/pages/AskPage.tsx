import { FormEvent, useState } from "react";
import { askVault } from "../api";

interface ChatMessage {
  role: "user" | "assistant";
  content: string;
}

export function AskPage() {
  const [input, setInput] = useState("");
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isSending, setIsSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const message = input.trim();
    if (!message || isSending) {
      return;
    }

    setMessages((current) => [...current, { role: "user", content: message }]);
    setInput("");
    setIsSending(true);
    setError(null);

    try {
      const response = await askVault(message);
      setMessages((current) => [...current, { role: "assistant", content: response.answer }]);
    } catch (err) {
      setError(err instanceof Error ? err.message : "发送失败");
    } finally {
      setIsSending(false);
    }
  }

  return (
    <section className="page ask-page">
      <header className="page-header">
        <h2>提问</h2>
      </header>
      <div className="message-list" aria-live="polite">
        {messages.map((message, index) => (
          <article className={`message ${message.role}`} key={`${message.role}-${index}`}>
            <span>{message.role === "user" ? "你" : "助手"}</span>
            <p>{message.content}</p>
          </article>
        ))}
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
