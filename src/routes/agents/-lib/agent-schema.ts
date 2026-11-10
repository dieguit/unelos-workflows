import { z } from "zod"

export const agentSchema = z.object({
  id: z
    .string()
    .min(1, "Enter a Name (id).")
    .max(128, "Use at most 128 characters.")
    .regex(/^[A-Za-z0-9_-]*$/, "Use letters, numbers, underscores or hyphens."),
  instructions: z.string(),
  skills: z.array(z.string()),
  mcps: z.array(z.string()),
  tools: z.array(z.string()),
  model: z
    .union([z.string(), z.object({ harness: z.string(), model: z.string() })])
    .nullable(),
})
