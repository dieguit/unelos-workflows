import { z } from "zod"
import { ALIAS_RE } from "./model-mappings-list"

export const mappingSchema = z.object({
  alias: z
    .string()
    .min(1, "Enter an alias.")
    .regex(
      ALIAS_RE,
      "Use lowercase letters and numbers, joined by “-”, “_” or “.”"
    ),
  models: z.record(
    z.string(),
    z.object({
      model: z.string().refine(
        (model) => !/[\s#]/u.test(model),
        "Model must not contain whitespace or #."
      ),
      effort: z.string().min(1).regex(/^[^\s#]+$/u, "Invalid effort.").optional(),
    })
  ),
})
export type MappingDraft = z.infer<typeof mappingSchema>
