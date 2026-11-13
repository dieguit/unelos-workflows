import { Link } from "@tanstack/react-router"

/** Skill name; `global/<id>` links to the Global Skills page. */
export function SkillLink({
  skill,
  className,
}: {
  skill: string
  className?: string
}) {
  if (!skill.startsWith("global/"))
    return (
      <span className={className} title={skill}>
        {skill}
      </span>
    )
  return (
    <Link
      to="/global-skills"
      search={{ skill: skill.slice(7) }}
      className={`${className ?? ""} hover:underline`}
      title="Open in Global Skills"
    >
      {skill}
    </Link>
  )
}
