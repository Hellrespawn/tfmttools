# Rust features after 1.86

Checked 2026-10-08 against official release notes through Rust 1.99.0
(released 2026-10-01). The workspace already declares Rust 1.89.0 and
edition 2024 in [Cargo.toml](../../Cargo.toml). The benefit assessments
below are based on inspecting this repository, rather than release-note
claims about this project.

## Language features

- **Let chains, 1.88:** combine pattern matches and boolean conditions in
  `if`/`while`. Requires edition 2024. Already used in
  [reserved-name validation](../../crates/core/src/action/validation/forbidden.rs#L83),
  [template resolution](../../crates/tfmt/src/commands/rename/template_resolution.rs#L51),
  and [rename progress](../../crates/tfmt/src/commands/rename/apply.rs#L72).
  This is the clearest useful language addition, and needs no MSRV bump.
  [Official announcement](https://blog.rust-lang.org/2025/06/26/Rust-1.88.0/).
- **`if let` guards in match arms, 1.95:** permit extracting a value in a
  match guard and using its bindings in the arm. Potentially useful when
  parser or action handling needs a second pattern match. Current code
  has no compelling replacement that warrants raising the MSRV solely
  for this syntax. Guards do not contribute to exhaustiveness checking.
  [Official announcement](https://blog.rust-lang.org/2026/04/16/Rust-1.95.0/).
- **Precise capture bounds in traits, 1.87:** `use<...>` on return-position
  `impl Trait` in trait methods offers more explicit lifetime capture.
  Our [path extension trait](../../crates/core/src/util.rs#L27) does not
  return opaque types, so this is useful mainly if future interfaces do.
  [Official release notes](https://doc.rust-lang.org/releases.html#version-1870-2025-05-15).
- **Inferred const generic arguments, 1.89:** `_` can stand for a const
  argument inferred from context. No significant const-generic API in
  the inspected crates makes this an immediate opportunity.
  [Official announcement](https://blog.rust-lang.org/2025/08/07/Rust-1.89.0/).

The later releases also add low-level assembly, FFI and macro capabilities,
including C-variadic function definitions in 1.99. Those do not fit the
current application code. [Complete release notes](https://doc.rust-lang.org/releases.html).

## Standard library additions

These are useful APIs, rather than new Rust language syntax.

| Addition | Stable | Repository opportunity |
| --- | --- | --- |
| `File::lock`, `try_lock`, shared locking and unlocking | 1.89 | Building blocks for protecting history against simultaneous CLI processes. [History loading](../../crates/history/src/history.rs#L39) and [saving](../../crates/history/src/history.rs#L64) currently use separate reads/writes. A correct design must cover the entire load/change/save lifecycle; locking only the write would still allow stale history to overwrite newer records. |
| `str::floor_char_boundary` | 1.91 | Direct replacement for the clamping/backtracking loop in [diagnostic line/column conversion](../../crates/picotmpl/src/diagnostic.rs#L32): `let start = source.floor_char_boundary(self.span.start);`. Small, clear cleanup, requiring an MSRV bump. |
| `assert_matches!` | 1.96 | Replace existing [validation pattern assertions](../../crates/core/src/action/validation.rs#L94) and similar filesystem tests, providing the unexpected value in failure diagnostics. Small testing improvement, requiring an MSRV bump. |
| `cfg_select!` | 1.95 | Compile-time selection among configuration branches. Existing platform-specific blocks are short, so no strong immediate benefit. |

Sources: [1.89 locking announcement](https://blog.rust-lang.org/2025/08/07/Rust-1.89.0/),
[UTF-8 boundary API](https://doc.rust-lang.org/std/primitive.str.html#method.floor_char_boundary),
[1.96 stabilized APIs](https://doc.rust-lang.org/releases.html#version-1960-2026-05-28),
[1.95 configuration macro](https://blog.rust-lang.org/2026/04/16/Rust-1.95.0/).

Recommendation: continue using let chains where they simplify conditions.
History locking could justify substantive work without increasing the
current MSRV. Boundary rounding and assertion diagnostics are convenient
extras if there is a separate reason to raise the MSRV; these findings
alone do not establish a strong reason to do so.
