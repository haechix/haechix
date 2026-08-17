# HAECHIX Development Note

## 개발 기준

### QEMU `virt` 선정

QEMU `virt` machine을 Haechix의 주 개발 플랫폼으로 사용한다.

실제 Raspberry Pi 5 하드웨어로 포팅하기 전에 다음 AArch64 커널 기능을 빠르게 구현하고 반복 검증하기 위함이다.

- AArch64 최소 부팅
- Rust kernel 진입
- UART 출력
- Exception Level 정규화
- Device Tree 기반 하드웨어 탐색
- exception vector
- generic timer
- IRQ
- MMU
- allocator
- scheduler
- userspace와 syscall
- IPC와 capability

QEMU에서 검증된 공통 기능은 board 전용 코드와 분리하여 Raspberry Pi 5 포팅 시 재사용한다.

### 기본 개발환경

| 항목 | 설정 |
|---|---|
| 주 개발환경 | WSL2 Ubuntu |
| 가상 보드 | QEMU `virt` |
| QEMU executable | `qemu-system-aarch64` |
| 기본 CPU model | `cortex-a76` |
| 기본 CPU 수 | 1 |
| 기본 메모리 | 256 MiB |
| 첫 물리 보드 | Raspberry Pi 5 |
| Rust version | `1.97.1` |
| Rust target | `aarch64-unknown-none` |
| Debugger | `gdb-multiarch` |

### GDB 설치 목적

WSL2의 x86_64 환경에서 QEMU가 실행하는 AArch64 Haechix kernel을 원격으로 디버깅하기 위해 사용한다.

QEMU에는 GDB remote stub이 내장되어 있으며, QEMU를 `-S -s` 옵션으로 실행하면 TCP `1234` port에서 GDB 연결을 기다린다.

### GDB 기본 검증 절차

1. QEMU를 `-S -s` 옵션으로 실행한다.
2. QEMU는 CPU 실행을 중단한 상태로 GDB 연결을 기다린다.
3. `gdb-multiarch`에서 Haechix ELF를 symbol file로 연다.
4. `target remote :1234`로 QEMU에 연결한다.
5. `_start`에서 assembly 실행 흐름을 추적한다.
6. `haechix_qemu_virt::start` breakpoint 도달을 확인한다.
7. `haechix_kernel::start` breakpoint 도달을 확인한다.
8. AArch64 register와 memory를 검사한다.
9. Rust 반환 후 `wfe` parking loop 진입을 확인한다.

---

## Phase A — QEMU Minimal Boot

Phase A의 목적은 QEMU `virt` 환경에서 AArch64 kernel의 최소 부팅 경로를 확립하고, Raspberry Pi 5 포팅 전에 공통 kernel 기반을 검증하는 것이다.

### Milestone 구성

| Milestone | 제목 | 핵심 목표 |
|---|---|---|
| M00 | Workspace Initialization | 개발환경, workspace 및 crate 경계 확립 |
| M01 | QEMU Rust Entry | AArch64 `_start`에서 Rust kernel 진입 |
| M02 | QEMU PL011 UART | MMIO 기반 초기 UART 출력 |
| M03 | Exception Level Normalization | EL1 확인 및 EL2→EL1 전환 |
| M04 | Minimal DTB Parser | DTB 기반 하드웨어 정보 탐색과 `BootInfo` 전달 |

---

### M00 — Workspace Initialization

| 작업 | 내용 |
|---|---|
| M00-A | WSL2 Ubuntu를 주 개발·빌드·검증 환경으로 확정 |
| M00-B | QEMU, GDB 및 AArch64 원격 디버깅 환경 구성 |
| M00-C | Rust `1.97.1` 및 `aarch64-unknown-none` target 고정 |
| M00-D | `rust-src`, `llvm-tools`, `rustfmt`, `clippy` 구성 |
| M00-E | `kernel`, `arch-aarch64`, `drivers`, `boot-protocol`, `user-abi` 공통 crate 생성 |
| M00-F | QEMU virt와 Raspberry Pi 5 board binary crate 생성 |
| M00-G | Cargo workspace, resolver 3 및 panic abort profile 구성 |
| M00-H | 모든 공통 crate와 board binary를 `no_std` 기반으로 전환 |
| M00-I | 개발 규칙, 의존성 경계, 테스트 계약 및 Git 규칙 정리 |

