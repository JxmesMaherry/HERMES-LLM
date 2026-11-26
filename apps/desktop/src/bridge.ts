import { invoke, Channel, isTauri } from "@tauri-apps/api/core";
import type {
  Config,
  Health,
  Model,
  Knowledge,
  Preview,
  SyncResult,
  SyncState,
  Conversation,
  Message,
  StreamEvent,
} from "./types";
const call = <T>(command: string, args?: Record<string, unknown>) => {
  if (!isTauri())
    return Promise.reject(
      new Error(
        "Open the HERMES desktop app with npm run desktop. Native services are unavailable in a browser.",
      ),
    );
  return invoke<T>(command, args);
};
export const bridge = {
  unlock: () => call<void>("unlock_credentials"),
  config: () => call<Config>("get_config"),
  save: (settings: Config) => call<Config>("save_config", { settings }),
  health: () => call<Health>("health"),
  backend: (action: string) => call<string>("backend_action", { action }),
  open: (target: string) => call<void>("open_target", { target }),
  auth: (email: string, password: string, create: boolean) =>
    call<void>("authenticate", { email, password, create }),
  signOut: () => call<void>("sign_out"),
  models: async () => (await call<{ models: Model[] }>("list_models")).models,
  pull: (
    model: string,
    update: (progress: {
      status?: string;
      completed?: number;
      total?: number;
    }) => void,
  ) => {
    const progress = new Channel<{
      status?: string;
      completed?: number;
      total?: number;
    }>();
    progress.onmessage = update;
    return call<void>("pull_model", { model, progress });
  },
  folder: () => call<Config>("choose_folder"),
  knowledge: async () =>
    (await call<{ items: Knowledge[] }>("list_knowledge")).items,
  createKnowledge: (name: string) =>
    call<Knowledge>("create_knowledge", { name }),
  preview: () => call<Preview>("preview_sync"),
  sync: (
    previewId: string,
    confirmDeletions: boolean,
    update: (p: { done: number; total: number; filename: string }) => void,
  ) => {
    const progress = new Channel<{
      done: number;
      total: number;
      filename: string;
    }>();
    progress.onmessage = update;
    return call<SyncResult>("apply_sync", {
      previewId,
      confirmDeletions,
      progress,
    });
  },
  lastSync: () => call<SyncState>("last_sync"),
  history: () => call<Conversation[]>("load_history"),
  saveHistory: (conversations: Conversation[]) =>
    call<void>("save_history", { conversations }),
  chat: (
    requestId: string,
    model: string,
    messages: Message[],
    useKnowledge: boolean,
    update: (event: StreamEvent) => void,
  ) => {
    const events = new Channel<StreamEvent>();
    events.onmessage = update;
    return call<void>("chat_send", {
      requestId,
      model,
      messages,
      useKnowledge,
      events,
    });
  },
  cancel: (requestId: string) => call<void>("cancel_chat", { requestId }),
};
export function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function singleFlight<T>(action: () => Promise<T>): () => Promise<T> {
  let pending: Promise<T> | null = null;
  return () => {
    if (pending) return pending;
    pending = action().finally(() => {
      pending = null;
    });
    return pending;
  };
}
