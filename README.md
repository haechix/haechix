# Haechix

**A Rust-based ARM64 microkernel for hard real-time systems, robotics, and Physical AI.**

> **Do Not Fear.**

Haechix is an open-source microkernel written in **Rust** and designed for **ARM64** systems.

The project is being built from scratch to explore modern operating-system architecture with a strong focus on **determinism, isolation, reliability, and hard real-time behavior**.

Haechix begins as a minimal bare-metal kernel running on **QEMU ARM64 `virt`** and will progressively evolve toward a practical microkernel capable of running on real ARM64 hardware, starting with the **Raspberry Pi 5**.

The long-term vision is to explore whether a small, deterministic, and memory-safe operating system can provide a reliable foundation for **robotics, Physical AI, autonomous systems, and other real-time edge computing platforms**.

---

## Why Haechix?

Modern robots and Physical AI systems combine increasingly complex software with strict requirements for:

* deterministic execution
* low and predictable latency
* fault isolation
* memory safety
* efficient inter-process communication
* real-time scheduling
* hardware-level control
* system reliability

Traditional monolithic operating systems provide rich functionality, but their complexity can make strong isolation and predictable timing difficult to reason about.

Haechix explores a different direction:

```text
Applications
      │
      ▼
┌─────────────────────────────┐
│      User-space Services    │
│                             │
│ Drivers │ Runtime │ System  │
│ Services│         │ Services│
└──────────────┬──────────────┘
               │ IPC
               ▼
┌─────────────────────────────┐
│          Haechix            │
│         Microkernel         │
│                             │
│ Scheduler │ IPC │ Memory    │
│ Exception │ IRQ │ Isolation │
└──────────────┬──────────────┘
               │
               ▼
        ARM64 Hardware
```

The goal is to keep the privileged kernel small while moving higher-level functionality into isolated user-space components.

---

## Design Goals

Haechix is being designed around several core principles.

### Hard Real-Time

Hard real-time capability is a **design goal**, not a current claim.

The project will explore mechanisms required for deterministic execution, including:

* priority-based preemptive scheduling
* bounded kernel execution paths
* predictable interrupt handling
* deterministic IPC
* priority inversion mitigation
* timer and deadline management
* latency and jitter measurement
* WCET-aware design where practical

Real-time guarantees will only be claimed when they can be demonstrated through measurement and analysis.

### Memory Safety

Rust is used as the primary implementation language to reduce classes of memory-safety bugs while retaining low-level control over the hardware.

`unsafe` Rust will inevitably be required at hardware and architecture boundaries, but Haechix aims to keep unsafe code **small, explicit, and auditable**.

### Isolation

Processes and system services should operate within clearly defined memory and privilege boundaries.

A failure in one component should not unnecessarily compromise the rest of the system.

### Minimalism

The microkernel should contain only mechanisms that genuinely require privileged execution.

Where practical, drivers, services, and higher-level OS functionality should live outside the kernel.

### Deterministic IPC

IPC is one of the most important mechanisms in a microkernel architecture.

Haechix aims to explore IPC designs suitable for real-time systems where not only throughput but also **latency predictability** matters.

---

## Target Architecture

The initial architecture is:

```text
ARM64 / AArch64
```

Initial development environment:

```text
Windows
   │
   ▼
WSL2 Ubuntu
   │
   ▼
QEMU ARM64 virt
   │
   ▼
Haechix
```

The first real-hardware target is planned to be:

```text
Raspberry Pi 5
      │
      ▼
ARM64
      │
      ▼
Haechix
```

Additional ARM64 platforms may be supported as the project matures.

---

## Development Roadmap

### Phase 0 — Bootstrap

* [ ] Rust bare-metal project
* [ ] AArch64 target configuration
* [ ] Linker script
* [ ] Kernel entry point
* [ ] QEMU ARM64 boot
* [ ] Basic UART output

First milestone:

```text
Hello from Haechix
```

### Phase 1 — Architecture Foundation

* [ ] Exception vector table
* [ ] Exception handling
* [ ] IRQ handling
* [ ] ARM generic timer
* [ ] Basic architecture abstraction

### Phase 2 — Memory Management

