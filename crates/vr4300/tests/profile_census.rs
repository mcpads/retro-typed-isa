use std::collections::BTreeSet;

use vr4300::{
    CacheOperation, Coprocessor, Coprocessor2ControlRegister, Coprocessor2Register,
    CoprocessorBranchCondition, CoprocessorMemoryOperation, CoprocessorMemoryRegister,
    Cp0Operation, Cp0Register, EqualityCondition, FixedFormat, FloatingBinaryOperation,
    FloatingComparePredicate, FloatingFormat, FloatingRegister, FloatingRounding,
    FloatingUnaryOperation, FpuControlRegister, GeneralRegister, ImmediateArithmeticOperation,
    Instruction, LogicalImmediateOperation, MemoryOperation, MultiplyDivideOperation,
    NumericFormat, ShiftOperation, SignCondition, SpecialRegister, ThreeRegisterOperation,
    TrapCondition, decode_word, encode_word,
};

const ADDRESS: u64 = 0x8000_0000;

#[test]
fn appendix_opcode_maps_have_an_independent_selector_denominator() {
    // NEC U10504EJ7V0UM00, Figure 16-1, cross-checked against the original
    // MIPS R4000 manual, Figure A-2. These sets are intentionally written
    // independently of the production decoder and its semantic types.
    for primary in 0u8..64 {
        let expected = !matches!(primary, 0x13 | 0x1c..=0x1f | 0x33 | 0x3b);
        assert_eq!(
            decode_word(u32::from(primary) << 26, ADDRESS).is_ok(),
            expected,
            "primary opcode {primary:#04x}"
        );
    }

    for function in 0u8..64 {
        let expected = matches!(
            function,
            0x00 | 0x02..=0x04 | 0x06..=0x09 | 0x0c..=0x0d | 0x0f..=0x14
                | 0x16..=0x27 | 0x2a..=0x34 | 0x36 | 0x38 | 0x3a..=0x3c | 0x3e..=0x3f
        );
        assert_eq!(
            decode_word(u32::from(function), ADDRESS).is_ok(),
            expected,
            "SPECIAL function {function:#04x}"
        );
    }

    for selector in 0u8..32 {
        let expected = matches!(selector, 0x00..=0x03 | 0x08..=0x0c | 0x0e | 0x10..=0x13);
        let word = 0x0400_0000 | u32::from(selector) << 16;
        assert_eq!(
            decode_word(word, ADDRESS).is_ok(),
            expected,
            "REGIMM selector {selector:#04x}"
        );
    }

    for function in 0u8..64 {
        let expected = matches!(function, 0x01 | 0x02 | 0x06 | 0x08 | 0x18);
        let word = 0x4200_0000 | u32::from(function);
        assert_eq!(
            decode_word(word, ADDRESS).is_ok(),
            expected,
            "CP0 function {function:#04x}"
        );
    }
}

#[test]
fn legacy_rfe_is_outside_the_vr4300_cp0_table() {
    // NEC VR4300 page 435 and the R4000 manual page A-77 define ERET at
    // function 0x18. They do not include the pre-MIPS-III RFE operation at
    // function 0x10.
    assert!(decode_word(0x4200_0010, ADDRESS).is_err());
    assert!(decode_word(0x4200_0018, ADDRESS).is_ok());
}

#[test]
fn coprocessor_subtables_and_register_maps_fail_closed() {
    for sub in 0u8..32 {
        let cp0_expected = matches!(sub, 0 | 1 | 4 | 5 | 8);
        let cp1_expected = matches!(sub, 0 | 1 | 2 | 4 | 5 | 6 | 8 | 16 | 17);
        let cp2_expected = matches!(sub, 0 | 1 | 2 | 4 | 5 | 6 | 8);
        for (primary, expected, name) in [
            (0x10u8, cp0_expected, "CP0"),
            (0x11u8, cp1_expected, "CP1"),
            (0x12u8, cp2_expected, "CP2"),
        ] {
            let word = u32::from(primary) << 26 | u32::from(sub) << 21;
            assert_eq!(
                decode_word(word, ADDRESS).is_ok(),
                expected,
                "{name} sub {sub:#04x}"
            );
        }
    }

    for branch_selector in 0u8..32 {
        let expected = branch_selector < 4;
        for primary in [0x10u8, 0x11, 0x12] {
            let word = u32::from(primary) << 26 | 8 << 21 | u32::from(branch_selector) << 16;
            assert_eq!(
                decode_word(word, ADDRESS).is_ok(),
                expected,
                "COP branch primary {primary:#04x}, selector {branch_selector:#04x}"
            );
        }
    }

    for register in 0u8..32 {
        let expected_cp0 = matches!(register, 0..=6 | 8..=20 | 26..=30);
        let cp0_word = 0x4000_0000 | u32::from(register) << 11;
        assert_eq!(
            decode_word(cp0_word, ADDRESS).is_ok(),
            expected_cp0,
            "CP0 register {register}"
        );

        let expected_fpu_control = matches!(register, 0 | 31);
        let fpu_control_word = 0x4440_0000 | u32::from(register) << 11;
        assert_eq!(
            decode_word(fpu_control_word, ADDRESS).is_ok(),
            expected_fpu_control,
            "FPU control register {register}"
        );
    }

    // CP2 CO is an implementation-defined extension slot, not a typed VR4300
    // operation. The profile rejects it instead of exposing raw executable bits.
    assert!(decode_word(0x4a00_0000, ADDRESS).is_err());
}

