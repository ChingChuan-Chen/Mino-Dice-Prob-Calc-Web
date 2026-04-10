# Mino Dice Probability Calculator - Copilot Instructions

## Project Context
- This is a Rust + WASM frontend project built with Trunk.
- Keep changes minimal, focused, and aligned with existing module boundaries in `src/`.

## Build and Verification
- Run `cargo test --lib` after code changes.
- Run `cargo fmt --all -- --check` for formatting checks.
- Run `cargo clippy --target wasm32-unknown-unknown -- -D warnings` when the wasm target is available.
- Run `trunk build --release` when Trunk is installed.

## Coding Expectations
- Prefer small, incremental changes over large refactors.
- Do not add dependencies unless necessary.
- Preserve existing behavior unless the task explicitly requires behavior changes.
- Avoid introducing unrelated formatting churn.

## Boundaries
- Always: validate changes with relevant existing checks.
- Ask first: broad architecture shifts, dependency additions, or CI workflow changes.
- Never: commit secrets, remove tests to make builds pass, or bypass verification.

## Skills
- Use skills from `.github/skills/` when relevant, starting with:
  - `test-driven-development`
- Source: https://github.com/addyosmani/agent-skills
