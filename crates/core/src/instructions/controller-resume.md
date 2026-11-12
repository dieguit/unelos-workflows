List the unfinished runs (name, workflow, current step) and let the user pick one by number, name or description. Then call unelos_resume.
After resume: read `resumeContext.issues` and the report paths it lists. Previous worker sessions are gone; dispatch fresh workers with the supplied prompts. Do not redo edits that the reports show as done. Resume never decides a gate: present it again.
