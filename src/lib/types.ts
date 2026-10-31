export type ProviderKind = "openai" | "anthropic";
export type ApprovalMode = "always" | "writes" | "never";

export interface ProviderConfig {
  id: string;
  label: string;
  kind: ProviderKind;
  base_url: string;
  model: string;
  api_key: string;
}

export interface McpServerConfig {
  name: string;
  command: string;
  args: string[];
  enabled: boolean;
}

/** Mirrors the Rust `Settings` struct field-for-field (snake_case on the wire). */
export interface Settings {
  providers: ProviderConfig[];
  active_provider: string;
  workspace: string;
  system_prompt: string;
  max_iterations: number;
  approval_mode: ApprovalMode;
  mcp_servers: McpServerConfig[];
}

export interface ToolCall {
  id: string;
  name: string;
  arguments: string;
}

export interface DiffPayload {
  path: string;
  before: string;
  after: string;
}

export interface ToolResult {
  id: string;
  name: string;
  ok: boolean;
  content: string;
  diff?: DiffPayload;
}

export interface ChatItem {
  id: string;
  role: "user" | "assistant";
  text: string;
  toolCalls: ToolCall[];
  results: Record<string, ToolResult>;
}

export interface PendingApproval {
  id: string;
  tool: string;
  arguments: string;
  mutating: boolean;
}

export interface LiveTool {
  name: string;
  args: string;
}

export interface HistoryMessage {
  role: string;
  content: string;
  tool_calls: ToolCall[];
  tool_call_id: string | null;
}
