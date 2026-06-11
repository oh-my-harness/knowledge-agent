import { useState } from "react";
import { runMaintenanceScan } from "../api";
import type { MaintenanceInbox } from "../types";

export function MaintenancePage() {
  const [inbox, setInbox] = useState<MaintenanceInbox | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);

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

  return (
    <section className="page">
      <header className="page-header">
        <h2>维护扫描</h2>
        <button className="primary-button" type="button" onClick={handleScan} disabled={isScanning}>
          {isScanning ? "扫描中" : "开始扫描"}
        </button>
      </header>
      {error && <p className="error-text">{error}</p>}
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
  );
}
