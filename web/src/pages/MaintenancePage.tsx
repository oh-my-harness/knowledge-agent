import { useEffect, useState } from "react";
import { applyConfirmation, listConfirmations, rejectConfirmation, runMaintenanceScan } from "../api";
import type { ConfirmationQueue, MaintenanceInbox } from "../types";

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
                <div className="confirmation-preview">
                  <div>
                    <span>当前内容</span>
                    <pre>{item.original_content}</pre>
                  </div>
                  <div>
                    <span>拟写入内容</span>
                    <pre>{item.proposed_content}</pre>
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
