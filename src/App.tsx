import { useCallback, useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useStore } from "./lib/store";
import {
  inTauri,
  loadSettings,
  mcpConnect,
  mcpConnected,
  mcpDisconnect,
  respondApproval,
  saveSettings,
  sendMessage,
} from "./lib/api";
import { demoSettings, runDemo } from "./lib/demo";
import type { Settings } from "./lib/types";
import { Sidebar } from "./components/Sidebar";
import { ChatPane } from "./components/ChatPane";
import { ApprovalDialog } from "./components/ApprovalDialog";
import { SettingsPanel } from "./components/SettingsPanel";
import { McpPanel } from "./components/McpPanel";

const EVENTS = [
  "kestrel://delta",
  "kestrel://tool_start",
  "kestrel://tool_args",
  "kestrel://assistant",
  "kestrel://tool_result",
  "kestrel://approval",
  "kestrel://turn_done",
  "kestrel://error",
];

export default function App() {
  const state = useStore();
  const tauri = inTauri();
  const [showSettings, setShowSettings] = useState(false);
  const [showMcp, setShowMcp] = useState(false);
  const [connected, setConnected] = useState<string[]>([]);

  // Single dispatcher shared by the real backend and the browser demo.
  const handleEvent = useCallback((event: string, payload: any) => {
    const s = useStore.getState();
    switch (event) {
      case "kestrel://delta":
        s.appendDelta(payload.text ?? "");
        break;
      case "kestrel://tool_start":
        s.toolStarted(payload.id, payload.name);
        break;
      case "kestrel://tool_args":
        s.appendToolArgs(payload.text ?? "");
        break;
      case "kestrel://assistant":
        s.assistantDone(payload.text ?? "", payload.toolCalls ?? []);
        break;
      case "kestrel://tool_result":
        s.toolFinished({
          id: payload.id,
          name: payload.name,
          ok: Boolean(payload.ok),
          content: payload.content ?? "",
          diff: payload.diff,
        });
        break;
      case "kestrel://approval":
        s.setPending({
          id: payload.id,
          tool: payload.tool,
          arguments: payload.arguments ?? "{}",
          mutating: Boolean(payload.mutating),
        });
        break;
      case "kestrel://turn_done":
        s.turnDone();
        break;
      case "kestrel://error":
        s.setError(payload.message ?? "unknown error");
        s.turnDone();
        break;
      default:
        break;
    }
  }, []);

  useEffect(() => {
    (async () => {
      if (!tauri) {
        useStore.getState().setSettings(demoSettings);
        return;
      }
      try {
        const loaded = await loadSettings();
        useStore.getState().setSettings(loaded);
        setConnected(await mcpConnected());
      } catch (err) {
        useStore.getState().setError(String(err));
      }
    })();
  }, [tauri]);

  useEffect(() => {
    if (!tauri) return;
    let cancelled = false;
    const unlisteners: UnlistenFn[] = [];

    (async () => {
      for (const event of EVENTS) {
        const un = await listen(event, (e) => handleEvent(event, e.payload));
        if (cancelled) {
          un();
          return;
        }
        unlisteners.push(un);
      }
    })();

    return () => {
      cancelled = true;
      unlisteners.forEach((un) => un());
    };
  }, [tauri, handleEvent]);

  const onSend = useCallback(
    async (text: string) => {
      const s = useStore.getState();
      if (s.busy) return;
      s.addUser(text);
      try {
        if (tauri) {
          await sendMessage(s.sessionId, text);
        } else {
          await runDemo(handleEvent, text, s.sessionId);
        }
      } catch (err) {
        useStore.getState().setError(String(err));
        useStore.getState().turnDone();
      }
    },
    [tauri, handleEvent],
  );

  const onRespond = useCallback(
    async (approved: boolean) => {
      const s = useStore.getState();
      const pending = s.pending;
      if (!pending) return;
      s.setPending(null);
      if (!tauri) return;
      try {
        await respondApproval(pending.id, approved);
      } catch (err) {
        s.setError(String(err));
      }
    },
    [tauri],
  );

  const onSaveSettings = async (next: Settings) => {
    if (tauri) await saveSettings(next);
    useStore.getState().setSettings(next);
  };

  const onConnectMcp = async (name: string, command: string, args: string[]) => {
    const discovered = await mcpConnect(name, command, args);
    setConnected(await mcpConnected());
    const refreshed = await loadSettings();
    useStore.getState().setSettings(refreshed);
    return discovered;
  };

  const onDisconnectMcp = async (name: string) => {
    await mcpDisconnect(name);
    setConnected(await mcpConnected());
  };

  return (
    <div className="app">
      <Sidebar
        settings={state.settings}
        onNewSession={state.newSession}
        onOpenSettings={() => setShowSettings(true)}
        onOpenMcp={() => setShowMcp(true)}
        mcpCount={connected.length}
        demoTauri={!tauri}
      />

      <ChatPane
        items={state.items}
        streaming={state.streaming}
        liveTools={state.liveTools}
        busy={state.busy}
        error={state.error}
        onSend={onSend}
      />

      {state.pending ? (
        <ApprovalDialog approval={state.pending} onRespond={onRespond} />
      ) : null}

      {showSettings && state.settings ? (
        <SettingsPanel
          settings={state.settings}
          onClose={() => setShowSettings(false)}
          onSave={onSaveSettings}
        />
      ) : null}

      {showMcp && state.settings ? (
        <McpPanel
          servers={state.settings.mcp_servers}
          connected={connected}
          onConnect={onConnectMcp}
          onDisconnect={onDisconnectMcp}
          onClose={() => setShowMcp(false)}
        />
      ) : null}
    </div>
  );
}