* [ ] Physical memory discovery
* [ ] Physical page allocator
* [ ] ARM64 page tables
* [ ] MMU initialization
* [ ] Virtual address spaces
* [ ] Kernel/user memory isolation

### Phase 3 — Scheduling

* [ ] Task representation
* [ ] Context switching
* [ ] Kernel threads
* [ ] Preemptive scheduler
* [ ] Priority scheduling
* [ ] Real-time scheduling experiments
* [ ] Scheduling latency measurement

### Phase 4 — IPC

* [ ] Synchronous message passing
* [ ] Endpoint abstraction
* [ ] Blocking and wake-up
* [ ] IPC timeout
* [ ] Priority-aware IPC
* [ ] IPC latency benchmarks

### Phase 5 — User Mode

* [ ] EL0 execution
* [ ] System calls
* [ ] Per-process address spaces
* [ ] User-space process loading
* [ ] Process isolation

### Phase 6 — Microkernel Services

* [ ] User-space system services
* [ ] User-space drivers
* [ ] Resource management model
* [ ] Capability or permission model
* [ ] Service discovery

### Phase 7 — Hard Real-Time

* [ ] Deterministic scheduling model
* [ ] Priority inversion handling
* [ ] Bounded critical sections
* [ ] Interrupt latency analysis
* [ ] Scheduling latency analysis
* [ ] IPC latency analysis
* [ ] Jitter measurement
* [ ] Stress testing
* [ ] Real-time benchmark suite

### Phase 8 — Raspberry Pi 5

* [ ] Raspberry Pi 5 boot
* [ ] UART
* [ ] Interrupt controller
* [ ] ARM generic timer
* [ ] Memory map support
* [ ] Multicore bring-up
* [ ] Hardware validation

### Future

Possible future areas include:

* [ ] SMP scheduling
* [ ] CPU affinity
* [ ] real-time multicore scheduling
* [ ] zero-copy IPC
* [ ] POSIX compatibility layer
* [ ] robotics middleware integration
* [ ] ROS 2 experiments
* [ ] heterogeneous compute support
* [ ] safety-oriented architecture
* [ ] additional ARM64 platforms

---

## Physical AI & Robotics

Haechix ultimately aims to investigate operating-system architecture for systems where software interacts directly with the physical world.

Examples include:

```text
Robots
Autonomous machines
Industrial systems
Edge AI devices
Real-time controllers
Physical AI platforms
```

A future Haechix-based system might look like:

```text
        Physical AI Application
                  │
        Robotics Middleware
                  │
     ┌────────────┴────────────┐
     │                         │
 Perception                Control
     │                         │
     └────────────┬────────────┘
                  │
                 IPC
                  │
             Haechix
           Microkernel
                  │
     ┌────────────┼────────────┐
     │            │            │
    CPU         Memory       Devices
     │            │            │
     └────────────┴────────────┘
                  │
            Physical World
```

Haechix does not attempt to solve Physical AI itself.

Instead, the project aims to explore the **deterministic and reliable system-software foundation underneath it**.

---

## Project Status

> **Early development / experimental**

Haechix is currently a learning, research, and engineering project under active development.

APIs, architecture, directory structures, and design decisions are expected to change significantly.

At this stage, Haechix should **not** be considered production-ready or safety-certified.

---

## Contributing

Haechix is currently in its early architectural stage.

Discussions, experiments, bug reports, design proposals, and contributions are welcome as the project evolves.

Please keep contributions:

* small and reviewable
* clearly documented
* architecture-conscious
* deterministic where real-time behavior matters
* explicit about the use of `unsafe`
* accompanied by tests or measurements where practical

---

## License

Haechix is licensed under the **Apache License 2.0**.

The license permits commercial and non-commercial use, modification, and distribution under the terms of the license.

See [`LICENSE`](LICENSE) for details.

---

## Name

**Haechix** is inspired by the Korean mythical creature **Haechi (해치 / 해태)**, traditionally associated with justice, protection, and the ability to distinguish right from wrong.

The **X** represents experimentation, extensibility, and the unknown challenges ahead.

---

## Philosophy

Haechix is built around a simple idea:

> Build small.
> Understand deeply.
> Measure everything.
> Make timing predictable.
> Move from simulation to real hardware.

**Do Not Fear.**
