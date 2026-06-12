import { FormEvent, useEffect, useState } from "react";
import { getLocalSettings, saveLocalSettings } from "../api";
import type { LocalSettings } from "../types";

const defaultSettings: LocalSettings = {
  llm: {
    provider: "deepseek",
    deepseek_api_key: null,
    deepseek_model: "deepseek-v4-flash"
  },
  web_search: {
    enabled: false,
    provider: "duckduckgo"
  }
};

export function SettingsPage() {
  const [settings, setSettings] = useState<LocalSettings>(defaultSettings);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getLocalSettings()
      .then((loaded) => {
        setSettings(loaded);
        setError(null);
      })
      .catch((err) => {
        setError(err instanceof Error ? err.message : "加载设置失败");
      })
      .finally(() => setIsLoading(false));
  }, []);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsSaving(true);
    setError(null);
    setMessage(null);

    try {
      const saved = await saveLocalSettings({
        llm: {
          ...settings.llm,
          deepseek_api_key: settings.llm.deepseek_api_key?.trim() || null,
          deepseek_model: settings.llm.deepseek_model.trim() || "deepseek-v4-flash"
        },
        web_search: settings.web_search
      });
      setSettings(saved);
      setMessage("设置已保存。LLM 和网页搜索配置会在服务重启后用于新 runner。");
    } catch (err) {
      setError(err instanceof Error ? err.message : "保存设置失败");
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <section className="page settings-page">
      <header className="page-header">
        <h2>设置</h2>
        <span>.knowledge-agent/local.toml</span>
      </header>

      {isLoading ? (
        <p className="muted">加载设置中</p>
      ) : (
        <form className="settings-form" onSubmit={handleSubmit}>
          <section className="settings-section">
            <h3>模型</h3>
            <label className="field">
              <span>Provider</span>
              <select
                value={settings.llm.provider}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    llm: { ...current.llm, provider: event.target.value }
                  }))
                }
              >
                <option value="deepseek">DeepSeek</option>
              </select>
            </label>

            <label className="field">
              <span>DeepSeek API Key</span>
              <input
                autoComplete="off"
                type="password"
                value={settings.llm.deepseek_api_key ?? ""}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    llm: { ...current.llm, deepseek_api_key: event.target.value }
                  }))
                }
                placeholder="sk-..."
              />
            </label>

            <label className="field">
              <span>模型名</span>
              <input
                value={settings.llm.deepseek_model}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    llm: { ...current.llm, deepseek_model: event.target.value }
                  }))
                }
                placeholder="deepseek-v4-flash"
              />
            </label>
          </section>

          <section className="settings-section">
            <h3>网页搜索</h3>
            <label className="check-field">
              <input
                type="checkbox"
                checked={settings.web_search.enabled}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    web_search: {
                      ...current.web_search,
                      enabled: event.target.checked,
                      provider: event.target.checked ? "duckduckgo" : current.web_search.provider
                    }
                  }))
                }
              />
              <span>启用网页搜索工具</span>
            </label>

            <label className="field">
              <span>Provider</span>
              <select
                value={settings.web_search.provider}
                onChange={(event) =>
                  setSettings((current) => ({
                    ...current,
                    web_search: { ...current.web_search, provider: event.target.value }
                  }))
                }
              >
                <option value="duckduckgo">DuckDuckGo</option>
              </select>
            </label>
          </section>

          {error && <p className="error-text">{error}</p>}
          {message && <p className="success-text">{message}</p>}

          <div className="settings-actions">
            <button className="primary-button" disabled={isSaving} type="submit">
              {isSaving ? "保存中" : "保存设置"}
            </button>
          </div>
        </form>
      )}
    </section>
  );
}
