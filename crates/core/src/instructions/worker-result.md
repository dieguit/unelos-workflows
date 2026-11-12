## Rules
- Read every input, the feedback and existing outputs before acting.
- If something essential is missing, return NEEDS_CONTEXT. Never guess requirements.
- Do not edit .unelos state or the run ledger, talk to the user, decide gates, or choose the next step.
- A FINISHED result without every output written is rejected.

## Final message (strict)
End with exactly one block. Copy it and change only the values in <>:
<unelos-worker-result>
{"taskId":{taskId},"status":"FINISHED","outcome":{outcome},"summary":"<one paragraph>","changedPaths":["<path>"],"verification":["<what you checked>"]}
</unelos-worker-result>
Need an answer to continue: "status":"NEEDS_CONTEXT","outcome":null,"question":"<one question>"
Cannot proceed: "status":"BLOCKED","outcome":null,"reason":"<why>"
Valid JSON only. No other keys. Do not write the block anywhere else.
