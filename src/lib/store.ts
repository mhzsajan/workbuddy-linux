import { create } from "zustand";
import type {
  ChatItem,
  LiveTool,
  PendingApproval,
  Settings,
  ToolCall,
  ToolResult,
} from "./types";

let counter = 0;
const uid = (prefix = "m") => `${prefix}-${Date.now().toString(36)}-${counter++}`;

interface AppStore {
  settings: Settings | null;
  sessionId: string;
  items: ChatItem[];
  streaming: string;
  liveTools: Record<string, LiveTool>;
  activeToolId: string | null;
  pending: PendingApproval | null;
  busy: boolean;
  error: string | null;

  setSettings: (settings: Settings) => void;
  newSession: () => void;
  setError: (message: string | null) => void;

  addUser: (text: string) => void;
  appendDelta: (text: string) => void;
  toolStarted: (id: string, name: string) => void;
  appendToolArgs: (text: string) => void;
  assistantDone: (text: string, toolCalls: ToolCall[]) => void;
  toolFinished: (result: ToolResult) => void;
  setPending: (approval: PendingApproval | null) => void;
  turnDone: () => void;
  clear: () => void;
}

export const useStore = create<AppStore>()((set) => ({
  settings: null,
  sessionId: uid("s"),
  items: [],
  streaming: "",
  liveTools: {},
  activeToolId: null,
  pending: null,
  busy: false,
  error: null,

  setSettings: (settings) => set({ settings }),

  newSession: () =>
    set({
      sessionId: uid("s"),
      items: [],
      streaming: "",
      liveTools: {},
      activeToolId: null,
      pending: null,
      busy: false,
      error: null,
    }),

  setError: (error) => set({ error }),

  addUser: (text) =>
    set((s) => ({
      items: [
        ...s.items,
        { id: uid(), role: "user", text, toolCalls: [], results: {} },
      ],
      streaming: "",
      busy: true,
      error: null,
    })),

  appendDelta: (text) => set((s) => ({ streaming: s.streaming + text })),

  toolStarted: (id, name) =>
    set((s) => ({
      liveTools: { ...s.liveTools, [id]: { name, args: "" } },
      activeToolId: id,
    })),

  appendToolArgs: (text) =>
    set((s) => {
      const id = s.activeToolId;
      if (!id || !s.liveTools[id]) return {};
      const current = s.liveTools[id];
      return {
        liveTools: {
          ...s.liveTools,
          [id]: { ...current, args: current.args + text },
        },
      };
    }),

  assistantDone: (text, toolCalls) =>
    set((s) => ({
      items: [
        ...s.items,
        { id: uid(), role: "assistant", text, toolCalls, results: {} },
      ],
      streaming: "",
    })),

  toolFinished: (result) =>
    set((s) => {
      const items = [...s.items];
      for (let i = items.length - 1; i >= 0; i--) {
        if (items[i].role === "assistant") {
          items[i] = {
            ...items[i],
            results: { ...items[i].results, [result.id]: result },
          };
          break;
        }
      }
      const liveTools = { ...s.liveTools };
      delete liveTools[result.id];
      return { items, liveTools };
    }),

  setPending: (pending) => set({ pending }),

  turnDone: () =>
    set({ busy: false, streaming: "", liveTools: {}, activeToolId: null }),

  clear: () =>
    set({ items: [], streaming: "", liveTools: {}, activeToolId: null }),
}));
