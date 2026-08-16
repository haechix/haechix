# Haechix Development Rules

## Project

1. Haechix is a Rust-based AArch64 microkernel.
2. The primary development platform is QEMU `virt`.
3. The default QEMU CPU model is `cortex-a76`.
4. Initial development and validation use one virtual CPU.
5. The first official physical hardware target is Raspberry Pi 5.
6. Rust is pinned to version 1.97.1.
7. Bare-metal builds use the `aarch64-unknown-none` target.

## Atomic Milestones

1. Implement only one milestone at a time.
2. Do not implement functionality from the next milestone.
3. Each atomic task must have one explicit validation condition.
4. Do not perform unrelated refactoring.
5. Do not add dependencies that are unnecessary for the current milestone.
6. Stop after satisfying the current milestone completion conditions.
7. A successful build must not be reported as successful hardware validation.

## Architecture Boundaries

1. `crates/kernel` contains platform-independent kernel policy and state.
2. `crates/arch-aarch64` contains reusable AArch64-specific mechanisms.
3. `crates/drivers` contains reusable device implementations.
4. `crates/boot-protocol` defines the boot-time board/kernel contract.
5. `crates/user-abi` defines the future userspace/kernel binary contract.
6. `boards/qemu-virt` owns QEMU-specific addresses and initialization.
7. `boards/rpi5` owns Raspberry Pi 5-specific addresses and initialization.
8. The kernel must not depend on a board crate.
9. Board-specific MMIO addresses must not appear in the kernel.
10. Platform differences must be supplied through explicit data or interfaces.

## Rust and Memory

1. Kernel, architecture, driver, protocol, and ABI crates use `#![no_std]`.
2. Do not use `Vec`, `Box`, or `String` before an allocator exists.
3. Do not use the `alloc` crate before the allocator milestone.
4. Prefer fixed-size data structures during early development.
5. Avoid mutable global state.
6. Document synchronization requirements when mutable global state is unavoidable.

## Unsafe Rust

1. Use `unsafe` only at architecture, boot, MMU, context-switch, and driver boundaries.
2. Every `unsafe` block must include a documented safety invariant.
3. Document pointer alignment, lifetime, and aliasing requirements.
4. Use volatile operations for MMIO access.
5. Do not hide unverified assumptions behind a safe abstraction.

## Validation

Run the validation appropriate for the current milestone:

1. `cargo fmt --all --check`
2. Relevant package builds
3. `cargo clippy` where applicable
4. QEMU smoke tests where applicable
5. GDB validation where required
6. Raspberry Pi 5 hardware validation only on real hardware

## Reporting

Report the following after each milestone:

1. Changed files
2. Design decisions
3. Dependency changes
4. Unsafe locations and safety invariants
5. Validation commands
6. Actual validation results
7. Failed or unverified areas
8. Work intentionally deferred to the next milestone

## User-Owned Files

`DEVNOTE.md` is maintained by the user. Do not modify it unless explicitly requested.

## Git Commit Convention

1. Use the format `type(scope): description`.
2. Include the completed phase and milestone in milestone commits.
3. Use `chore` for workspace, configuration, and maintenance changes.
4. Use `feat` for new functionality.
5. Use `fix` for defect corrections.
6. Use `refactor` for structural changes without behavior changes.
7. Use `docs` for documentation-only changes.
8. Use `test` for test-only changes.
9. Use a scope that identifies the affected subsystem, such as `workspace`, `boot`, `uart`, `exception`, `mmu`, or `ipc`.
10. Record validation commands, actual results, warnings, and deferred work in the commit body.

Example:

`chore(workspace): complete Phase 0 M00 initialization`