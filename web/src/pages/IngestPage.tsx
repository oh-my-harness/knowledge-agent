import { FormEvent, useEffect, useMemo, useState } from "react";
import ReactMarkdown from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import remarkGfm from "remark-gfm";
import { askVault, listPdfAssets } from "../api";
import type { PdfAsset } from "../types";

type SourceKind = "url" | "pdf";

export function IngestPage() {
  const [sourceKind, setSourceKind] = useState<SourceKind>("url");
  const [source, setSource] = useState("");
  const [pdfs, setPdfs] = useState<PdfAsset[]>([]);
  const [isLoadingPdfs, setIsLoadingPdfs] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setIsLoadingPdfs(true);
    listPdfAssets()
      .then((items) => {
        setPdfs(items);
        setError(null);
      })
      .catch((err) => setError(err instanceof Error ? err.message : "加载 PDF 列表失败"))
      .finally(() => setIsLoadingPdfs(false));
  }, []);

  const prompt = useMemo(() => buildIngestPrompt(sourceKind, source), [sourceKind, source]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = source.trim();
    if (!trimmed || isSubmitting) {
      return;
    }

    setIsSubmitting(true);
    setResult(null);
    setError(null);
    try {
      const response = await askVault(prompt, "ingest");
      setResult(response.answer);
    } catch (err) {
      setError(err instanceof Error ? err.message : "资料摄入失败");
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <section className="page ingest-page">
      <header className="page-header">
        <h2>资料摄入</h2>
        <span>网页与 PDF</span>
      </header>

      <form className="ingest-form" onSubmit={handleSubmit}>
        <div className="segmented-control" role="group" aria-label="资料类型">
          <button
            className={sourceKind === "url" ? "active" : ""}
            onClick={() => setSourceKind("url")}
            type="button"
          >
            网页链接
          </button>
          <button
            className={sourceKind === "pdf" ? "active" : ""}
            onClick={() => setSourceKind("pdf")}
            type="button"
          >
            本地 PDF
          </button>
        </div>

        <label htmlFor="ingest-source">{sourceKind === "url" ? "网页 URL" : "PDF 路径"}</label>
        <input
          id="ingest-source"
          list={sourceKind === "pdf" ? "pdf-assets" : undefined}
          onChange={(event) => setSource(event.target.value)}
          placeholder={sourceKind === "url" ? "https://example.com/article" : "assets/papers/example.pdf"}
          value={source}
        />
        <datalist id="pdf-assets">
          {pdfs.map((pdf) => (
            <option key={pdf.path} value={pdf.path} />
          ))}
        </datalist>

        {sourceKind === "pdf" && (
          <div className="pdf-picker" aria-label="PDF 文件">
            {isLoadingPdfs && <p className="muted">加载 PDF 列表中</p>}
            {!isLoadingPdfs && pdfs.length === 0 && <p className="muted">当前知识库中没有 PDF</p>}
            {pdfs.map((pdf) => (
              <button key={pdf.path} onClick={() => setSource(pdf.path)} type="button">
                <span>{pdf.path}</span>
                <small>{formatBytes(pdf.bytes)}</small>
              </button>
            ))}
          </div>
        )}

        <button className="primary-button" disabled={isSubmitting || source.trim().length === 0} type="submit">
          {isSubmitting ? "摄入中" : "开始摄入"}
        </button>
      </form>

      {error && <p className="error-text">{error}</p>}
      {result && (
        <article className="ingest-result markdown-body">
          <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeSanitize]}>
            {result}
          </ReactMarkdown>
        </article>
      )}
    </section>
  );
}

function buildIngestPrompt(sourceKind: SourceKind, source: string): string {
  const trimmed = source.trim();
  if (sourceKind === "url") {
    return `请摄入这个网页链接：${trimmed}

要求：
1. 必须先使用 web_fetch_page 读取网页正文。
2. 总结核心内容、关键概念、适用范围和待验证问题。
3. 使用 vault_find_related_notes 查找现有知识库中的相关笔记。
4. 生成一张符合 Obsidian 的 Markdown 资料卡，frontmatter 至少包含 title、tags、created、updated、source_url、source_type: webpage。
5. 资料卡中必须包含“关联笔记”小节，用 wikilink 关联现有相关笔记。
6. 如果适合写入知识库，请使用 vault_create_note 创建新资料卡，并使用 vault_append_index_entry 维护对应 _index.md；如果不确定路径，请先给出建议路径并说明原因。`;
  }

  return `请摄入这个知识库 PDF：${trimmed}

要求：
1. 必须先使用 vault_read_pdf_text 读取 PDF 文本。
2. 总结核心内容、关键概念、适用范围和待验证问题。
3. 使用 vault_find_related_notes 查找现有知识库中的相关笔记。
4. 生成一张符合 Obsidian 的 Markdown 资料卡，frontmatter 至少包含 title、tags、created、updated、source、source_type: pdf。
5. 资料卡中必须包含“原文”链接和“关联笔记”小节，用 wikilink 关联现有相关笔记。
6. 如果适合写入知识库，请使用 vault_create_note 创建新资料卡，并使用 vault_append_index_entry 维护对应 _index.md；如果 PDF 无法提取有效文字，请明确说明。`;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
