import { z } from "zod"

export const taskSchema = z.object({
  id: z
    .string()
    .min(1, "Enter a Name (id).")
    .max(128, "Use at most 128 characters.")
    .regex(/^[A-Za-z0-9_-]*$/, "Use letters, numbers, underscores or hyphens."),
  description: z.string().nullable(),
  instructions: z.string(),
  skills: z.array(z.string()),
  outcomes: z
    .array(z.string().trim().min(1, "Enter an outcome name."))
    .refine((o) => new Set(o).size === o.length, "Outcome names must be unique."),
})