#### 주요 설계 결과

| 영역 | 책임 |
|---|---|
| `crates/kernel` | board·architecture에 독립적인 kernel 정책과 상태 |
| `crates/arch-aarch64` | 재사용 가능한 AArch64 mechanism |
| `crates/drivers` | 재사용 가능한 device driver |
| `crates/boot-protocol` | board에서 kernel로 전달하는 boot contract |
| `crates/user-abi` | 향후 userspace와 kernel 사이의 ABI |
| `boards/qemu-virt` | QEMU 전용 주소, linker layout 및 초기화 |
| `boards/rpi5` | Raspberry Pi 5 전용 boot·hardware 초기화 |

#### Workspace 의존성 원칙

- `kernel`은 board crate에 의존하지 않는다.
- board 전용 MMIO 주소를 `kernel`에 넣지 않는다.
- `arch-aarch64`는 QEMU나 Raspberry Pi 전용 주소를 소유하지 않는다.
- `drivers`는 호출자가 전달한 MMIO 주소를 사용한다.
- `boot-protocol`은 board와 kernel 사이의 명시적인 데이터 계약만 정의한다.
- crate 사이의 순환 의존성을 허용하지 않는다.
- 현재 milestone에 필요하지 않은 외부 dependency를 추가하지 않는다.

#### `no_std` 적용 대상

- `crates/kernel`
- `crates/arch-aarch64`
- `crates/drivers`
- `crates/boot-protocol`
- `crates/user-abi`
- `boards/qemu-virt`
- `boards/rpi5`

`#![no_std]`는 운영체제가 제공하는 Rust 표준 라이브러리 `std`를 연결하지 않고, bare-metal에서도 사용할 수 있는 `core`만으로 빌드한다는 의미이다.

#### 검증 결과

- `rustc 1.97.1` 활성화 확인
- `aarch64-unknown-none` target 설치 확인
- `rust-src`, `llvm-tools`, `rustfmt`, `clippy` 설치 확인
- `qemu-system-aarch64` 실행 확인
- `gdb-multiarch` 실행 확인
- QEMU의 `cortex-a76` CPU model 지원 확인
- `cargo fmt --all --check` 통과
- `haechix-qemu-virt` AArch64 cross-build 성공
- `haechix-rpi5` AArch64 cross-build 성공
- workspace package 및 dependency boundary 확인
- Cargo build artifact를 `/target/`으로 Git에서 제외
- `testspecs/m00.yaml` 테스트 계약 작성 및 문법 검증

#### 의도적으로 유예한 항목

- 실제 boot entry `_start`
- linker script와 boot stack
- UART 및 device 초기화
- Exception Level 전환
- Raspberry Pi 5 실제 hardware boot
- Raspberry Pi 4 지원

---

### M01 — QEMU Rust Entry

| 작업 | 내용 |
|---|---|
| M01-A | QEMU board linker layout 정의 |
| M01-B | 최소 AArch64 `_start` assembly 작성 |
| M01-C | assembly와 linker script를 QEMU board 빌드에 연결 |
| M01-D | interrupt masking과 64 KiB boot stack 설정 |
| M01-E | linker symbol 기반 `.bss` zero clear 구현 |
| M01-F | board `start()`에서 `kernel::start()` 호출 |
| M01-G | Rust 반환 시 `wfe` loop, panic 시 spin loop 진입 |
| M01-H | ELF entry point, section, symbol 및 disassembly 검사 |
| M01-I | QEMU Cortex-A76 실행 및 GDB 진입 경로 검증 |

#### 부트 흐름

```text
QEMU
  │
  ▼
ELF entry `_start`
  │
  ├─ DAIF interrupt masking
  ├─ boot stack 설치
  ├─ .bss zero clear
  ▼
haechix_qemu_virt::start()
  │
  ▼
haechix_kernel::start()
  │
  ▼
반환 시 WFE parking loop
```

#### Linker layout

| 항목 | 값 또는 역할 |
|---|---|
| `_start` | `0x40080000` |
| `.text.boot` | 초기 assembly entry |
| `.text` | Rust 및 일반 실행 코드 |
| `.rodata` | 읽기 전용 데이터 |
| `.data` | 초기화된 writable data |
| `.bss` | zero clear 대상 |
| boot stack | 64 KiB |
| `__bss_start` | `.bss` 시작 linker symbol |
| `__bss_end` | `.bss` 끝 linker symbol |
| `__boot_stack_top` | 초기 stack top |

