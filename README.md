# Retro Typed ISA

레트로 게임 패치 도구가 실행 코드를 안전하게 분석하고 생성할 수 있도록 CPU별 typed ISA 프로필을 Rust 라이브러리로 제공한다.

각 CPU 변형은 독립된 crate가 책임지며, typed instruction과 operand, byte decode와 encode, 배치 인식 checked assembler를 제공한다. 명령 실행기, CPU 상태 전이, pipeline, cycle timing과 simulator는 범위에 포함하지 않는다.

CPU 독립 `expected-write` crate는 불변 원본 선조건, 쓰기 소유권, 보호 영역과 typed machine-code 검산을 담당한다.

## 지원 crate

- CPU 프로필: `w65c816`, `psx-r3000a`, `huc6280`, `rp2a03`, `sh2`, `z80`, `sh4`, `m68000`, `v30`
- 최종 byte 검증: `expected-write`

모든 crate는 선언한 정적 ISA 프로필 전체를 분모로 검증하며 Cargo 배포는 현재 비활성화되어 있다.

## 라이선스

GNU General Public License version 2 only (`GPL-2.0-only`).
