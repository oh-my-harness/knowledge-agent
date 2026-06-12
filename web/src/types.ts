export interface WikiLink {
  target: string;
  alias: string | null;
}

export interface ScannedNote {
  relative_path: string;
  title: string | null;
  note_type: string | null;
  tags: string[];
  links: WikiLink[];
}

export interface VaultScan {
  root: string;
  notes: ScannedNote[];
}

export interface MaintenanceItem {
  priority: string;
  kind: string;
  file: string;
  evidence: string;
  requires_confirmation: boolean;
}

export interface MaintenanceInbox {
  items: MaintenanceItem[];
}

export interface ConfirmationQueue {
  items: ConfirmationItem[];
}

export interface ConfirmationItem {
  id: string;
  kind: "replace_note";
  path: string;
  reason: string | null;
  original_content: string;
  proposed_content: string;
  created_at: string;
}

export interface LocalSettings {
  llm: {
    provider: string;
    deepseek_api_key: string | null;
    deepseek_model: string;
  };
  web_search: {
    enabled: boolean;
    provider: string;
  };
}

export interface AskRequest {
  message: string;
  session_id?: string;
  mode: "vault";
}

export interface ChatSession {
  id: string;
  name: string;
  updated_at: string | null;
}

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
}

export interface AskSource {
  title: string;
  path: string;
}

export interface AskResponse {
  answer: string;
  sources: AskSource[];
  requires_followup: boolean;
}

export interface AskActivityEvent {
  kind: string;
  label: string;
  detail: string | null;
}