#[test]
fn fpu_format_matrix_matches_tables_17_1_and_17_3() {
    // NEC Tables 17-1 and Figure 17-3, independently corroborated by the MIPS
    // R4000 manual's Table B-1 and Figure B-3.
    for format in 16u8..32 {
        for function in 0u8..64 {
            let expected = match format {
                16 => matches!(function, 0x00..=0x0f | 0x21 | 0x24 | 0x25 | 0x30..=0x3f),
                17 => matches!(function, 0x00..=0x0f | 0x20 | 0x24 | 0x25 | 0x30..=0x3f),
                20 | 21 => matches!(function, 0x20 | 0x21),
                _ => false,
            };
            let word = 0x4400_0000 | u32::from(format) << 21 | u32::from(function);
            assert_eq!(
                decode_word(word, ADDRESS).is_ok(),
                expected,
                "FPU format {format}, function {function:#04x}"
            );
        }
    }

    assert!(
        decode_word(0x4601_0004, ADDRESS).is_err(),
        "SQRT ft must be zero"
    );
    assert!(
        decode_word(0x4601_0020, ADDRESS).is_err(),
        "CVT.S ft must be zero"
    );
    assert!(
        decode_word(0x4600_0830, ADDRESS).is_err(),
        "compare fd must be zero"
    );
}

#[test]
fn every_source_listed_semantic_form_has_checked_lowering() {
    let vectors = source_form_vectors();
    let forms = vectors
        .iter()
        .map(|instruction| instruction.form())
        .collect::<BTreeSet<_>>();

    // The count is derived below from Figure 16-1, Tables 17-1 and 17-2, and
    // the source-defined operand distinctions, not from a production iterator.
    assert_eq!(forms.len(), 225);
    assert_eq!(vectors.len(), 225);

    for instruction in vectors {
        let word = encode_word(&instruction, ADDRESS)
            .unwrap_or_else(|error| panic!("failed to encode {:?}: {error}", instruction.form()));
        let decoded = decode_word(word, ADDRESS)
            .unwrap_or_else(|error| panic!("failed to decode {:?}: {error}", instruction.form()));
        assert_eq!(
            decoded,
            instruction,
            "round trip changed {:?}",
            instruction.form()
        );
    }
}

