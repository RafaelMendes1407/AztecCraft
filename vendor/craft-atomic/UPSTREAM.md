# craft-atomic provenance

Source: [storytold/craft-libs](https://github.com/storytold/craft-libs), public
upstream base `457a078809690994b22debdffb703ff36d3129df`. Only the atomic crate's
five files are included, with the complete upstream `LICENSE-MIT`,
`LICENSE-APACHE`, `NOTICE` and `ATTRIBUTION.md`. The latter two describe the whole
workspace; their CMS asset entries do not mean those assets are bundled here.
No CMS data, artwork or other upstream crates are included. The exact two-line
upstream `rustfmt.toml` is retained as a sixth support file so root and standalone
Cargo formatting discover the same settings without changing reviewed source.
The trailing-whitespace hook excludes only this upstream README and the exact
patch, whose whitespace is part of their recorded byte identity. Other hooks
remain active for both artifacts and all vendored files.

The library retains its `MIT OR Apache-2.0` license expression, both complete
license texts and original notices. Packaging includes all four license/notice
files unchanged with each platform binary. No trademark endorsement or upstream
acceptance of local changes is inferred. AztecCraft's own license (MIT OR Apache-2.0) is in the root
`LICENSE-MIT`, `LICENSE-APACHE` and `NOTICE`.

The reviewed local writable-preflight hook has provenance commit
`8fb530c18dcbe617d5ba81fd2df21216a863c89b`, tree
`820a982448001dbb8f8a01db69b1412a50ca3f66`. Four unit-test Result-handling lint
fixes followed in local commit `c39a5ce6d636685a96d226e34a03250273931865`, tree
`182c1435899c276898145de9d517d0beb9e68a8f`. These local IDs record history;
public reconstruction does not require unpublished commit objects.

`UPSTREAM.patch` is the complete base-to-reviewed-source delta, affecting only
`crates/atomic/src/lib.rs`, with SHA256
`48d22693d53c2a74db52164019840af18662d30ea5677d6cc9b78253000467ee`.
To reconstruct, obtain the public upstream base, verify this patch hash and run
`git apply /path/to/UPSTREAM.patch` from that upstream repository root. Copy the
five crate files and four root notice files. No private patch/path is required.

The only subsequent source change is this exact two-line prefix to `src/lib.rs`:

```rust
// Modified for AztecCraft: injectable writable preflight and test Result handling.
// The complete source delta and provenance are in UPSTREAM.patch and UPSTREAM.md.
```

`Cargo.toml` adds a modification notice and expands the five inherited package
fields to their exact upstream values and workspace lints to
`unsafe_code = "forbid"`, `dbg_macro = "warn"`, `todo = "warn"`. Its description
and Apple-only `rustix` 1.1 dependency retain disabled default features and
`std,fs`. There is no workspace, build script or further runtime/test change.

SHA256 inventory, where unchanged means identical reviewed/distributed bytes:

| File | Reviewed upstream | Distributed |
| --- | --- | --- |
| `Cargo.toml` | `132c06636b8592118aa304f07c3dec0aaf65dd604c68f6d8f76b9561dbc7e1fe` | `f224cd5ef974ef0014b9c31289f4e6ee2ff20d23c58e0ca8444c23732327a879` |
| `README.md` | `d64e0b37e3c57166669ea0293009a3bf6f97d148bd5230bd329102e3f20cbca5` | unchanged |
| `src/lib.rs` | `bab26dda490329ec352c88951b14390ec5c99d2d4e585d7b24ec74f19a4f3daa` | `45c373dc34e0632af37b30f3ae9b411edcc61c560d7269a1e7ef84e3fd3c635a` |
| `tests/atomic_tests.rs` | `883a61d02af5d5f358f40158d0563babf0b95d1b9c5d471f85167316c9938ed6` | unchanged |
| `tests/permissions.rs` | `71cfcf79669b3acba2bdac6292ad0aa86437fc2be89fcc85cfdf333f134743fc` | unchanged |
| `LICENSE-MIT` | `2f42c0cafc8ad394d91373fe1f342aee5453850b151a51405c68cae9ada86d26` | unchanged |
| `LICENSE-APACHE` | `0d542e0c8804e39aa7f37eb00da5a762149dc682d7829451287e11b938e94594` | unchanged |
| `NOTICE` | `016d721855c84d18096c592cc1f8f4674ab8dabbf18ed1f817c76eefed26df8b` | unchanged |
| `ATTRIBUTION.md` | `ba16e1878f96bb08ed5512217fd2a66019ad69cb214fb1250bb3f3a64e444634` | unchanged |
| `rustfmt.toml` | `1b51e53edb3571229e84540b9b08514fb23d8c8d3225e8b984773f5c36eaa906` | unchanged |

The standalone test lock derives from the root lock at AztecCraft parent
`dd20981bafdbda12e52d593a6944a9954531341e`, root-lock SHA256
`ea3fbfc272e543287fa10df9716cfa811ad3bffbce496a94feb6d360e892888b`.
The production lock only removes the craft-atomic Git source for its path
identity. Its registry resolution is unchanged. The standalone lock contains
exactly seven needed root registry tuples: `bitflags 2.13.2`, `errno 0.3.14`,
`libc 0.2.190`, `linux-raw-sys 0.12.1`, `rustix 1.1.5`, `windows-link 0.2.1`,
`windows-sys 0.61.2`, with identical versions, sources and checksums and zero
extra tuples. Apple-target dependencies remain locked on non-Apple test hosts.

Root tests do not run dependency tests. The Linux/macOS/Windows CI matrix runs
standalone format, strict Clippy, unit/integration tests and docs with this lock.
Vendor formatting uses upstream settings
`max_width=150,use_small_heuristics=Max` to preserve reviewed bytes. The retained
README describes upstream guarantees; its import does not prove physical power
loss durability, every platform or native Maya Save support.

Updates compare the new public revision and security changes, rebase the small
explicit patch, refresh hashes/locks without unrelated upgrades, and rerun the
standalone tests, actual AztecCraft consumers, package/archive notice checks and
platform CI. There is no automatic sync.
