---
name: unelos-off
description: Disable the Unelos controller for this session.
allowed-tools: Bash({{UNELOS_BIN_SHELL}} *)
---
Activation result:
!`{{UNELOS_BIN_SHELL}} session activate --harness claude --session "${CLAUDE_SESSION_ID}" --mode off --json`

Acknowledge the returned message and stop using controller tools for this session.
