import { useEffect, useRef, useState } from "react";
import type { ChatItem, LiveTool } from "../lib/types";
import { Rich } from "./Rich";
import { ToolCallCard } from "./ToolCallCard";

interface Props {
  items: ChatItem[];
  streaming: string;
  liveTools: Record<string, LiveTool>;
  busy: boolean;
  error: string | null;
  onSend: (text: string) => void;
}

const SUGGESTIONS = [
  "What is in this project, and how do I run the tests?",
  "Find every TODO comment and group them by file.",
  "Bump the version and show me the diff before you write.",
];

export function ChatPane({ items, streaming, liveTools, busy, error, onSend }: Props) {
  const [draft, setDraft] = useState("");
  const endRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [items, streaming, liveTools, error]);

  const submit = () => {
    const text = draft.trim();
    if (!text || busy) return;
    setDraft("");
    onSend(text);
  };

  const liveList = Object.entries(liveTools);

  return (
    <main className="chat">
      <div className="chat-scroll">
        {items.length === 0 && !streaming ? (
          <div className="empty">
            <h1>What should I work on?</h1>
            <p>
              Kestrel runs an agent against your own machine. It reads files, runs
              commands and edits code - and asks before it changes anything.
            </p>
            <div className="suggestions">
              {SUGGESTIONS.map((s) => (
                <button key={s} className="suggestion" onClick={() => onSend(s)}>
                  {s}
                </button>
              ))}
            </div>
          </div>
        ) : null}

        {items.map((item) => (
          <article className={`msg ${item.role}`} key={item.id}>
            <div className="msg-label">{item.role === "user" ? "You" : "Kestrel"}</div>
            {item.text ? (
              item.role === "user" ? (
                <div className="msg-user-text">{item.text}</div>
              ) : (
                <Rich text={item.text} />
              )
            ) : null}
            {item.toolCalls.map((call) => (
              <ToolCallCard
                key={call.id}
                id={call.id}
                name={call.name}
                args={call.arguments}
                result={item.results[call.id]}
              />
            ))}
          </article>
        ))}

        {streaming || liveList.length > 0 ? (
          <article className="msg assistant">
            <div className="msg-label">
              Kestrel
              <span className="typing" aria-label="working" />
            </div>
            {streaming ? <Rich text={streaming} /> : null}
            {liveList.map(([id, tool]) => (
              <ToolCallCard
                key={id}
                id={id}
                name={tool.name}
                args={tool.args}
                running
              />
            ))}
          </article>
        ) : null}

        {error ? (
          <div className="error-box">
            <strong>Something went wrong</strong>
            <span>{error}</span>
          </div>
        ) : null}

        <div ref={endRef} />
      </div>

      <div className="composer">
        <textarea
          ref={inputRef}
          value={draft}
          placeholder={
            busy ? "Kestrel is working..." : "Describe a task. Enter to send, Shift+Enter for a new line."
          }
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              submit();
            }
          }}
          rows={3}
        />
        <div className="composer-row">
          <span className="composer-hint">
            Mutating tools always ask first.
          </span>
          <button className="btn primary" onClick={submit} disabled={busy || !draft.trim()}>
            {busy ? "Working" : "Send"}
          </button>
        </div>
      </div>
    </main>
  );
}