#### 검증 결과

- ELF `_start`: `0x40080000`
- QEMU CPU model: `cortex-a76`
- boot stack 크기: 64 KiB
- GDB에서 `_start` 진입 확인
- `haechix_qemu_virt::start` breakpoint 도달
- `haechix_kernel::start` 진입 확인
- Rust 함수 반환 후 `wfe` loop 진입 확인
- ELF section, symbol 및 `_start` disassembly 확인
- QEMU AArch64 cross-build 성공
- Raspberry Pi 5 AArch64 회귀 빌드 성공
- `testspecs/m01.yaml` 테스트 계약 작성 및 문법 검증

#### 의도적으로 유예한 항목

- Raspberry Pi 5의 `_start`와 linker layout
- 실제 UART 출력
- Exception Level 정규화
- exception vector
- generic timer와 IRQ
- MMU와 allocator

---

### M02 — QEMU PL011 UART

| 작업 | 내용 |
|---|---|
| M02-A | `m02.yaml` 테스트 계약과 정확한 UART 출력 문자열 정의 |
| M02-B | `drivers`에 재사용 가능한 PL011 UART 모듈 생성 |
| M02-C | PL011 data register와 flag register offset 정의 |
| M02-D | volatile MMIO read/write 경계 구현 |
| M02-E | TX FIFO full bit polling 구현 |
| M02-F | byte 및 UTF-8 문자열 순차 송신 구현 |
| M02-G | QEMU board가 PL011 MMIO base address를 driver에 주입 |
| M02-H | Rust 진입 전 EL1 FP/Advanced SIMD 접근 허용 |
| M02-I | QEMU·GDB·Raspberry Pi 5 회귀 검증 |
| M02-J | QEMU terminal에서 실제 UART 문자열 출력 확인 |

#### PL011 설계

| 항목 | 값 |
|---|---|
| QEMU PL011 base | `0x09000000` |
| Data Register offset | `0x00` |
| Flag Register offset | `0x18` |
| TX FIFO Full bit | bit 5 |
| MMIO 접근 방식 | `read_volatile` / `write_volatile` |
| 전송 방식 | polling |
| 주소 소유자 | `boards/qemu-virt` |
| driver 소유자 | `crates/drivers` |

QEMU 전용 주소는 board가 소유하고 PL011 driver는 호출자가 전달한 주소만 사용한다.

따라서 같은 driver를 Raspberry Pi 5에서도 해당 board의 PL011 주소를 전달하여 재사용할 수 있다.

#### 전송 흐름

```text
write_str()
  │
  └─ 문자열을 byte 단위로 순회
       │
       ▼
    write_byte()
       │
       ├─ Flag Register의 TXFF bit 확인
       ├─ FIFO가 가득 찬 동안 polling
       └─ Data Register에 byte 기록
```

#### 초기 exception 원인과 해결

Rust compiler는 `aarch64-unknown-none` target에서도 FP/Advanced SIMD 명령을 생성할 수 있다.

초기 EL1 환경에서 FP/Advanced SIMD 접근이 허용되지 않은 상태로 해당 명령이 실행되면서 CPU가 `0x200` exception vector로 진입하는 문제가 발생했다.

해결을 위해 Rust 진입 전에 다음 작업을 수행했다.

1. `CPACR_EL1`을 읽는다.
2. `FPEN` field를 설정한다.
3. `CPACR_EL1`에 다시 기록한다.
4. `isb`를 실행하여 설정을 반영한다.
5. Rust board entry로 진입한다.

#### 검증 결과

- `haechix-drivers` AArch64 빌드 성공
- `haechix-qemu-virt` AArch64 빌드 성공
- `haechix-rpi5` 회귀 빌드 성공
- GDB에서 `Pl011::write_byte()` breakpoint 도달
- 첫 전송 byte가 `72`, 즉 ASCII `H`임을 확인
- PL011 base address가 `0x09000000`임을 확인
- Flag Register의 TX FIFO full bit가 clear 상태임을 확인
- QEMU terminal에서 다음 문자열 출력 확인

```text
Haechix M02: QEMU UART OK
```

