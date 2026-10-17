import { useEffect, type RefObject } from "react"
import { SearchIcon } from "lucide-react"
import { Input } from "@/components/ui/input"

export function countLabel(
  shown: number,
  total: number,
  noun: string,
  filtering: boolean
) {
  const n = total === 1 ? noun : `${noun}s`
  return filtering ? `${shown} of ${total} ${n}` : `${total} ${n}`
}

const isEditable = (el: EventTarget | null) =>
  el instanceof HTMLElement &&
  (el.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName))

export function ListSearch({
  noun,
  value,
  onChange,
  shown,
  total,
  inputRef,
}: {
  noun: string
  value: string
  onChange: (value: string) => void
  shown: number
  total: number
  inputRef: RefObject<HTMLInputElement | null>
}) {
  const label = `Search ${noun}s`

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "/" && !isEditable(e.target)) {
        e.preventDefault()
        inputRef.current?.focus()
      }
    }
    document.addEventListener("keydown", onKey)
    return () => document.removeEventListener("keydown", onKey)
  }, [inputRef])

  return (
    <div className="mt-1 flex items-center gap-3">
      <div className="relative w-70">
        <SearchIcon
          aria-hidden="true"
          className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
        />
        <Input
          ref={inputRef}
          type="search"
          aria-label={label}
          placeholder={label}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key !== "Escape") return
            if (value) onChange("")
            else e.currentTarget.blur()
          }}
          className="rounded-md bg-background pr-8 pl-8 md:text-base [&::-webkit-search-cancel-button]:hidden"
        />
        <kbd
          aria-hidden="true"
          className="absolute top-1/2 right-2 -translate-y-1/2 rounded-sm border border-border-strong px-1.25 py-px font-mono text-xs text-muted-foreground"
        >
          /
        </kbd>
      </div>
      <span
        data-list-count
        aria-live="polite"
        className="text-sm text-muted-foreground"
      >
        {countLabel(shown, total, noun, value.trim() !== "")}
      </span>
    </div>
  )
}
