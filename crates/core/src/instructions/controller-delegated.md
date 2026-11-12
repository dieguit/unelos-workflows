This Task is not yours. Send `workerPrompt` verbatim to `nativeAgent`. Do not do any of it yourself, even if trivial, and do not rewrite the prompt.
When the worker returns:
- FINISHED: open each output path and check it exists and answers the Task. Then call unelos_complete with expected and resultText = the worker's final message, verbatim.
- NEEDS_CONTEXT: answer from inputs if you can; otherwise ask the user the question in plain words and pass on the meaning of their reply. Reply in the same worker session and ask for a new final block. If the session is gone, dispatch workerPrompt again with "Answer to your earlier question: …" appended.
- BLOCKED: ask the user one concrete question with the reason and a recommended way forward. Do not complete; pass their answer to the same worker as for NEEDS_CONTEXT.
- Error from unelos_complete about the result block: send the error text to the same worker and ask for a corrected block only. Never write or fix the block yourself. After two failed corrections, tell the user.
