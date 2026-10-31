import { Fragment } from "react";

/** Inline `code` spans only - deliberately tiny, no markdown dependency. */
function Inline({ text }: { text: string }) {
  const bits = text.split(/(`[^`]+`)/g);
  return (
    <>
      {bits.map((bit, i) =>
        bit.length > 2 && bit.startsWith("`") && bit.endsWith("`") ? (
          <code className="inline-code" key={i}>
            {bit.slice(1, -1)}
          </code>
        ) : (
          <Fragment key={i}>{bit}</Fragment>
        ),
      )}
    </>
  );
}

function Paragraphs({ text }: { text: string }) {
  const blocks = text.split(/\n{2,}/).filter((b) => b.trim().length > 0);
  if (blocks.length === 0) return null;
  return (
    <>
      {blocks.map((block, i) => (
        <p key={i}>
          <Inline text={block} />
        </p>
      ))}
    </>
  );
}

/** Renders fenced code blocks and paragraphs. Good enough for agent output. */
export function Rich({ text }: { text: string }) {
  const parts = text.split("```");

  return (
    <div className="rich">
      {parts.map((part, i) => {
        if (i % 2 === 1) {
          const newline = part.indexOf("\n");
          const lang = newline > 0 ? part.slice(0, newline).trim() : "";
          const code = newline > 0 ? part.slice(newline + 1) : part;
          return (
            <pre className="codeblock" key={i}>
              {lang ? <span className="codeblock-lang">{lang}</span> : null}
              <code>{code.replace(/\n$/, "")}</code>
            </pre>
          );
        }
        return <Paragraphs key={i} text={part} />;
      })}
    </div>
  );
}
