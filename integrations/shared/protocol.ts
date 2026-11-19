// CLI serialization: crates/core/src/execution.rs and work.rs; crates/cli/src/main.rs.
export type ExecutionIdentity =
  | {kind:'task'|'parallel';runId:string;nodeId:string;taskId:string}
  | {kind:'human-gate';runId:string;nodeId:string;activationId:string};

export interface ToolArguments {
  next: Record<string, never>;
  start: ({workflow:string;steps?:never}|{workflow?:never;steps:InlineStep[]}) & {name?:string};
  resume: {runId:string};
  complete: {expected:ExecutionIdentity;resultText?:string;summary?:string;outcome?:string};
  approve: {expected:ExecutionIdentity};
  reject: {expected:ExecutionIdentity;reason:string};
  status: Record<string, never>;
  inspect: {workflow:string;node?:string};
  rename: {name:string;runId:string};
}

export interface CliError {
  error: {code:string;message:string};
}

export interface CompletionControl {
  expected:ExecutionIdentity;
  summary:string|null;
  resultText:string|null;
  outcome:string|null;
  reason:string|null;
}

export interface WorkerPrompt {
  key:string;
  agent:string;
  prompt:string;
}

// Only fields the adapter reads; commands also return inspection/status objects.
export interface CliResult {
  runId?:string|null;
  packet?:{agent?:string|null}|null;
  workerPrompt?:string|null;
  workerPrompts?:WorkerPrompt[];
}
export type InlineStep =
  | {workflow:string;inputs?:Record<string,string>;stopOn?:string[];gate?:never}
  | {gate:string;workflow?:never;inputs?:never;stopOn?:never};