- UART 출력 후 kernel 진입 및 parking loop 유지 확인
- `testspecs/m02.yaml` 테스트 계약 작성 및 문법 검증

#### 의도적으로 유예한 항목

- UART 수신
- interrupt 기반 UART
- 동시성 및 locking
- Raspberry Pi 5 실제 UART 출력
- 범용 console abstraction

---

### M03 — Exception Level Normalization

| 작업 | 내용 |
|---|---|
| M03-A | `m03.yaml` 테스트 계약 및 지원 Exception Level 정책 정의 |
| M03-B | `arch-aarch64`에 `ExceptionLevel` enum과 판별 모듈 추가 |
| M03-C | `CurrentEL` register를 읽고 EL0~EL3 encoding 해석 |
| M03-D | stack을 사용하지 않는 `aarch64_normalize_to_el1` assembly 구현 |
| M03-E | EL1 시작 경로에서 boot stack을 설치하고 계속 실행 |
| M03-F | EL2 시작 경로에서 `SP_EL1`, `HCR_EL2`, `CPTR_EL2` 설정 |
| M03-G | `SPSR_EL2`와 `ELR_EL2`를 구성하고 `eret`으로 EL1h 진입 |
| M03-H | EL3 거부 경로와 EL0 제외 계약 정의 |
| M03-I | `CurrentEL`, `MPIDR_EL1`, `SCTLR_EL1` UART 출력 및 GDB 검증 |

#### Exception Level 정책

| 시작 상태 | 처리 |
|---|---|
| EL0 | bare-metal boot contract 범위 밖 |
| EL1 | 별도 전환 없이 EL1h boot stack 설치 |
| EL2 | 필요한 EL2 register 설정 후 `eret`으로 EL1h 진입 |
| EL3 | 현재 미지원, board 초기 진단 후 parking loop 진입 |

#### 정규화 흐름

```text
_start
  │
  ├─ x0 = DTB 주소 보존
  ├─ x1 = EL1 boot stack top 전달
  ▼
aarch64_normalize_to_el1
  │
  ├─ EL1 → stack 설치 후 return
  ├─ EL2 → SP_EL1/HCR_EL2/CPTR_EL2 설정 → ERET
  └─ EL3 → unsupported 결과 반환
  │
  ▼
EL1h에서 Rust board start 진입
```

#### 주요 register

| Register | 목적 |
|---|---|
| `CurrentEL` | 현재 Exception Level 식별 |
| `SP_EL1` | EL1h에서 사용할 boot stack 설치 |
| `HCR_EL2.RW` | EL1을 AArch64 상태로 실행 |
| `CPTR_EL2.TFP` | EL1 FP/Advanced SIMD trap 방지 |
| `SPSR_EL2` | EL1h 복귀 상태와 DAIF mask 설정 |
| `ELR_EL2` | EL1 복귀 주소 설정 |
| `MPIDR_EL1` | 현재 CPU affinity 확인 |
| `SCTLR_EL1` | EL1 system control 상태 확인 |

#### 검증 결과

기본 QEMU profile:

```text
Haechix M02: QEMU UART OK
Haechix M03: EL1 OK
CurrentEL=EL1
MPIDR_EL1=0x0000000080000000
SCTLR_EL1=0x0000000030c50838
```

- 기본 QEMU profile에서 EL1 진입 확인
- GDB에서 board `start()` 진입 시 CurrentEL encoding `0x4` 확인
- `-machine virt,virtualization=on` profile에서 초기 EL2 encoding `0x8` 확인
- EL2에서 EL1으로 전환한 뒤 CurrentEL encoding `0x4` 확인
- M02 UART 출력이 EL 정규화 이후에도 유지되는지 확인
- EL1 FP/Advanced SIMD 접근 유지 확인
- QEMU AArch64 빌드 성공
- Raspberry Pi 5 회귀 빌드 성공
- `testspecs/m03.yaml` 테스트 계약 작성 및 문법 검증

#### 의도적으로 유예한 항목

- EL3→EL1 전환
- exception vector table
- synchronous exception handler
- IRQ/FIQ handler
- generic timer
- SMP secondary CPU 진입

---

### M04 — Minimal DTB Parser

