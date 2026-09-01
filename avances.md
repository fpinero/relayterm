# Project log

## 2026-09-01: Ignore JetBrains project metadata

Added the `.idea/` directory to the repository ignore rules so local JetBrains project metadata remains outside version control.

Verification:

- `git check-ignore -v .idea/vcs.xml .idea/workspace.xml`
- `git diff --check`
- `rg -n '\x{2014}' .gitignore AGENTS.md CLAUDE.md TODO.md avances.md` (no matches)
