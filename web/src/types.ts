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
