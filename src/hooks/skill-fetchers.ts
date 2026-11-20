import {
  listGlobalSkills,
  listSkills,
  type SkillSummary,
} from "@/lib/projects"

export async function listGlobalPrefixedSkills(): Promise<SkillSummary[]> {
  return (await listGlobalSkills()).map((s) => ({
    ...s,
    name: `global/${s.name}`,
  }))
}

export async function listAllSkills(root: string): Promise<SkillSummary[]> {
  const [project, global] = await Promise.all([
    listSkills(root),
    listGlobalPrefixedSkills(),
  ])
  return [...project, ...global]
}

/** Project + global skills, or only global ones when `root` is null. */
export const listSkillChoices = (root: string | null) =>
  root === null ? listGlobalPrefixedSkills() : listAllSkills(root)
