import { useEffect, useState } from "react";
import { applyConfirmation, listConfirmations, rejectConfirmation, runMaintenanceScan } from "../api";
import type { ConfirmationQueue, MaintenanceInbox } from "../types";

type DiffLine = {
  kind: "unchanged" | "removed" | "added";
  oldLine: number | null;
  newLine: number | null;
  content: string;
};

function splitLines(value: string) {
  const lines = value.replace(/\r\n/g, "\n").split("\n");
  if (lines.length > 1 && lines[lines.length - 1] === "") {
    return lines.slice(0, -1);
  }
  return lines;
}

function buildLineDiff(original: string, proposed: string): DiffLine[] {
  const originalLines = splitLines(original);
  const proposedLines = splitLines(proposed);
  let prefix = 0;

  while (
    prefix < originalLines.length &&
    prefix < proposedLines.length &&
    originalLines[prefix] === proposedLines[prefix]
  ) {
    prefix += 1;
  }

  let suffix = 0;
  while (
    suffix < originalLines.length - prefix &&
    suffix < proposedLines.length - prefix &&
    originalLines[originalLines.length - 1 - suffix] === proposedLines[proposedLines.length - 1 - suffix]
  ) {
    suffix += 1;
  }

  const diff: DiffLine[] = [];
  for (let index = 0; index < prefix; index += 1) {
    diff.push({ kind: "unchanged", oldLine: index + 1, newLine: index + 1, content: originalLines[index] });
  }

  for (let index = prefix; index < originalLines.length - suffix; index += 1) {
    diff.push({ kind: "removed", oldLine: index + 1, newLine: null, content: originalLines[index] });
  }

  for (let index = prefix; index < proposedLines.length - suffix; index += 1) {
    diff.push({ kind: "added", oldLine: null, newLine: index + 1, content: proposedLines[index] });
  }

  const originalSuffixStart = originalLines.length - suffix;
  const proposedSuffixStart = proposedLines.length - suffix;
  for (let index = 0; index < suffix; index += 1) {
    diff.push({
      kind: "unchanged",
      oldLine: originalSuffixStart + index + 1,
      newLine: proposedSuffixStart + index + 1,
      content: originalLines[originalSuffixStart + index]
    });
  }

  return diff;
}

function diffMarker(kind: DiffLine["kind"]) {
  if (kind === "added") {
    return "+";
  }
  if (kind === "removed") {
    return "-";
  }
  return " ";
}

export function MaintenancePage() {
  const [inbox, setInbox] = useState<MaintenanceInbox | null>(null);
  const [confirmations, setConfirmations] = useState<ConfirmationQueue | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [isLoadingConfirmations, setIsLoadingConfirmations] = useState(true);
  const [busyConfirmationId, setBusyConfirmationId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void loadConfirmations();
  }, []);

  async function loadConfirmations() {
    setIsLoadingConfirmations(true);
    try {
      setConfirmations(await listConfirmations());
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "加载确认队列失败");
    } finally {
      setIsLoadingConfirmations(false);
    }
  }

  async function handleScan() {
    setIsScanning(true);
    setError(null);
    try {
      setInbox(await runMaintenanceScan());
    } catch (err) {
      setError(err instanceof Error ? err.message : "扫描失败");
    } finally {
      setIsScanning(false);
    }
  }

  async function handleApplyConfirmation(id: string) {
    setBusyConfirmationId(id);
    setError(null);
    try {
      await applyConfirmation(id);
      await loadConfirmations();
    } catch (err) {
      setError(err instanceof Error ? err.message : "确认失败");
    } finally {
      setBusyConfirmationId(null);
    }
  }

  async function handleRejectConfirmation(id: string) {
    setBusyConfirmationId(id);
    setError(null);
    try {
      await rejectConfirmation(id);
      await loadConfirmations();
    } catch (err) {
      setError(err instanceof Error ? err.message : "拒绝失败");
    } finally {
      setBusyConfirmationId(null);
    }
  }

  return (
    <section className="page">
      <header className="page-header">
        <h2>维护扫描</h2>
        <div className="page-actions">
          <button className="secondary-button" type="button" onClick={loadConfirmations} disabled={isLoadingConfirmations}>
            刷新确认队列
          </button>
          <button className="primary-button" type="button" onClick={handleScan} disabled={isScanning}>
            {isScanning ? "扫描中" : "开始扫描"}
          </button>
        </div>
      </header>
      {error && <p className="error-text">{error}</p>}
      <section className="maintenance-section">
        <h3>待确认修改</h3>
        {isLoadingConfirmations ? (
          <p className="muted">加载确认队列中</p>
        ) : (
          <div className="inbox-list">
            {confirmations?.items.map((item) => (
              <article className="confirmation-item" key={item.id}>
                <div className="confirmation-header">
                  <div>
                    <h4>{item.path}</h4>
                    <p>{item.reason ?? "无说明"}</p>
                  </div>
                  <div className="confirmation-actions">
                    <button
                      className="secondary-button"
                      disabled={busyConfirmationId === item.id}
                      onClick={() => void handleRejectConfirmation(item.id)}
                      type="button"
                    >
                      拒绝
                    </button>
                    <button
                      className="primary-button"
                      disabled={busyConfirmationId === item.id}
                      onClick={() => void handleApplyConfirmation(item.id)}
                      type="button"
                    >
                      确认应用
                    </button>
                  </div>
                </div>
                <div className="confirmation-diff-wrap">
                  <span>修改预览</span>
                  <div className="confirmation-diff" aria-label={`修改预览 ${item.path}`}>
                    {buildLineDiff(item.original_content, item.proposed_content).map((line, index) => (
                      <div className={`diff-line ${line.kind}`} key={`${line.kind}-${line.oldLine ?? "n"}-${line.newLine ?? "n"}-${index}`}>
                        <span className="diff-gutter">{line.oldLine ?? ""}</span>
                        <span className="diff-gutter">{line.newLine ?? ""}</span>
                        <span className="diff-marker">{diffMarker(line.kind)}</span>
                        <code>{line.content || " "}</code>
                      </div>
                    ))}
                  </div>
                </div>
              </article>
            ))}
            {confirmations && confirmations.items.length === 0 && <p>没有待确认修改。</p>}
          </div>
        )}
      </section>
      <section className="maintenance-section">
        <h3>扫描结果</h3>
        <div className="inbox-list">
          {inbox?.items.map((item, index) => (
            <article className="inbox-item" key={`${item.file}-${item.kind}-${index}`}>
              <span className="priority">{item.priority}</span>
              <div>
                <h3>{item.kind}</h3>
                <p>{item.file}</p>
                <p>{item.evidence}</p>
              </div>
            </article>
          ))}
          {inbox && inbox.items.length === 0 && <p>没有发现维护问题。</p>}
        </div>
      </section>
    </section>
  );
}
