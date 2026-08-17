#HAECHIX Development Note

1. QEMU virt 선정. AArch64커널 기능을 사전 검증 및 평가하기 위함
2. GDB
   1. 설치 목적 : WSL2의 x86_64 환경에서 QEMU가 실행하는 AArch64 커널을 원격 디버깅하기 위해 설치
   2. 검증 방법 :
      1. QEMU를 디버그 대기 상태로 실행
      2. GDB가 QEMU에 접속
      3. _start에서 실행 흐름 추적
      4. haechix-kernel::start breakpoint 도달 확인
      5. AArch64 레지스터와 메모리 검사
3. Phase 0
   1. M00 — Workspace Initialization
      1. M00-A — Development Environment
         1. WSL2 Ubuntu 개발환경 확정
         2. QEMU 및 GDB 설치
         3. Rust 1.97.1 설치
         4. aarch64-unknown-none target 설치
         5. rust-src, llvm-tools, rustfmt, clippy 설치
         6. QEMU 기본 CPU를 Cortex-A76으로 결정

      2. M00-B — Common Library Crates
         1. kernel
         2. arch-aarch64
         3. drivers
         4. boot-protocol
         5. user-abi

      3. M00-C — Board Binary Crates
         1. QEMU virt용 haechix-qemu-virt 생성
         2. Raspberry Pi 5용 haechix-rpi5 생성
         3. Raspberry Pi 4 board는 optional/parked 처리

      4. M00-D — Root Workspace Configuration
         1. 루트 Cargo.toml 생성
         2. Cargo resolver 3 적용
         3. dev/release panic abort 설정
         4. rust-toolchain.toml에서 Rust 1.97.1 고정
         5. .gitignore에 /target/ 추가

      5. M00-E — Minimal no_std Conversion
         1. kernel에 #![no_std] 적용
         2. arch-aarch64에 #![no_std] 적용
         3. drivers에 #![no_std] 적용
         4. boot-protocol에 #![no_std] 적용
         5. user-abi에 #![no_std] 적용
         6. qemu-virt board의 std/println 제거
         7. rpi5 board의 std/println 제거

      6. M00-F — Workspace Dependency Boundaries
         1. qemu-virt board와 공통 crate 연결
         2. rpi5 board와 공통 crate 연결
         3. kernel과 boot-protocol 연결
         4. kernel과 user-abi 연결
         5. board 전용 코드의 kernel 유입 금지
         6. crate 간 순환 의존성 금지
         7. 불필요한 외부 dependency 추가 금지

      7. M00-G — Development Rules and Base Structure
         1. CLAUDE.md 생성
         2. atomic milestone 작업 규칙 작성
         3. unsafe 사용 및 safety invariant 규칙 작성
         4. 플랫폼별 코드 분리 규칙 작성
         5. scripts 디렉터리 생성
         6. docs 디렉터리 생성
         7. tests 디렉터리 생성
         8. userspace 디렉터리 생성

      8. M00-H — Workspace Validation
         1. cargo fmt --all --check
         2. 공통 library crate AArch64 빌드
         3. haechix-qemu-virt 최소 빌드
         4. haechix-rpi5 최소 빌드
         5. cargo clippy 검증
         6. dependency graph 확인
         7. std 의존성 유입 여부 확인
         8. M01 이후 기능이 구현되지 않았는지 확인

      9. M00-I — Milestone Report and Git Commit
         1. 생성 및 변경 파일 정리
         2. workspace 구조 보고
         3. crate 의존 관계 보고
         4. 실행한 검증 명령 보고
         5. 실제 검증 결과 보고
         6. 미검증 영역 명시
         7. M00 atomic commit 생성      

   2. M01 
      1. AArch64 _start 진입점
      2. interrupt masking
      3. 부트용 stack pointer 설정
      4. .bss 영역 0으로 초기화
      5. Rust haechix-kernel::start() 호출
      6. 함수가 반환되면 wfe 무한 루프
      7. 필요한 linker script와 메모리 section 배치






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

#### 검증 결과

- ELF `_start`: `0x40080000`
- QEMU CPU model: `cortex-a76`
- GDB에서 `_start` 진입 확인
- `haechix_qemu_virt::start` breakpoint 도달
- `haechix_kernel::start` 진입 확인
- Rust 함수 반환 후 `wfe` loop 진입 확인
- QEMU 및 Raspberry Pi 5 AArch64 cross-build 성공
- Raspberry Pi 5의 `_start` 경고는 해당 포팅 단계까지 의도적으로 유예

### Haechix M02: QEMU UART OK
| 작업 | 내용 |
|---|---|
| M02-A | 테스트 계약과 정확한 출력 문자열 정의 |
| M02-B | `drivers`에 PL011 모듈과 register offset 정의 |
| M02-C | volatile MMIO read/write 경계 구현 |
| M02-D | TX FIFO full bit polling 구현 |
| M02-E | byte 및 문자열 송신 구현 |
| M02-F | QEMU board에서 UART 주소 주입 |
| M02-G | board `start()`에서 문자열 출력 |
| M02-H | Rust 진입 전 EL1 FP/Advanced SIMD 접근 허용 |
| M02-I | format, QEMU 및 RPi5 회귀 빌드 |
| M02-J | QEMU에서 실제 문자열 출력 확인 |



### Haechix M04: Minimal DTB Parser
| 단계 | 내용 |
|---|---|
| M04-A | `m04.yaml` 테스트 계약 |
| M04-B | `BootInfo` 부트 계약 정의 |
| M04-C | `x0`의 DTB 주소를 Rust로 전달 |
| M04-D | FDT header와 big-endian 값 검증 |
| M04-E | structure/string block 최소 순회 |
| M04-F | compatible, memory, chosen, stdout-path 추출 |
| M04-G | UART와 interrupt controller 주소 해석 |
| M04-H | `BootInfo` 구성 및 UART 출력 |
| M04-I | 포맷·빌드·QEMU·RPi5 회귀 검증 |



| Phase | 주 실행 환경 | 목적 |
|---|---|---|
| Phase A | QEMU | 최소 부팅·UART·EL·DTB 기반 완성 |
| Phase B | Pi 5 | 최소 entry/UART만 조기 검증 |
| Phase C | QEMU | exception, timer, IRQ, allocator, MMU, scheduler |
| Phase D | QEMU | userspace, syscall, IPC, capability |
| Phase E | Pi 5 + QEMU | 완성된 microkernel 기능을 Pi 5에 포팅 |
| Phase F 이후 | Pi 5 + QEMU | SMP 및 고급 기능 |