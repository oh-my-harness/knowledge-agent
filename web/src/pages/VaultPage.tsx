import { useEffect, useState } from "react";
import { getVaultIndex } from "../api";
import type { VaultScan } from "../types";

export function VaultPage() {
  const [scan, setScan] = useState<VaultScan | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getVaultIndex()
      .then(setScan)
      .catch((err: Error) => setError(err.message));
  }, []);

  return (
    <section className="page">
      <header className="page-header">
        <h2>知识库</h2>
        {scan && <span>{scan.notes.length} 篇 Markdown</span>}
      </header>
      {error && <p className="error-text">{error}</p>}
      <div className="note-list">
        {scan?.notes.map((note) => (
          <article className="note-row" key={note.relative_path}>
            <div>
              <h3>{note.title ?? note.relative_path}</h3>
              <p>{note.relative_path}</p>
            </div>
            <div className="tag-list">
              {note.tags.map((tag) => (
                <span className="tag" key={tag}>
                  #{tag}
                </span>
              ))}
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}