| 작업 | 내용 |
|---|---|
| M04-A | `m04.yaml` 테스트 계약과 parser·board·kernel 책임 경계 정의 |
| M04-B | `boot-protocol`에 정규화된 `BootInfo` 계약 정의 |
| M04-C | QEMU가 `x0`로 전달한 DTB 물리 주소를 `_start`에서 Rust까지 보존 |
| M04-D | 재사용 가능한 allocation-free `no_std` FDT parser crate 생성 |
| M04-E | FDT header, magic, version, block offset·size·alignment 검증 |
| M04-F | structure token, strings block, property 및 `reg` cell 해석 |
| M04-G | compatible, memory, chosen/stdout-path, UART와 interrupt controller 탐색 |
| M04-H | bootstrap UART 소유권 종료 후 DTB 기반 UART로 전환 |
| M04-I | `BootInfo` 구성, kernel 전달 및 QEMU·GDB·Raspberry Pi 5 회귀 검증 |

#### ELF와 raw binary의 역할

| Artifact | 목적 |
|---|---|
| `haechix-qemu-virt` ELF | symbol, section 및 GDB source-level debugging |
| `haechix-qemu-virt.bin` | QEMU AArch64 raw-kernel boot protocol 실행 |

```text
Rust source
  │
  ▼
cargo build
  │
  ├─ ELF ───────────────► GDB symbol/debug 정보
  │
  └─ llvm-objcopy
         │
         ▼
      raw .bin ─────────► QEMU 실제 부팅
```

QEMU의 raw-kernel boot stub은 `_start`로 이동하기 전에 DTB의 물리 주소를 `x0`에 전달한다.

DTB는 kernel entry address를 결정하지 않으며 하드웨어 구성 정보를 전달하는 데이터 구조로만 사용된다.

#### FDT parser 검증 범위

- FDT magic `0xd00dfeed`
- 40-byte FDT header
- big-endian `u32` 및 `u64`
- `totalsize`
- structure/string/reservation block 범위
- structure block 4-byte alignment
- memory reservation block 8-byte alignment
- block overlap 방지
- `FDT_BEGIN_NODE`
- `FDT_END_NODE`
- `FDT_PROP`
- `FDT_NOP`
- `FDT_END`
- node depth underflow 검사
- unclosed node 검사
- 활성 node 밖의 property 거부
- NUL-terminated node/property string
- UTF-8 검증
- 1-cell 및 2-cell `reg` tuple
- allocation 없는 borrowed slice와 `&str` 반환

#### QEMU platform 탐색 결과

| 정보 | DTB source | 결과 |
|---|---|---|
| Compatible | root `compatible` | `linux,dummy-virt` |
| Memory start | memory node `reg` | `0x40000000` |
| Memory size | memory node `reg` | `0x10000000` |
| Console path | `/chosen/stdout-path` | `/pl011@9000000` |
| PL011 base | console node `reg` | `0x09000000` |
| Interrupt controller | `interrupt-controller` node `reg` | `0x08000000` |

`stdout-path`의 `:115200n8` 같은 선택적 suffix는 제거한다.

Alias 기반 `stdout-path`는 M04 범위에서 지원하지 않고 절대 node path만 허용한다.

#### `BootInfo` 계약

```rust
pub struct BootInfo {
    pub memory_start: usize,
    pub memory_size: usize,
    pub uart_base: usize,
    pub interrupt_controller_base: usize,
}
```

`BootInfo`에는 raw DTB pointer나 QEMU 전용 상수가 들어가지 않는다.

Board가 DTB 값을 검증하고 정규화한 후 명시적인 참조로 kernel에 전달한다.

#### M04 완료 시점 부트 흐름

```text
qemu-system-aarch64
  │
  ├─ raw kernel binary 적재
  ├─ QEMU virt DTB 생성
  └─ x0 = DTB physical address
          │
          ▼
_start @ 0x40080000
  │
  ├─ interrupt masking
  ├─ Exception Level → EL1h 정규화
  ├─ x0 DTB 주소 보존
  ├─ boot stack 설치
  ├─ FP/Advanced SIMD 허용
  └─ .bss zero clear
          │
          ▼
haechix_qemu_virt::start(dtb_address)
  │
  ├─ bootstrap PL011 생성
  ├─ DTB pointer·totalsize 검증
  ├─ FDT header·token·property 검증
  ├─ QEMU platform 정보 탐색
  ├─ BootInfo 구성
  ├─ bootstrap UART release
  └─ DTB 기반 PL011 생성
          │
          ▼
haechix_kernel::start(&boot_info)
  │
  ▼
반환 시 WFE parking loop
```

