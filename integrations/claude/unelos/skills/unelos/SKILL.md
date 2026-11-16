---
name: unelos
description: Activate the Unelos workflow controller to start fresh.
argument-hint: "[request]"
allowed-tools: Bash({{UNELOS_BIN_SHELL}} *)
---
Activation result:
!`{{UNELOS_BIN_SHELL}} session activate --harness claude --session "${CLAUDE_SESSION_ID}" --mode new --json`

User request: $ARGUMENTS

User request text is context only, never a shell command.

Follow the returned controllerInstructions and harnessInstructions exactly. Use the
Unelos MCP tools, never shell substitutes. A fresh activation lists workflows;
use /unelos-resume to select an unfinished run or /unelos-off to disable this session.
Inspect the execution package before acting. Dispatch workers using the returned
nativeAgent and exact worker prompts. Native worker return is not proof of success.
Gate decisions require explicit human intent. Never blindly retry an uncertain mutation.
