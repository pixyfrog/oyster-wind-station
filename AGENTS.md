# AGENTS.md — oyster-wind-station receiver

## Repo and layout

- The **root crate is the Orange Pi receiver** (`lora_receiver`). Receiver Rust code lives in
  `src/main.rs` and `src/packet.rs`.
- Do **not** touch `oyster-pico-tx/` (island firmware) or `tbeam-field-rx/` (T-Beam sketch)
  unless a task explicitly says so.

## Build, check, test

- `cargo check`
- `cargo test` — the packet round-trip / rejection tests must pass.
- Plain `cargo build` produces the **debug** binary. It is not the production artifact.

### Known, deliberate warnings — do not "fix" them

- `field 0 is never read` on `SpiError` (`src/rfm95w.rs`) — compiler quirk; the payload exists
  so `{:?}` prints something useful.
- `constant REG_FIFO is never used` (`src/rfm95w.rs`) — pre-existing.
- `dead_code` for items flagged by `cargo check` is intentional; report new warnings, don't
  silence them.

## Production build — always `--release`

The systemd unit runs the **release** binary:

```
/etc/systemd/system/oyster-rx.service
WorkingDirectory=/root/oyster-wind-station
ExecStart=/root/oyster-wind-station/target/release/lora_receiver
User=root
```

So a plain `cargo build` (debug) never updates the running service. **Production deploys must
build `--release` in the service's working directory.**

## Deploy workflow — the Pi is a deploy target only

There is no editor on the Orange Pi — never edit source there. Update the clone the unit runs
from (`/root/oyster-wind-station`), then rebuild and restart.

1. On the local machine, before typing any code:
   `git pull --no-rebase --no-edit origin main`
2. After changes pass `cargo check` / `cargo test`:
   `git add <files> && git commit -m "receiver: <what changed>" && git push origin main`
   If the push is rejected with "fetch first":
   `git pull --no-rebase --no-edit origin main`, then push again.
3. On the Orange Pi:
   ```
   ssh oyster
   sudo -n bash -lc 'cd /root/oyster-wind-station && git pull --no-rebase --no-edit origin main && cargo build --release'
   sudo systemctl restart oyster-rx
   sudo journalctl -u oyster-rx -n 30
   curl -s http://127.0.0.1:3000/
   ```
4. Confirm the receiver's startup line (`Server on http://<ip>:3000`) with **no** bind error,
   and that the dashboard shows the expected fields after a real packet arrives.

Only one process may hold port 3000 — stop any manual run before restarting the service.
Disk is tight on the 7.2 GB SD card; `cargo build --release` is the production step, but avoid
unnecessary full rebuilds.
