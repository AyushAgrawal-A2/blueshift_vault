# blueshift_vault

Solution to Blueshift's **Pinocchio Vault** challenge — the same per-wallet lamport vault as the
`blueshift_anchor_vault` repo, rebuilt without Anchor: `#![no_std]`, `nostd_panic_handler!`, and
every account check written by hand.

Dependencies: `pinocchio 0.11.2` (with the `cpi` feature), `pinocchio-system 0.6.1`. Program id is
`22222222222222222222222222222222222222222222` via `Address::from_str_const` — the fixed address
Blueshift's challenge harness expects.

## Wire format

| discriminator (byte 0) | instruction | data |
|------------------------|-------------|------|
| `0` | `deposit` | `amount: u64` LE — exactly 8 bytes, must exceed the rent-exempt minimum for a zero-data account |
| `1` | `withdraw` | none |

Accounts for both instructions: `[owner (signer), vault (PDA), system_program]` (the third is
accepted but unused). Vault PDA seeds: `[b"vault", owner]`.

## Validation

Each instruction is a struct built through `TryFrom` impls — one for the account slice, one for the
instruction data — so every check runs before `process()` issues a single CPI:

- `owner.is_signer()`
- `vault` owned by the System Program
- `vault`'s address recomputed with `derive_program_address([b"vault", owner], ID)` and compared —
  a caller cannot substitute another wallet's vault because the derivation commits to the signer
- deposit: `vault.lamports() == 0` (one live deposit per wallet) and
  `amount > Rent::minimum_balance(0)` — the same rent floor the Anchor variant enforces with
  `require_gt!`, so the vault is rent-exempt from the moment it is funded
- withdraw: `vault.lamports() != 0`, then the full balance goes back to `owner` via
  `invoke_signed` with the recomputed canonical bump

The vault never carries data — there is no state struct anywhere in the program. Draining it to
zero lamports lets the runtime reap the account, after which the same wallet can deposit again.

## Build / test

```console
$ cargo build-sbf     # → target/deploy/blueshift_vault.so
```

No tests in the repo; the challenge is graded by Blueshift's own suite against the compiled
program.
