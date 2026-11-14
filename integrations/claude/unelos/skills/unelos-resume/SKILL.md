---
name: unelos-resume
description: Resume an unfinished Unelos workflow run.
allowed-tools: Bash({{UNELOS_BIN_SHELL}} *)
---
Activation result:
!`{{UNELOS_BIN_SHELL}} session activate --harness claude --session "${CLAUDE_SESSION_ID}" --mode resume --json`

Follow the returned controllerInstructions and harnessInstructions exactly.
Select an unfinished run explicitly. Use the Unelos MCP tools, never shell substitutes.
Gate decisions require explicit human intent. Never blindly retry an uncertain mutation.
