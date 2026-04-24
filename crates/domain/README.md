# stadera-domain

Pure business logic for Stadera: types, validation, and calculations.
No I/O, no async, no dependencies on other Stadera crates.

## Key types

- `Weight`, `Height`, `BodyFatPercentage`, `LeanMass` — validated newtypes
- `Measurement` — single scale reading
- `UserProfile` — user static data
- `DailyTarget` — kcal + protein goals

## Key functions

- `tdee()` — Katch-McArdle BMR × activity multiplier
- `daily_target()` — kcal/protein goals for a given deficit
- `compute_trend()` — 7-day moving average and weekly delta
- `estimate_goal_date()` — projection to goal weight
