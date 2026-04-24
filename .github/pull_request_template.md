<!--
  PR Title: use Conventional Commits format.
  Examples: feat(domain): add BMI newtype  |  fix(api): reject invalid weight  |  chore: bump serde
  Types: feat, fix, refactor, chore, docs, test, ci, perf, build, style
-->

## Summary

<!-- One or two sentences: what changes and why. -->

## Motivation / Context

<!--
  Why is this change needed? Link to issue, milestone (M1–M7), or discussion.
  Delete the line that doesn't apply.
-->

- Milestone: M?
- Closes #
- Related: #

## Type of change

<!-- Tick one or more. Must match the Conventional Commits prefix in the title. -->

- [ ] `feat` — new feature
- [ ] `fix` — bug fix
- [ ] `refactor` — code restructuring, no behavior change
- [ ] `chore` — maintenance (deps, configs, tooling)
- [ ] `docs` — documentation only
- [ ] `test` — tests only
- [ ] `ci` — CI / CD
- [ ] `perf` — performance
- [ ] `build` — build system
- [ ] `style` — formatting only

## Changes

<!-- Bullet list of the main changes the reviewer should look at. -->

-

<details>
<summary><b>Screenshots / Demo</b> — optional, for UI or visual changes</summary>

<!-- Before / after screenshots, GIFs, terminal recordings, sample API responses, etc. -->

</details>

<details>
<summary><b>Breaking changes</b> — optional, API / schema / config breaks</summary>

<!--
  - What breaks?
  - Who is affected (api consumers, cron jobs, CLI users)?
  - How to migrate?
-->

</details>

<details>
<summary><b>Migration notes</b> — optional, DB migrations / data backfill</summary>

<!--
  - Order of operations
  - Downtime window (if any)
  - Rollback plan
-->

</details>

## Test plan

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all`
- [ ] Manual verification: <!-- describe or delete -->

## Checklist

- [ ] Title follows [Conventional Commits](https://www.conventionalcommits.org/)
- [ ] No `unwrap()`, `panic!()`, `expect()` in non-test code
- [ ] No `Utc::now()` / `SystemTime::now()` inside `crates/domain/`
- [ ] Domain values use newtypes, not raw `f64` / `i64`
- [ ] New deps centralized in `[workspace.dependencies]` and inherited via `dep.workspace = true`
- [ ] CHANGELOG handled by release-please (no manual edit)
- [ ] Rustdoc / README / CLAUDE.md updated where relevant

<details>
<summary><b>Deployment notes</b> — optional, infra / secrets / cron</summary>

<!--
  - Terraform changes
  - New secrets required in Secret Manager
  - Cloud Run job schedule changes
  - Environment variables added / removed
-->

</details>

<details>
<summary><b>Observability</b> — optional, logging / metrics / tracing</summary>

<!--
  - New log fields, spans, metrics
  - Dashboards / alerts to update
-->

</details>

<details>
<summary><b>Follow-ups</b> — optional, known gaps left for future PRs</summary>

<!--
  Issues to open or carry-over work. Link them when created.
-->

</details>

## Notes for reviewer

<!-- Anything specific to focus on. Delete this section if nothing to add. -->
