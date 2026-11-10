import { z } from "zod"

export const skillSchema = z.object({
  name: z
    .string()
    .min(1, "Enter a Name.")
    .refine(
      (id) =>
        id.trim() === id && id !== "." && id !== ".." && !/[/\\:\0]/.test(id),
      "Use a name without surrounding whitespace, slashes, colons or null characters; not . or .."
    ),
  description: z.string(),
  body: z.string(),
})