fn source_form_vectors() -> Vec<Instruction> {
    let mut vectors = Vec::new();
    let r0 = GeneralRegister::ZERO;
    let r1 = GeneralRegister::AT;
    let f0 = FloatingRegister::new(0).unwrap();
    let f2 = FloatingRegister::new(2).unwrap();

    for operation in [
        ThreeRegisterOperation::Add,
        ThreeRegisterOperation::AddUnsigned,
        ThreeRegisterOperation::Subtract,
        ThreeRegisterOperation::SubtractUnsigned,
        ThreeRegisterOperation::And,
        ThreeRegisterOperation::Or,
        ThreeRegisterOperation::ExclusiveOr,
        ThreeRegisterOperation::Nor,
        ThreeRegisterOperation::SetLessThan,
        ThreeRegisterOperation::SetLessThanUnsigned,
        ThreeRegisterOperation::DoublewordAdd,
        ThreeRegisterOperation::DoublewordAddUnsigned,
        ThreeRegisterOperation::DoublewordSubtract,
        ThreeRegisterOperation::DoublewordSubtractUnsigned,
    ] {
        vectors.push(Instruction::ThreeRegister {
            operation,
            destination: r1,
            left: r1,
            right: r0,
        });
    }
    for operation in [
        ImmediateArithmeticOperation::Add,
        ImmediateArithmeticOperation::AddUnsigned,
        ImmediateArithmeticOperation::SetLessThan,
        ImmediateArithmeticOperation::SetLessThanUnsigned,
        ImmediateArithmeticOperation::DoublewordAdd,
        ImmediateArithmeticOperation::DoublewordAddUnsigned,
    ] {
        vectors.push(Instruction::ImmediateArithmetic {
            operation,
            destination: r1,
            source: r0,
            immediate: -1,
        });
    }
    for operation in [
        LogicalImmediateOperation::And,
        LogicalImmediateOperation::Or,
        LogicalImmediateOperation::ExclusiveOr,
    ] {
        vectors.push(Instruction::LogicalImmediate {
            operation,
            destination: r1,
            source: r0,
            immediate: 0x1234,
        });
    }
    vectors.push(Instruction::LoadUpperImmediate {
        destination: r1,
        immediate: 0x1234,
    });

    for operation in [
        ShiftOperation::LeftLogical,
        ShiftOperation::RightLogical,
        ShiftOperation::RightArithmetic,
        ShiftOperation::DoublewordLeftLogical,
        ShiftOperation::DoublewordRightLogical,
        ShiftOperation::DoublewordRightArithmetic,
    ] {
        vectors.push(Instruction::ShiftImmediate {
            operation,
            destination: r1,
            source: r1,
            amount: if matches!(
                operation,
                ShiftOperation::DoublewordLeftLogical
                    | ShiftOperation::DoublewordRightLogical
                    | ShiftOperation::DoublewordRightArithmetic
            ) {
                40
            } else {
                8
            },
        });
        if matches!(
            operation,
            ShiftOperation::DoublewordLeftLogical
                | ShiftOperation::DoublewordRightLogical
                | ShiftOperation::DoublewordRightArithmetic
        ) {
            vectors.push(Instruction::ShiftImmediate {
                operation,
                destination: r1,
                source: r1,
                amount: 8,
            });
        }
        vectors.push(Instruction::ShiftVariable {
            operation,
            destination: r1,
            source: r1,
            amount_register: r1,
        });
    }

    for operation in [
        MultiplyDivideOperation::Multiply,
        MultiplyDivideOperation::MultiplyUnsigned,
        MultiplyDivideOperation::Divide,
        MultiplyDivideOperation::DivideUnsigned,
        MultiplyDivideOperation::DoublewordMultiply,
        MultiplyDivideOperation::DoublewordMultiplyUnsigned,
        MultiplyDivideOperation::DoublewordDivide,
        MultiplyDivideOperation::DoublewordDivideUnsigned,
    ] {
        vectors.push(Instruction::MultiplyDivide {
            operation,
            left: r1,
            right: r1,
        });
    }
    for special in [SpecialRegister::Hi, SpecialRegister::Lo] {
        vectors.push(Instruction::MoveFromSpecial {
            source: special,
            destination: r1,
        });
        vectors.push(Instruction::MoveToSpecial {
            source: r1,
            destination: special,
        });
    }

    for operation in [
        MemoryOperation::LoadByte,
        MemoryOperation::LoadHalfword,
        MemoryOperation::LoadWordLeft,
        MemoryOperation::LoadWord,
        MemoryOperation::LoadByteUnsigned,
        MemoryOperation::LoadHalfwordUnsigned,
        MemoryOperation::LoadWordRight,
        MemoryOperation::LoadWordUnsigned,
        MemoryOperation::LoadDoublewordLeft,
        MemoryOperation::LoadDoublewordRight,
        MemoryOperation::LoadLinked,
        MemoryOperation::LoadLinkedDoubleword,
        MemoryOperation::LoadDoubleword,
        MemoryOperation::StoreByte,
        MemoryOperation::StoreHalfword,
        MemoryOperation::StoreWordLeft,
        MemoryOperation::StoreWord,
        MemoryOperation::StoreDoublewordLeft,
        MemoryOperation::StoreDoublewordRight,
        MemoryOperation::StoreWordRight,
        MemoryOperation::StoreConditional,
        MemoryOperation::StoreConditionalDoubleword,
        MemoryOperation::StoreDoubleword,
    ] {
        vectors.push(Instruction::Memory {
            operation,
            register: r1,
            base: r0,
            offset: -4,
        });
    }

    for condition in [EqualityCondition::Equal, EqualityCondition::NotEqual] {
        for likely in [false, true] {
            vectors.push(Instruction::EqualityBranch {
                condition,
                likely,
                left: r1,
                right: r0,
                target: ADDRESS + 4,
            });
        }
    }
    for condition in [
        SignCondition::LessThanZero,
        SignCondition::GreaterThanOrEqualZero,
    ] {
        for likely in [false, true] {
            for link in [false, true] {
                vectors.push(Instruction::SignBranch {
                    condition,
                    likely,
                    link,
                    source: r1,
                    target: ADDRESS + 4,
                });
            }
        }
    }
    for condition in [
        SignCondition::LessThanOrEqualZero,
        SignCondition::GreaterThanZero,
    ] {
        for likely in [false, true] {
            vectors.push(Instruction::SignBranch {
                condition,
                likely,
                link: false,
                source: r1,
                target: ADDRESS + 4,
            });
        }
    }
    for link in [false, true] {
        vectors.push(Instruction::Jump {
            link,
            target: ADDRESS + 0x1000,
        });
        vectors.push(Instruction::JumpRegister {
            link: link.then_some(GeneralRegister::RA),
            target: r1,
        });
    }
    vectors.extend([
        Instruction::SystemCall { code: 1 },
        Instruction::Breakpoint { code: 2 },
        Instruction::Synchronize,
    ]);
    for condition in [
        TrapCondition::GreaterThanOrEqual,
        TrapCondition::GreaterThanOrEqualUnsigned,
        TrapCondition::LessThan,
        TrapCondition::LessThanUnsigned,
        TrapCondition::Equal,
        TrapCondition::NotEqual,
    ] {
        vectors.push(Instruction::TrapRegister {
            condition,
            left: r1,
            right: r0,
            code: 3,
        });
        vectors.push(Instruction::TrapImmediate {
            condition,
            source: r1,
            immediate: -2,
        });
    }

    for doubleword in [false, true] {
        vectors.push(Instruction::MoveFromCp0 {
            doubleword,
            destination: r1,
            source: Cp0Register::Status,
        });
        vectors.push(Instruction::MoveToCp0 {
            doubleword,
            source: r1,
            destination: Cp0Register::Status,
        });
        vectors.push(Instruction::MoveFromFpu {
            doubleword,
            destination: r1,
            source: f0,
        });
        vectors.push(Instruction::MoveToFpu {
            doubleword,
            source: r1,
            destination: f0,
        });
        vectors.push(Instruction::MoveFromCoprocessor2 {
            doubleword,
            destination: r1,
            source: Coprocessor2Register::new(1).unwrap(),
        });
        vectors.push(Instruction::MoveToCoprocessor2 {
            doubleword,
            source: r1,
            destination: Coprocessor2Register::new(1).unwrap(),
        });
    }
    vectors.extend([
        Instruction::MoveControlFromFpu {
            destination: r1,
            source: FpuControlRegister::ControlStatus,
        },
        Instruction::MoveControlToFpu {
            source: r1,
            destination: FpuControlRegister::ControlStatus,
        },
        Instruction::MoveControlFromCoprocessor2 {
            destination: r1,
            source: Coprocessor2ControlRegister::new(1).unwrap(),
        },
        Instruction::MoveControlToCoprocessor2 {
            source: r1,
            destination: Coprocessor2ControlRegister::new(1).unwrap(),
        },
    ]);

    for coprocessor in [
        Coprocessor::SystemControl,
        Coprocessor::FloatingPoint,
        Coprocessor::Coprocessor2,
    ] {
        for condition in [
            CoprocessorBranchCondition::False,
            CoprocessorBranchCondition::True,
        ] {
            for likely in [false, true] {
                vectors.push(Instruction::CoprocessorBranch {
                    coprocessor,
                    condition,
                    likely,
                    target: ADDRESS + 4,
                });
            }
        }
    }
    for operation in [
        CoprocessorMemoryOperation::LoadWord,
        CoprocessorMemoryOperation::StoreWord,
        CoprocessorMemoryOperation::LoadDoubleword,
        CoprocessorMemoryOperation::StoreDoubleword,
    ] {
        vectors.push(Instruction::CoprocessorMemory {
            operation,
            register: CoprocessorMemoryRegister::FloatingPoint(f0),
            base: r0,
            offset: 0,
        });
        vectors.push(Instruction::CoprocessorMemory {
            operation,
            register: CoprocessorMemoryRegister::Coprocessor2(
                Coprocessor2Register::new(0).unwrap(),
            ),
            base: r0,
            offset: 0,
        });
    }
    for operation in [
        Cp0Operation::ReadIndexedTlbEntry,
        Cp0Operation::WriteIndexedTlbEntry,
        Cp0Operation::WriteRandomTlbEntry,
        Cp0Operation::ProbeTlb,
        Cp0Operation::ReturnFromException,
    ] {
        vectors.push(Instruction::Cp0(operation));
    }
    vectors.push(Instruction::Cache {
        operation: CacheOperation::new(0).unwrap(),
        base: r0,
        offset: 0,
    });

    for format in [FloatingFormat::Single, FloatingFormat::Double] {
        for operation in [
            FloatingBinaryOperation::Add,
            FloatingBinaryOperation::Subtract,
            FloatingBinaryOperation::Multiply,
            FloatingBinaryOperation::Divide,
        ] {
            vectors.push(Instruction::FloatingBinary {
                operation,
                format,
                destination: f0,
                left: f0,
                right: f2,
            });
        }
        for operation in [
            FloatingUnaryOperation::SquareRoot,
            FloatingUnaryOperation::AbsoluteValue,
            FloatingUnaryOperation::Move,
            FloatingUnaryOperation::Negate,
        ] {
            vectors.push(Instruction::FloatingUnary {
                operation,
                format,
                destination: f0,
                source: f2,
            });
        }
        for destination_format in [FixedFormat::Word, FixedFormat::Longword] {
            for rounding in [
                FloatingRounding::NearestEven,
                FloatingRounding::TowardZero,
                FloatingRounding::TowardPositiveInfinity,
                FloatingRounding::TowardNegativeInfinity,
            ] {
                vectors.push(Instruction::FloatingRoundToInteger {
                    rounding,
                    destination_format,
                    source_format: format,
                    destination: f0,
                    source: f2,
                });
            }
        }
        for predicate in [
            FloatingComparePredicate::False,
            FloatingComparePredicate::Unordered,
            FloatingComparePredicate::Equal,
            FloatingComparePredicate::UnorderedOrEqual,
            FloatingComparePredicate::OrderedLessThan,
            FloatingComparePredicate::UnorderedOrLessThan,
            FloatingComparePredicate::OrderedLessThanOrEqual,
            FloatingComparePredicate::UnorderedOrLessThanOrEqual,
            FloatingComparePredicate::SignalingFalse,
            FloatingComparePredicate::NotGreaterOrLessOrEqual,
            FloatingComparePredicate::SignalingEqual,
            FloatingComparePredicate::NotGreaterOrLess,
            FloatingComparePredicate::LessThan,
            FloatingComparePredicate::NotGreaterOrEqual,
            FloatingComparePredicate::LessThanOrEqual,
            FloatingComparePredicate::NotGreaterThan,
        ] {
            vectors.push(Instruction::FloatingCompare {
                predicate,
                format,
                left: f0,
                right: f2,
            });
        }
    }

    for (source_format, destination_format) in [
        (
            NumericFormat::Floating(FloatingFormat::Double),
            NumericFormat::Floating(FloatingFormat::Single),
        ),
        (
            NumericFormat::Fixed(FixedFormat::Word),
            NumericFormat::Floating(FloatingFormat::Single),
        ),
        (
            NumericFormat::Fixed(FixedFormat::Longword),
            NumericFormat::Floating(FloatingFormat::Single),
        ),
        (
            NumericFormat::Floating(FloatingFormat::Single),
            NumericFormat::Floating(FloatingFormat::Double),
        ),
        (
            NumericFormat::Fixed(FixedFormat::Word),
            NumericFormat::Floating(FloatingFormat::Double),
        ),
        (
            NumericFormat::Fixed(FixedFormat::Longword),
            NumericFormat::Floating(FloatingFormat::Double),
        ),
        (
            NumericFormat::Floating(FloatingFormat::Single),
            NumericFormat::Fixed(FixedFormat::Word),
        ),
        (
            NumericFormat::Floating(FloatingFormat::Double),
            NumericFormat::Fixed(FixedFormat::Word),
        ),
        (
            NumericFormat::Floating(FloatingFormat::Single),
            NumericFormat::Fixed(FixedFormat::Longword),
        ),
        (
            NumericFormat::Floating(FloatingFormat::Double),
            NumericFormat::Fixed(FixedFormat::Longword),
        ),
    ] {
        vectors.push(Instruction::FloatingConvert {
            destination_format,
            source_format,
            destination: f0,
            source: f2,
        });
    }

    vectors
}
