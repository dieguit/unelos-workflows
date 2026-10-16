# Rust Architecture

Use a Cargo workspace with three crates:

```text
crates/
├── core/       # shared application/domain logic
├── cli/        # CLI-only code
└── ../src-tauri/ # Tauri backend
```

Dependency direction:

```text
CLI ──┐
      ├──> core
Tauri ┘
```

Rules:

- `core`: all logic shared by CLI and Tauri.
- `cli`: CLI parsing, prompts, terminal output, `--json`, exit codes.
- `src-tauri`: Tauri commands, state, events, windows, frontend bridge.
- CLI and Tauri must not depend on each other.
- Keep adapters thin; business logic belongs in `core`.
- Do not put shared logic under `src-tauri`.
- Do not put the CLI under `src-tauri/src/bin`.
- Prefer one `core` crate until splitting provides a clear benefit.

Test: deleting either the CLI or Tauri app should not require changes to `core` or the other adapter.

# Frontend Structure

Fractal, colocated layout. TanStack Router file-based routes in `src/routes`.

```text
src/
├── routes/
│   ├── __root.tsx
│   ├── -components/          # root-layout-only
│   └── <section>/
│       ├── route.tsx         # layout, only when the section has child routes
│       ├── index.tsx
│       ├── -components/      # used only by this section
│       ├── -hooks/
│       └── $param/           # same shape, one level down
├── components/               # shared across top-level routes / app shell
│   └── ui/                   # shadcn primitives
├── hooks/                    # shared hooks
└── lib/                      # shared non-React code
```

Rules:

- New code starts in the nearest route's `-components/` / `-hooks/` / `-lib/`.
- `-` prefix keeps files out of the route tree; never colocate without it.
- Used by a second branch → move it up to the nearest shared parent route folder.
- `src/components`, `src/hooks`, `src/lib` only for code used by 2+ top-level routes or the app shell.
- Never import from another route's `-` folders; move it up instead.
- No barrel `index.ts` files in route folders; import files directly.
- Do not edit `src/routeTree.gen.ts`; it is generated.
