# PR #2 Review — `feat(domain): introduce domain crate`

**URL**: https://github.com/MicheleBellitti/Stadera/pull/2
**Branch**: `feature/domain-logic`
**HEAD SHA**: `125fad8001c0b7f5b2b2dfee1228d69c0c5f11d5`
**Review date**: 2026-04-24
**Status**: ⏳ blockers open — not mergeable yet

## Build signal

- `cargo fmt --all -- --check` → ✅
- `cargo clippy --all-targets --all-features -- -D warnings` → ✅
- `cargo test --all` → ✅ 45 tests pass
- `cargo doc --no-deps` → ✅

## Blockers (3)

### B1 — `#[derive(Deserialize)]` bypasses newtype validation

**Location**: `crates/domain/src/units.rs:8,41,74,107`

Tuple-struct `pub struct Weight(f64)` with `#[derive(Deserialize)]` deserializes any `f64` directly, skipping `Weight::new()`. Same for `BodyFatPercent`, `LeanMass`, `Height`.

Breaks: `.claude/CLAUDE.md` invariant "No raw f64 for domain values".

Reproduction: `serde_json::from_str::<Weight>("-10.0")` returns `Ok(Weight(-10.0))` instead of `Err`.

**Fix** (per type):
```rust
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64")]
pub struct Weight(f64);
```
No `into = "f64"` needed — default `Serialize` for single-field tuple structs is already transparent.
No `impl From<Weight> for f64` needed.
`TryFrom<f64>` already exists on all 4 types.
`DomainError: Display` already satisfied via `thiserror::Error` derive.

Regression test (1 per type):
```rust
#[test]
fn weight_deserialize_rejects_invalid() {
    assert!(serde_json::from_str::<Weight>("-10.0").is_err());
    assert!(serde_json::from_str::<Weight>("600.0").is_err());
    assert!(serde_json::from_str::<Weight>("null").is_err());
}
```
Requires `serde_json` in `[dev-dependencies]`.

### B2 — `README:md` filename has `:` instead of `.`

**Location**: `crates/domain/README:md`

Typo. No tool in the Rust ecosystem (cargo, crates.io) or git host (GitHub, GitLab) recognizes this as a README. Crate documentation is effectively invisible.

**Fix**: `git mv "crates/domain/README:md" "crates/domain/README.md"`

### B3 — `.vscode/settings.json` committed, `.gitignore` too minimal

**Location**: `.vscode/settings.json`, `.gitignore`

`.gitignore` contains only `/target`. `.vscode/settings.json` contains personal `autoApprove` config — IDE noise.

**Fix**:
1. Extend `.gitignore`:
   ```
   /target
   /.vscode/
   /.idea/
   .DS_Store
   *.swp
   *.swo
   ```
2. `git rm --cached .vscode/settings.json`

## Concerns (12) — design decisions, owner choice

Grouped by theme.

### Energy / daily target

- **C1** — `DailyTarget.kcal` can be negative with no validation. `crates/domain/src/energy.rs:18-28`. Options: fail-fast with `Result<_, DomainError>` + `UnsafeCalorieTarget` variant (recommended); clamp to 1500 (adult male min); validate `Deficit` newtype upstream. Floor 100 rejected — no medical meaning.
- **C4** — `ActivityLevel` missing `ExtraActive` (1.9). `crates/domain/src/user.rs:13-18`. 4 canonical levels out of 5. Intentional MVP gap or omission? Document in doc-comment either way.

### No-raw-f64 convention violations

- **C2** — Derived values return raw `f64`, breaking the repo-wide "newtypes for domain values" rule:
  - `energy.rs:4` — `bmr_katch_mcardle() -> f64`
  - `energy.rs:8` — `tdee() -> f64`
  - `energy.rs:13-16` — `DailyTarget { kcal: f64, protein_g: f64 }`
  - `measurement.rs:29-32` — `fat_mass() -> Option<f64>`
  - `measurement.rs:34-37` — `bmi() -> f64`

  Decision: introduce newtypes (`Kcal`, `ProteinGrams`, `FatMass`, `Bmi`) OR explicitly document exception for derived (non-input) values.

### Errors