#### 검증 결과

FDT parser:

- `cargo test -p haechix-fdt`
- 단위 테스트 `31 passed`
- 잘못된 magic, header, block, token, string 및 property 거부 확인
- 1-cell·2-cell `reg` tuple 해석 확인
- 절대 `stdout-path`와 suffix 제거 확인
- `no_std`, allocation-free 및 board-independent 경계 확인

정적 검사:

- `cargo fmt --all --check` 통과
- M04 대상 Clippy `-D warnings` 통과
- `haechix-fdt` AArch64 cross-build 성공
- `haechix-boot-protocol` AArch64 cross-build 성공
- `haechix-qemu-virt` AArch64 빌드 성공
- ELF에서 raw `.bin` 변환 성공
- `haechix-rpi5` 회귀 빌드 성공

기본 EL1 및 EL2→EL1 QEMU profile:

```text
Haechix M02: QEMU UART OK
Haechix M03: EL1 OK
CurrentEL=EL1
MPIDR_EL1=0x0000000080000000
SCTLR_EL1=0x0000000030c50838
Haechix M04: DTB OK
compatible=linux,dummy-virt
memory=0x0000000040000000..0x0000000050000000
console=pl011@0x0000000009000000
interrupt-controller=0x0000000008000000
```

GDB 확인 결과:

```text
dtb_address = 0x48000000

BootInfo {
    memory_start: 0x40000000,
    memory_size: 0x10000000,
    uart_base: 0x09000000,
    interrupt_controller_base: 0x08000000,
}
```

- 기본 QEMU profile에서 DTB 탐색 성공
- EL2 시작 profile에서도 EL1 전환 후 DTB 주소 보존 확인
- `haechix_kernel::start(&BootInfo)` breakpoint 도달
- Python/PyYAML로 `m00.yaml`부터 `m04.yaml`까지 문법 검증

#### 의도적으로 유예한 항목

- 완전한 Device Tree specification
- DTB overlay 및 mutation
- alias와 phandle 해석
- multiple memory range 관리
- reserved-memory 처리
- interrupt specifier 해석
- device status filtering
- MMU와 allocator
- Raspberry Pi 5 DTB runtime parsing
- Raspberry Pi 5 실제 hardware 검증

---

## 다음 Milestone 기록 템플릿

### M05 — `<Milestone Title>`

> M05의 기능 범위를 확정한 후 `testspecs/m05.yaml`을 먼저 작성하고 아래 표에 atomic 작업 단위를 기록한다.

| 작업 | 내용 |
|---|---|
| M05-A | 테스트 계약과 완료 조건 정의 |
| M05-B | 핵심 데이터 구조 또는 interface 정의 |
| M05-C | 최소 구현 |
| M05-D | architecture·board·driver 경계 연결 |
| M05-E | 오류 및 안전 경계 처리 |
| M05-F | 단위 테스트 |
| M05-G | QEMU runtime 검증 |
| M05-H | GDB 검증 |
| M05-I | Raspberry Pi 5 회귀 빌드 |
| M05-J | 문서, 최종 diff 및 atomic commit |

#### 설계 결정

- M05 진행 시 기록

#### 검증 결과

- 실행한 명령
- 실제 출력
- 성공한 항목
- 경고와 제한사항
- 실제 hardware 검증 여부

#### 의도적으로 유예한 항목

- M06 이후 기능을 이곳에 명시

---

## 전체 개발 단계

| Phase | 주 실행 환경 | 목적 |
|---|---|---|
| Phase A | QEMU | 최소 부팅·UART·EL·DTB 기반 완성 |
| Phase B | Raspberry Pi 5 | 최소 entry와 UART 조기 검증 |
| Phase C | QEMU | exception vector, timer, IRQ, allocator, MMU, scheduler |
| Phase D | QEMU | userspace, syscall, IPC, capability |
| Phase E | Raspberry Pi 5 + QEMU | 완성된 microkernel 기능을 Raspberry Pi 5에 포팅 |
| Phase F 이후 | Raspberry Pi 5 + QEMU | SMP 및 고급 기능 |