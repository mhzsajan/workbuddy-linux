import { invoke } from "@tauri-apps/api/core";
import type { HistoryMessage, Settings } from "./types";

/** True when running inside the Tauri shell rather than a plain browser tab. */
export const inTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const loadSettings = () => invoke<Settings>("load_settings");

export const saveSettings = (settings: Settings) =>
  invoke<void>("save_settings", { settings });

export const sendMessage = (sessionId: string, text: string) =>
  invoke<void>("send_message", { sessionId, text });

export const respondApproval = (id: string, approved: boolean) =>
  invoke<void>("respond_approval", { id, approved });

export const getHistory = (sessionId: string) =>
  invoke<HistoryMessage[]>("get_history", { sessionId });

export const resetSession = (sessionId: string) =>
  invoke<void>("reset_session", { sessionId });

export const mcpConnect = (name: string, command: string, args: string[]) =>
  invoke<string[]>("mcp_connect", { name, command, args });

export const mcpDisconnect = (name: string) =>
  invoke<void>("mcp_disconnect", { name });

export const mcpConnected = () => invoke<string[]>("mcp_connected");
