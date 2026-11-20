import { z } from "zod"
import type { WorkflowStep } from "@/lib/projects"
import { humanize, slugify } from "@/lib/step-edit"

const route = z.object({ outcome: z.string().nullable(), target: z.string() })

const step = z
  .object({
    id: z.string(),
    kind: z.enum(["task", "parallel", "human-gate", "workflow", "end"]),
    agent: z.string().nullable(),
    execution: z.enum(["main", "subagent"]).nullable(),
    routes: z.array(route),
    task: z.object({ source: z.enum(["inline", "named"]) }).nullable(),
    workers: z.array(z.object({
      task: z.object({ source: z.enum(["inline", "named"]), description: z.string().nullable() }),
      agent: z.string(),
    })),
  })
  .loose()
  .superRefine((s, ctx) => {
    if (!s.id) ctx.addIssue({ code: "custom", path: ["id"], message: "Enter a name." })
    if (s.kind === "parallel") {
      if (!s.workers.length)
        ctx.addIssue({ code: "custom", path: ["workers"], message: "Add at least one task." })
      const names = new Set<string>()
      s.workers.forEach((worker, i) => {
        if (!worker.agent.trim())
          ctx.addIssue({ code: "custom", path: ["workers", i, "agent"], message: "Choose an Agent." })
        if (worker.task.source !== "inline") return
        const name = slugify(worker.task.description ?? "")
        if (!name)
          ctx.addIssue({ code: "custom", path: ["workers", i, "task", "description"], message: "Enter a task name." })
        else if (names.has(name))
          ctx.addIssue({ code: "custom", path: ["workers", i, "task", "description"], message: `Names must be unique within ${humanize(s.id)}.` })
        names.add(name)
      })
    }
    if (s.kind !== "task" && s.kind !== "human-gate") return
    if (s.kind === "task") {
      if (!s.agent) ctx.addIssue({ code: "custom", path: ["agent"], message: "Choose an agent." })
      if (!s.execution)
        ctx.addIssue({ code: "custom", path: ["execution"], message: "Choose where it runs." })
    }
    const seen = new Set<string>()
    s.routes.forEach((r, i) => {
      if (r.outcome !== null && s.task?.source !== "named" && s.kind === "task") {
        if (!r.outcome.trim())
          ctx.addIssue({ code: "custom", path: ["routes", i, "outcome"], message: "Enter an outcome name." })
        else if (seen.has(r.outcome))
          ctx.addIssue({ code: "custom", path: ["routes", i, "outcome"], message: "Outcome names must be unique." })
        seen.add(r.outcome)
      }
      if (!r.target)
        ctx.addIssue({
          code: "custom",
          path: ["routes", i, "target"],
          message: r.outcome ? `Choose a destination for ${r.outcome}.` : "Choose a next step.",
        })
    })
  }) as unknown as z.ZodType<WorkflowStep, WorkflowStep> // only the fields above are checked; the rest pass through

export const workflowSettingsSchema = (originalName: string) =>
  z
    .object({
      name: z
        .string()
        .min(1, "Enter a name.")
        .refine(
          (name) =>
            name === originalName ||
            (name.length <= 128 && /^[A-Za-z0-9_-]+$/.test(name)),
          "Use up to 128 letters, numbers, underscores or hyphens."
        ),
      description: z.string().nullable(),
      start: z.string().min(1, "Choose a start step."),
      inputs: z.array(z.string()),
      steps: z.array(step),
    })
    .superRefine((w, ctx) => {
      const seen = new Set<string>()
      w.steps.forEach((s, i) => {
        if (s.id && seen.has(s.id))
          ctx.addIssue({ code: "custom", path: ["steps", i, "id"], message: "Step names must be unique." })
        seen.add(s.id)
      })
    })
