import type { Command, Capability, RuntimeEvent } from "./generated/contracts";
export type ExecutionContext = Readonly<{
  invoke(command: Command, timeoutMs?: number): unknown;
  log(message: string): void;
  emit(event: RuntimeEvent): void;
  signal: Readonly<{ aborted: boolean }>;
  deadlineMs: number;
}>;
export type Action = Readonly<{
  id: string;
  capabilities: readonly Capability[];
  execute(args: Record<string, unknown>, context: ExecutionContext): unknown;
}>;
export type Plugin = Readonly<{
  onLoad?(context: ExecutionContext): unknown;
  onUnload?(context: ExecutionContext): unknown;
  invoke_action(
    action: string,
    args: Record<string, unknown>,
    context: ExecutionContext,
  ): unknown;
}>;
