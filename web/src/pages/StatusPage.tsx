import { useEffect, useState } from "react";
import { getHealth } from "../api";

export function StatusPage() {
  const [status, setStatus] = useState<"loading" | "online" | "offline">("loading");

  useEffect(() => {
    getHealth()
      .then(() => setStatus("online"))
      .catch(() => setStatus("offline"));
  }, []);

  return (
    <section className="page">
      <header className="page-header">
        <h2>服务状态</h2>
      </header>
      <div className={`status-pill ${status}`}>
        {status === "loading" && "检查中"}
        {status === "online" && "服务在线"}
        {status === "offline" && "服务离线"}
      </div>
    </section>
  );
}
