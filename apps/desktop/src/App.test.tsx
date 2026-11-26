import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import App from "./App";
import PasswordDialog from "./PasswordDialog";
import AccountRecovery from "./AccountRecovery";
import Dashboard from "./Dashboard";
import Setup, { type Controls } from "./Setup";
import Knowledge from "./Knowledge";
import Chat from "./Chat";
import { bridge } from "./bridge";
import type { Config, Health, StreamEvent } from "./types";
vi.mock("./bridge", async () => {
  const actual = await vi.importActual<typeof import("./bridge")>("./bridge");
  return {
    ...actual,
    bridge: {
      ...actual.bridge,
      config: vi.fn(),
      credentialStatus: vi.fn(async () => ({ exists: true, unlocked: false })),
      createCredentials: vi.fn(async () => {}),
      recoveryAccounts: vi.fn(async () => ({
        accounts: [{ id: "owner", name: "Owner", email: "owner@example.test" }],
      })),
      resetLocalAccount: vi.fn(async () => {}),
      health: vi.fn(),
      models: vi.fn(async () => []),
      knowledge: vi.fn(async () => []),
      lastSync: vi.fn(async () => ({ attempt: null, success: null })),
      unlock: vi.fn(async () => {}),
      history: vi.fn(async () => []),
      saveHistory: vi.fn(async () => {}),
      chat: vi.fn(),
      preview: vi.fn(),
      sync: vi.fn(),
      open: vi.fn(),
    },
    errorText: (e: unknown) => String(e),
  };
});
const config: Config = {
  port: 3000,
  model: "qwen3:4b-instruct",
  folder: "/tmp/My Documents",
  knowledge_id: "test-kb",
  theme: "dark",
  setup_complete: false,
  allow_downloads: false,
  max_file_mb: 20,
};
const health: Health = {
  docker: { state: "stopped", detail: "Open Docker Desktop" },
  ollama: { state: "missing", detail: "Install Ollama" },
  backend: { state: "failed", detail: "Health check failed" },
  authenticated: false,
  credentials_unlocked: true,
  folder: { state: "ready", detail: "/tmp/My Documents" },
  data_dir: "/tmp/hermes",
};
function controls(): Controls {
  return {
    config,
    health,
    models: [],
    knowledge: [],
    busy: "",
    save: vi.fn(async () => {}),
    run: async (_n, action) => {
      await action();
    },
    refresh: vi.fn(async () => {}),
    chooseFolder: vi.fn(async () => {}),
  };
}
describe("honest prerequisite states", () => {
  it("shows failed, missing and stopped service states", () => {
    render(
      <Dashboard
        {...controls()}
        navigate={vi.fn()}
        sync={{ attempt: null, success: null }}
      />,
    );
    expect(screen.getByText("failed")).toBeVisible();
    expect(screen.getByText("missing")).toBeVisible();
    expect(screen.getByText("stopped")).toBeVisible();
    expect(
      screen.getByText(
        "Selected model is unavailable. Choose or download a model.",
      ),
    ).toBeVisible();
  });
  it("does not let setup pass unavailable prerequisites", () => {
    render(<Setup {...controls()} finish={vi.fn()} />);
    expect(screen.getByRole("button", { name: /Continue/ })).toBeDisabled();
    expect(screen.getByText("Install Ollama")).toBeVisible();
  });
});
describe("safe document synchronisation", () => {
  it("previews deletion names and retains them unless explicitly confirmed", async () => {
    vi.mocked(bridge.preview).mockResolvedValue({
      id: "preview",
      added: 1,
      modified: 0,
      unchanged: 2,
      skipped: 0,
      deletions: ["old.txt"],
      scanned: 3,
    });
    const props = controls();
    props.health = { ...health, authenticated: true };
    render(
      <Knowledge
        {...props}
        sync={{ attempt: null, success: null }}
        reloadSync={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Preview changes" }));
    expect(await screen.findByText("old.txt")).toBeVisible();
    expect(screen.getByRole("checkbox")).not.toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: "Sync now" }));
    await waitFor(() =>
      expect(bridge.sync).toHaveBeenCalledWith(
        "preview",
        false,
        expect.any(Function),
      ),
    );
  });
});
describe("chat integration", () => {
  it("streams an authenticated backend response and saves desktop history", async () => {
    vi.mocked(bridge.chat).mockImplementation(
      async (
        _id,
        _model,
        _messages,
        _rag,
        update: (e: StreamEvent) => void,
      ) => {
        update({
          choices: [{ delta: { content: "A response from the mocked API" } }],
        });
      },
    );
    const report = vi.fn();
    render(
      <Chat
        config={config}
        models={[{ name: config.model, size: 3e9 }]}
        connected
        report={report}
      />,
    );
    await waitFor(() => expect(bridge.history).toHaveBeenCalled());
    fireEvent.change(screen.getByRole("textbox", { name: "Message" }), {
      target: { value: "A test question" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send message" }));
    expect(
      await screen.findByText("A response from the mocked API"),
    ).toBeVisible();
    await waitFor(() =>
      expect(bridge.saveHistory).toHaveBeenLastCalledWith([
        expect.objectContaining({
          messages: [
            { role: "user", content: "A test question" },
            { role: "assistant", content: "A response from the mocked API" },
          ],
        }),
      ]),
    );
  });
  it("disables sending when local services are unavailable", () => {
    render(
      <Chat config={config} models={[]} connected={false} report={vi.fn()} />,
    );
    expect(screen.getByRole("button", { name: "Send message" })).toBeDisabled();
  });
});

it("waits for a user-entered HERMES password and never unlocks on refresh", async () => {
  vi.mocked(bridge.config).mockResolvedValue({
    ...config,
    setup_complete: true,
  });
  vi.mocked(bridge.health).mockResolvedValue({
    ...health,
    credentials_unlocked: false,
  });
  render(<App />);
  const unlock = await screen.findByRole("button", {
    name: "Unlock HERMES",
  });
  expect(bridge.unlock).not.toHaveBeenCalled();
  fireEvent(window, new Event("focus"));
  await waitFor(() => expect(bridge.health).toHaveBeenCalledTimes(2));
  expect(bridge.unlock).not.toHaveBeenCalled();
  fireEvent.click(unlock);
  expect(bridge.unlock).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText("HERMES password"), {
    target: { value: "a known HERMES password" },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Unlock for this session" }),
  );
  await waitFor(() =>
    expect(bridge.unlock).toHaveBeenCalledWith("a known HERMES password"),
  );
});

it("creates an encrypted-vault password only after matching confirmation", async () => {
  const done = vi.fn(async () => {});
  render(
    <PasswordDialog
      status={{ exists: false, unlocked: false }}
      onClose={vi.fn()}
      onDone={done}
    />,
  );
  // Recovery must remain reachable if the old vault/container is missing but data remains.
  fireEvent.click(
    screen.getByRole("button", {
      name: "Recover an existing setup? Reset saved sign-ins",
    }),
  );
  expect(screen.getByRole("checkbox")).not.toBeChecked();
  expect(
    screen.getByRole("button", { name: "Save HERMES password" }),
  ).toBeDisabled();
  fireEvent.click(
    screen.getByRole("button", { name: "Back to password setup" }),
  );
  fireEvent.change(screen.getByLabelText("New HERMES password"), {
    target: { value: "my new known password" },
  });
  expect(
    screen.getByRole("button", { name: "Save HERMES password" }),
  ).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Confirm new HERMES password"), {
    target: { value: "my new known password" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Save HERMES password" }));
  await waitFor(() =>
    expect(bridge.createCredentials).toHaveBeenCalledWith(
      "my new known password",
      "my new known password",
      false,
    ),
  );
});
it("requires explicit consent before resetting saved sign-ins", () => {
  render(
    <PasswordDialog
      status={{ exists: true, unlocked: false }}
      onClose={vi.fn()}
      onDone={vi.fn()}
    />,
  );
  fireEvent.click(
    screen.getByRole("button", {
      name: "Forgot your HERMES password? Reset saved sign-ins",
    }),
  );
  fireEvent.change(screen.getByLabelText("New HERMES password"), {
    target: { value: "another known password" },
  });
  fireEvent.change(screen.getByLabelText("Confirm new HERMES password"), {
    target: { value: "another known password" },
  });
  expect(
    screen.getByRole("button", { name: "Save HERMES password" }),
  ).toBeDisabled();
  fireEvent.click(screen.getByRole("checkbox"));
  expect(
    screen.getByRole("button", { name: "Save HERMES password" }),
  ).toBeEnabled();
});
it("recovers only the selected local account after explicit confirmation", async () => {
  render(<AccountRecovery onDone={vi.fn(async () => {})} />);
  fireEvent.click(
    screen.getByRole("button", { name: "Forgot your local account password?" }),
  );
  await screen.findByRole("option", { name: "Owner · owner@example.test" });
  fireEvent.change(screen.getByLabelText("New local account password"), {
    target: { value: "my known account password" },
  });
  fireEvent.change(screen.getByLabelText("Confirm new password"), {
    target: { value: "my known account password" },
  });
  expect(
    screen.getByRole("button", { name: "Reset password and sign in" }),
  ).toBeDisabled();
  fireEvent.click(screen.getByRole("checkbox"));
  fireEvent.click(
    screen.getByRole("button", { name: "Reset password and sign in" }),
  );
  await waitFor(() =>
    expect(bridge.resetLocalAccount).toHaveBeenCalledWith(
      "owner@example.test",
      "my known account password",
      "my known account password",
      true,
    ),
  );
});