- **C3** — `DomainError::NotFinite` lacks field context. `crates/domain/src/error.rs:17-18`. Other variants carry `value`; this one doesn't. Fix: `NotFinite { field: &'static str }` or per-type variants.

### Trend

- **C5** — `f64::EPSILON` (~2.2e-16) as stagnation threshold in `estimate_goal_date`. `crates/domain/src/trend.rs:85`. Should be ~0.01 kg/week. Extract `STAGNANT_TREND_THRESHOLD_KG_PER_WEEK` constant.

### Conventions / tooling

- **C6** — `proptest = "1"` hardcoded in `crates/domain/Cargo.toml:14` instead of `[workspace.dependencies]`. Move now, M3/M4 crates will want property tests too.
- **C7** — `release-please-config.json` does not include `crates/domain`. After merge, no CHANGELOG entry will be generated for domain. Add entry + manifest `"crates/domain": "0.1.0"`.
- **C8** — `rust-toolchain.toml` pinned to 1.85 but CI uses `dtolnay/rust-toolchain@stable`. Choose one: pin everywhere (`@1.85`) or remove toolchain file.

### Test coverage

- **C9** — Gaps:
  - `test_energy.rs` covers only `Sedentary` and `VeryActive` levels; missing `LightlyActive` and `ModeratelyActive` through full `tdee()`.
  - `test_trend.rs` missing unordered input, same-day duplicates, `estimate_goal_date` gaining scenario (positive delta).
  - `test_measurement.rs` uses `Utc::now()` — should use fixed timestamps like `test_trend.rs` does.
  - No serde roundtrip tests anywhere (becomes important after B1 fix).

### API shape

- **C10** — `Measurement` and `UserProfile` have `pub` fields. Consumers can construct bypassing `new()`. OK now (passthrough), but future cross-field validation will be bypassed. Fix: `pub(crate)` + getters, OR document as transparent DTOs.
- **C11** — `Sex` stored but unused by Katch-McArdle BMR. `crates/domain/src/user.rs:6-10`. If kept for future Mifflin-St Jeor fallback, document.
- **C12** — `UserProfile::age()` with future birth_date → 0 (via `max(0) as u32`). Test doesn't pin this. Add regression test or return `Option<u32>`.

## Nits (9) — cosmetic

- **N1** — Remove `// WIP: …` comment in `crates/domain/src/error.rs:19`. Convert to issue if still relevant.
- **N2** — `Clone` on `DomainError` unusual. Keep `PartialEq` for tests, drop `Clone`.
- **N3** — `ActivityLevel::multiplier(&self)` → `multiplier(self)` (type is `Copy`). Clippy `trivially_copy_pass_by_ref`.
- **N4** — Magic number `0.01` in `crates/domain/src/trend.rs:82`. Extract `GOAL_TOLERANCE_KG` constant.
- **N5** — Field name `moving_average_7d` misleading (it's "average over last 7 days", not rolling window). Rename or document.
- **N6** — 4 identical newtype blocks in `units.rs` → candidate for `macro_rules!`. Opinion, not blocker.
- **N7** — `TryFrom<f64>` is pure delegate to `new()`. API duplication but idiomatic.
- **N8** — `(w.value() - v).abs() < f64::EPSILON` in tests is theatrical (constructor doesn't do arithmetic). Use `assert_eq!`.
- **N9** — `rustfmt.toml` `edition = "2021"` while workspace is `edition = "2024"`. Align.

## Global repo gaps (6)

Out of scope for this PR, but worth tracking:

- **G1** — No `./CLAUDE.md` in root (only `.claude/CLAUDE.md`).
- **G2** — CI missing `cargo audit` / `cargo deny`. Add workflow on schedule + on lockfile changes.
- **G3** — CI `cargo test` without `--locked`. With `Cargo.lock` committed, add `--locked`.
- **G4** — No MSRV matrix in CI (only `stable`). Add `1.85` job to protect edition-2024 floor.
- **G5** — No PR template → addressed in this branch concurrently.
- **G6** — `clippy.toml` exists but empty (0 bytes). Populate (`msrv = "1.85"`, `disallowed-methods` for `Utc::now` in domain) or remove.

## Proposed follow-up structure

1. **This branch, before merge**: fix B1 + B2 + B3. Minimal scope to unblock.
2. **Follow-up PR A (immediately post-merge of M2)**: C1 (DailyTarget), C2 (derived newtypes), C3 (NotFinite context), C9 (test coverage).
3. **Follow-up PR B (pre-M3 storage)**: C6 (proptest workspace), C7 (release-please), C8 (toolchain pin), C12 (age edge).
4. **M7 cloud milestone**: G2, G3, G4 (CI hardening).
5. **Opportunistic**: nits + G1, G6.

## Key architectural positives (for record)

- `domain` crate is correctly pure: grep confirms no `Utc::now()` / `SystemTime::now()` in `crates/domain/src/**`.
- `Eq` / `Hash` correctly absent on all `f64`-containing types (per convention).
- Katch-McArdle formula (`BMR = 370 + 21.6 * lean_mass_kg`) and TDEE multiplication are mathematically correct.
- No `unwrap()` / `panic!()` / `expect()` in non-test code (grep confirmed).
- Tests in dedicated files under `tests/` — good separation from source.
- Property tests with `proptest` present (shallow but a start).
