# Retro Typed ISA

레트로 게임 패치 도구가 실행 코드를 안전하게 분석하고 생성할 수 있도록 CPU별 typed ISA 프로필을 Rust 라이브러리로 제공한다.

각 CPU 변형은 독립된 crate가 책임지며, typed instruction과 operand, byte decode와 encode, 배치 인식 checked assembler를 제공한다. 명령 실행기, CPU 상태 전이, pipeline, cycle timing과 simulator는 범위에 포함하지 않는다.

모든 crate는 선언한 정적 ISA 프로필 전체를 분모로 검증하며 Cargo 배포는 현재 비활성화되어 있다.
