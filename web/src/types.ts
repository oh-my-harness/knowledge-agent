export interface HealthResponse {
  status: "ok";
}

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
