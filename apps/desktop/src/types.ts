export interface Config {
  port: number;
  model: string;
  folder: string | null;
  knowledge_id: string | null;
  theme: "dark" | "light";
  setup_complete: boolean;
  allow_downloads: boolean;
  max_file_mb: number;
}
export interface ServiceStatus {
  state: string;
  detail: string;
}
export interface Health {
  docker: ServiceStatus;
  ollama: ServiceStatus;
  backend: ServiceStatus;
  folder: ServiceStatus;
  authenticated: boolean;
  credentials_unlocked: boolean;
  data_dir: string;
}
export interface Model {
  name: string;
  size: number;
}
export interface Knowledge {
  id: string;
  name: string;
  write_access?: boolean;
}
export interface Preview {
  id: string;
  added: number;
  modified: number;
  unchanged: number;
  skipped: number;
  deletions: string[];
  scanned: number;
}
export interface SyncResult {
  added: number;
  modified: number;
  unchanged: number;
  skipped: number;
  failed: number;
  deleted: number;
  retained: number;
  errors: string[];
  completed_at: number;
  folder: string;
  knowledge_id: string;
}
export interface SyncState {
  attempt: SyncResult | null;
  success: SyncResult | null;
}
export interface Message {
  role: "user" | "assistant";
  content: string;
}
export interface Conversation {
  id: string;
  title: string;
  messages: Message[];
  model: string;
  updated: number;
}
export interface StreamEvent {
  choices?: { delta?: { content?: string } }[];
  sources?: { source?: { name?: string }; metadata?: { name?: string }[] }[];
  error?: unknown;
}
export type Page = "dashboard" | "chat" | "knowledge" | "settings" | "setup";
