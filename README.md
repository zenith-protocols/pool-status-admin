# Pool Status Admin

Admin for Blend pools that can only change their status.

## Overview

Make this contract the admin of a Blend v2 pool, and the pool's status is the
only admin power left. Reserves, rates, emissions and the admin itself can no
longer be changed. One deployment can administer any number of pools.

Built on the ownable module of
[OpenZeppelin Stellar Contracts](https://github.com/OpenZeppelin/stellar-contracts/tree/v0.7.0)
v0.7.0, pinned exactly.

## What it does

The contract is deployed with an owner and stores nothing else. A pool's admin
proposes it with `propose_admin`, and the owner takes over the role with
`accept_admin` for that pool. The pool keeps a proposal for 172,800 ledgers,
about 10 days, and anyone can extend it before it expires with
`stellar contract extend --key PropAdmin --durability temporary`.

From then on the owner calls `set_status` for the pool, and the pool applies its
own rules:

| Status | Effect | Allowed when |
|---|---|---|
| `0` admin active | Normal operation. `update_status` can only put it on-ice. | Backstop above the threshold, under 50% queued for withdrawal |
| `2` admin on-ice | Borrowing stops until the admin changes it | Under 75% queued |
| `3` on-ice | Borrowing stops, and `update_status` takes over again. Use it to lift `2` or `4`. | Under 75% queued |
| `4` admin frozen | Supplying and borrowing stop until the admin changes it | Always |

Withdrawals, repayments and liquidations stay open in every status. Only the
admin can lift an admin freeze.

Ownership is OpenZeppelin's `Ownable`. It can be transferred in two steps, and
it can be renounced. A renounce is immediate and leaves every pool in its current
status for good. The contract cannot be upgraded.

## Usage

```
stellar contract deploy --wasm-hash <hash> -- --owner <owner>
stellar contract invoke --id <pool> -- propose_admin --new_admin <this contract>               # pool admin
stellar contract invoke --id <this contract> -- accept_admin --pool <pool>                      # owner
stellar contract invoke --id <this contract> -- set_status --pool <pool> --pool_status 4       # owner
```

## Build and test

Requires stellar-cli 25.2.0 or newer. The OpenZeppelin crates need
`stellar contract build`; plain `cargo build` is refused.

```
make build
make test
make coverage   # needs cargo-llvm-cov
make fmt
```

## License

MIT, see [LICENSE](LICENSE).
