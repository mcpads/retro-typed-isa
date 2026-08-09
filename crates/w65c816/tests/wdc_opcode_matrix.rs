//! Independent W65C816 opcode coverage derived from the architecture source.
//!
//! Normative source: The Western Design Center, *W65C816S Datasheet*,
//! March 13, 2024, Table 5-4 "Opcode Matrix", datasheet page 31.
//! https://www.westerndesigncenter.com/wdc/documentation/w65c816s.pdf
//!
//! This fixture is deliberately not generated from `OPCODE_TABLE`,
//! `InstructionForm::all()`, or the production decoder.

use typed_isa_core::{
    AccessKind, ControlAction, ControlBoundary, StaticSemantics, StorageRelation,
};
use w65c816::{
    AccumulatorView, AddressingMode, AnalysisPoint, Boundary, CpuMode, InstructionForm, Location,
    Mnemonic, RegisterLocation, StatusLocation, W65C816M8X8, W65C816M8X16, W65C816M16X8,
    W65C816M16X16, W65C816Profile, Width, WidthState, decode_bytes, encode_bytes,
};

const fn form(mnemonic: Mnemonic, mode: AddressingMode) -> InstructionForm {
    InstructionForm::new(mnemonic, mode)
}

use AddressingMode::*;
use Mnemonic::*;

const WDC_OPCODE_MATRIX: [InstructionForm; 256] = [
    // $00-$0F
    form(Brk, ImmediateByte),
    form(Ora, DirectPageIndexedIndirectX),
    form(Cop, ImmediateByte),
    form(Ora, StackRelative),
    form(Tsb, DirectPage),
    form(Ora, DirectPage),
    form(Asl, DirectPage),
    form(Ora, DirectPageIndirectLong),
    form(Php, Implied),
    form(Ora, Immediate),
    form(Asl, Accumulator),
    form(Phd, Implied),
    form(Tsb, Absolute),
    form(Ora, Absolute),
    form(Asl, Absolute),
    form(Ora, AbsoluteLong),
    // $10-$1F
    form(Bpl, Relative8),
    form(Ora, DirectPageIndirectIndexedY),
    form(Ora, DirectPageIndirect),
    form(Ora, StackRelativeIndirectIndexedY),
    form(Trb, DirectPage),
    form(Ora, DirectPageX),
    form(Asl, DirectPageX),
    form(Ora, DirectPageIndirectLongIndexedY),
    form(Clc, Implied),
    form(Ora, AbsoluteY),
    form(Inc, Accumulator),
    form(Tcs, Implied),
    form(Trb, Absolute),
    form(Ora, AbsoluteX),
    form(Asl, AbsoluteX),
    form(Ora, AbsoluteLongX),
    // $20-$2F
    form(Jsr, Absolute),
    form(And, DirectPageIndexedIndirectX),
    form(Jsl, AbsoluteLong),
    form(And, StackRelative),
    form(Bit, DirectPage),
    form(And, DirectPage),
    form(Rol, DirectPage),
    form(And, DirectPageIndirectLong),
    form(Plp, Implied),
    form(And, Immediate),
    form(Rol, Accumulator),
    form(Pld, Implied),
    form(Bit, Absolute),
    form(And, Absolute),
    form(Rol, Absolute),
    form(And, AbsoluteLong),
    // $30-$3F
    form(Bmi, Relative8),
    form(And, DirectPageIndirectIndexedY),
    form(And, DirectPageIndirect),
    form(And, StackRelativeIndirectIndexedY),
    form(Bit, DirectPageX),
    form(And, DirectPageX),
    form(Rol, DirectPageX),
    form(And, DirectPageIndirectLongIndexedY),
    form(Sec, Implied),
    form(And, AbsoluteY),
    form(Dec, Accumulator),
    form(Tsc, Implied),
    form(Bit, AbsoluteX),
    form(And, AbsoluteX),
    form(Rol, AbsoluteX),
    form(And, AbsoluteLongX),
    // $40-$4F
    form(Rti, Implied),
    form(Eor, DirectPageIndexedIndirectX),
    form(Wdm, ImmediateByte),
    form(Eor, StackRelative),
    form(Mvp, BlockMove),
    form(Eor, DirectPage),
    form(Lsr, DirectPage),
    form(Eor, DirectPageIndirectLong),
    form(Pha, Implied),
    form(Eor, Immediate),
    form(Lsr, Accumulator),
    form(Phk, Implied),
    form(Jmp, Absolute),
    form(Eor, Absolute),
    form(Lsr, Absolute),
    form(Eor, AbsoluteLong),
    // $50-$5F
    form(Bvc, Relative8),
    form(Eor, DirectPageIndirectIndexedY),
    form(Eor, DirectPageIndirect),
    form(Eor, StackRelativeIndirectIndexedY),
    form(Mvn, BlockMove),
    form(Eor, DirectPageX),
    form(Lsr, DirectPageX),
    form(Eor, DirectPageIndirectLongIndexedY),
    form(Cli, Implied),
    form(Eor, AbsoluteY),
    form(Phy, Implied),
    form(Tcd, Implied),
    form(Jml, AbsoluteLong),
    form(Eor, AbsoluteX),
    form(Lsr, AbsoluteX),
    form(Eor, AbsoluteLongX),
    // $60-$6F
    form(Rts, Implied),
    form(Adc, DirectPageIndexedIndirectX),
    form(Per, Relative16),
    form(Adc, StackRelative),
    form(Stz, DirectPage),
    form(Adc, DirectPage),
    form(Ror, DirectPage),
    form(Adc, DirectPageIndirectLong),
    form(Pla, Implied),
    form(Adc, Immediate),
    form(Ror, Accumulator),
    form(Rtl, Implied),
    form(Jmp, AbsoluteIndirect),
    form(Adc, Absolute),
    form(Ror, Absolute),
    form(Adc, AbsoluteLong),
    // $70-$7F
    form(Bvs, Relative8),
    form(Adc, DirectPageIndirectIndexedY),
    form(Adc, DirectPageIndirect),
    form(Adc, StackRelativeIndirectIndexedY),
    form(Stz, DirectPageX),
    form(Adc, DirectPageX),
    form(Ror, DirectPageX),
    form(Adc, DirectPageIndirectLongIndexedY),
    form(Sei, Implied),
    form(Adc, AbsoluteY),
    form(Ply, Implied),
    form(Tdc, Implied),
    form(Jmp, AbsoluteIndexedIndirectX),
    form(Adc, AbsoluteX),
    form(Ror, AbsoluteX),
    form(Adc, AbsoluteLongX),
    // $80-$8F
    form(Bra, Relative8),
    form(Sta, DirectPageIndexedIndirectX),
    form(Brl, Relative16),
    form(Sta, StackRelative),
    form(Sty, DirectPage),
    form(Sta, DirectPage),
    form(Stx, DirectPage),
    form(Sta, DirectPageIndirectLong),
    form(Dey, Implied),
    form(Bit, Immediate),
    form(Txa, Implied),
    form(Phb, Implied),
    form(Sty, Absolute),
    form(Sta, Absolute),
    form(Stx, Absolute),
    form(Sta, AbsoluteLong),
    // $90-$9F
    form(Bcc, Relative8),
    form(Sta, DirectPageIndirectIndexedY),
    form(Sta, DirectPageIndirect),
    form(Sta, StackRelativeIndirectIndexedY),
    form(Sty, DirectPageX),
    form(Sta, DirectPageX),
    form(Stx, DirectPageY),
    form(Sta, DirectPageIndirectLongIndexedY),
    form(Tya, Implied),
    form(Sta, AbsoluteY),
    form(Txs, Implied),
    form(Txy, Implied),
    form(Stz, Absolute),
    form(Sta, AbsoluteX),
    form(Stz, AbsoluteX),
    form(Sta, AbsoluteLongX),
    // $A0-$AF
    form(Ldy, Immediate),
    form(Lda, DirectPageIndexedIndirectX),
    form(Ldx, Immediate),
    form(Lda, StackRelative),
    form(Ldy, DirectPage),
    form(Lda, DirectPage),
    form(Ldx, DirectPage),
    form(Lda, DirectPageIndirectLong),
    form(Tay, Implied),
    form(Lda, Immediate),
    form(Tax, Implied),
    form(Plb, Implied),
    form(Ldy, Absolute),
    form(Lda, Absolute),
    form(Ldx, Absolute),
    form(Lda, AbsoluteLong),
    // $B0-$BF
    form(Bcs, Relative8),
    form(Lda, DirectPageIndirectIndexedY),
    form(Lda, DirectPageIndirect),
    form(Lda, StackRelativeIndirectIndexedY),
    form(Ldy, DirectPageX),
    form(Lda, DirectPageX),
    form(Ldx, DirectPageY),
    form(Lda, DirectPageIndirectLongIndexedY),
    form(Clv, Implied),
    form(Lda, AbsoluteY),
    form(Tsx, Implied),
    form(Tyx, Implied),
    form(Ldy, AbsoluteX),
    form(Lda, AbsoluteX),
    form(Ldx, AbsoluteY),
    form(Lda, AbsoluteLongX),
    // $C0-$CF
    form(Cpy, Immediate),
    form(Cmp, DirectPageIndexedIndirectX),
    form(Rep, ImmediateByte),
    form(Cmp, StackRelative),
    form(Cpy, DirectPage),
    form(Cmp, DirectPage),
    form(Dec, DirectPage),
    form(Cmp, DirectPageIndirectLong),
    form(Iny, Implied),
    form(Cmp, Immediate),
    form(Dex, Implied),
    form(Wai, Implied),
    form(Cpy, Absolute),
    form(Cmp, Absolute),
    form(Dec, Absolute),
    form(Cmp, AbsoluteLong),
    // $D0-$DF
    form(Bne, Relative8),
    form(Cmp, DirectPageIndirectIndexedY),
    form(Cmp, DirectPageIndirect),
    form(Cmp, StackRelativeIndirectIndexedY),
    form(Pei, DirectPage),
    form(Cmp, DirectPageX),
    form(Dec, DirectPageX),
    form(Cmp, DirectPageIndirectLongIndexedY),
    form(Cld, Implied),
    form(Cmp, AbsoluteY),
    form(Phx, Implied),
    form(Stp, Implied),
    form(Jml, AbsoluteIndirectLong),
    form(Cmp, AbsoluteX),
    form(Dec, AbsoluteX),
    form(Cmp, AbsoluteLongX),
    // $E0-$EF
    form(Cpx, Immediate),
    form(Sbc, DirectPageIndexedIndirectX),
    form(Sep, ImmediateByte),
    form(Sbc, StackRelative),
    form(Cpx, DirectPage),
    form(Sbc, DirectPage),
    form(Inc, DirectPage),
    form(Sbc, DirectPageIndirectLong),
    form(Inx, Implied),
    form(Sbc, Immediate),
    form(Nop, Implied),
    form(Xba, Implied),
    form(Cpx, Absolute),
    form(Sbc, Absolute),
    form(Inc, Absolute),
    form(Sbc, AbsoluteLong),
    // $F0-$FF
    form(Beq, Relative8),
    form(Sbc, DirectPageIndirectIndexedY),
    form(Sbc, DirectPageIndirect),
    form(Sbc, StackRelativeIndirectIndexedY),
    form(Pea, Absolute),
    form(Sbc, DirectPageX),
    form(Inc, DirectPageX),
    form(Sbc, DirectPageIndirectLongIndexedY),
    form(Sed, Implied),
    form(Sbc, AbsoluteY),
    form(Plx, Implied),
    form(Xce, Implied),
    form(Jsr, AbsoluteIndexedIndirectX),
    form(Sbc, AbsoluteX),
    form(Inc, AbsoluteX),
    form(Sbc, AbsoluteLongX),
];

// Program-sequence byte counts printed in Table 5-4 for M=1 and X=1.
const WDC_M8_X8_LENGTHS: [usize; 256] = [
    2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $00-$0F
    2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $10-$1F
    3, 2, 4, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $20-$2F
    2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $30-$3F
    1, 2, 2, 2, 3, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $40-$4F
    2, 2, 2, 2, 3, 2, 2, 2, 1, 3, 1, 1, 4, 3, 3, 4, // $50-$5F
    1, 2, 3, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $60-$6F
    2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $70-$7F
    2, 2, 3, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $80-$8F
    2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $90-$9F
    2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $A0-$AF
    2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $B0-$BF
    2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $C0-$CF
    2, 2, 2, 2, 2, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $D0-$DF
    2, 2, 2, 2, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 3, 4, // $E0-$EF
    2, 2, 2, 2, 3, 2, 2, 2, 1, 3, 1, 1, 3, 3, 3, 4, // $F0-$FF
];

#[test]
fn every_wdc_opcode_has_the_manual_form_length_and_exact_lowering() {
    for opcode in 0_u8..=u8::MAX {
        let expected_form = WDC_OPCODE_MATRIX[usize::from(opcode)];
        let expected_len = WDC_M8_X8_LENGTHS[usize::from(opcode)];
        let source = [opcode, 0x41, 0x42, 0x43];

        let decoded = decode_bytes(&source, WidthState::M8_X8)
            .unwrap_or_else(|error| panic!("WDC opcode ${opcode:02X} did not decode: {error}"));

        assert_eq!(
            decoded.instruction.form(),
            expected_form,
            "WDC opcode ${opcode:02X} has the wrong instruction form"
        );
        assert_eq!(
            decoded.len, expected_len,
            "WDC opcode ${opcode:02X} has the wrong M=1/X=1 length"
        );
        assert_eq!(
            encode_bytes(&decoded.instruction, WidthState::M8_X8).unwrap(),
            source[..expected_len],
            "WDC opcode ${opcode:02X} did not lower to its exact selector"
        );
    }
}

#[test]
fn manual_immediate_width_classes_follow_m_and_x() {
    // WDC section 6.3.3.3 requires separate accumulator and index width state.
    const ACCUMULATOR_IMMEDIATE_OPCODES: [u8; 7] = [0x09, 0x29, 0x49, 0x69, 0x89, 0xA9, 0xE9];
    const INDEX_IMMEDIATE_OPCODES: [u8; 4] = [0xA0, 0xA2, 0xC0, 0xE0];

    for opcode in ACCUMULATOR_IMMEDIATE_OPCODES {
        assert_eq!(
            decode_bytes(&[opcode, 1], WidthState::M8_X16).unwrap().len,
            2
        );
        assert_eq!(
            decode_bytes(&[opcode, 1, 2], WidthState::M16_X8)
                .unwrap()
                .len,
            3
        );
    }
    for opcode in INDEX_IMMEDIATE_OPCODES {
        assert_eq!(
            decode_bytes(&[opcode, 1], WidthState::M16_X8).unwrap().len,
            2
        );
        assert_eq!(
            decode_bytes(&[opcode, 1, 2], WidthState::M8_X16)
                .unwrap()
                .len,
            3
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManualFlowClass {
    Continue,
    ConditionalTransfer,
    Transfer,
    LinkedTransfer,
    Return,
    ExceptionReturn,
    Trap,
    Wait,
    Stop,
}

fn manual_flow_class(opcode: u8) -> ManualFlowClass {
    match opcode {
        0x10 | 0x30 | 0x50 | 0x70 | 0x90 | 0xb0 | 0xd0 | 0xf0 => {
            ManualFlowClass::ConditionalTransfer
        }
        0x4c | 0x5c | 0x6c | 0x7c | 0x80 | 0x82 | 0xdc => ManualFlowClass::Transfer,
        0x20 | 0x22 | 0xfc => ManualFlowClass::LinkedTransfer,
        0x60 | 0x6b => ManualFlowClass::Return,
        0x40 => ManualFlowClass::ExceptionReturn,
        0x00 | 0x02 => ManualFlowClass::Trap,
        0xcb => ManualFlowClass::Wait,
        0xdb => ManualFlowClass::Stop,
        _ => ManualFlowClass::Continue,
    }
}

fn verify_manual_semantic_denominator<P>(mode: CpuMode)
where
    P: W65C816Profile
        + StaticSemantics<AnalysisPoint = AnalysisPoint, Location = Location, Boundary = Boundary>,
{
    for opcode in 0_u8..=u8::MAX {
        let decoded = decode_bytes(&[opcode, 0x41, 0x42, 0x43], P::WIDTHS)
            .unwrap_or_else(|error| panic!("manual opcode ${opcode:02X} did not decode: {error}"));
        let at = AnalysisPoint::new(w65c816::CodeLocation::new(0x12, 0x8000), mode);
        let semantics = P::semantics(&decoded.instruction, &at).unwrap_or_else(|error| {
            panic!("manual opcode ${opcode:02X} has no semantics: {error}")
        });

        let actual_flow = match &semantics.control_flow.action {
            ControlAction::Continue => ManualFlowClass::Continue,
            ControlAction::Transfer { .. } if semantics.control_flow.fallthrough.is_some() => {
                ManualFlowClass::ConditionalTransfer
            }
            ControlAction::Transfer { .. } => ManualFlowClass::Transfer,
            ControlAction::LinkedTransfer { .. } => ManualFlowClass::LinkedTransfer,
            ControlAction::Return { .. } => ManualFlowClass::Return,
            ControlAction::ExceptionReturn { .. } => ManualFlowClass::ExceptionReturn,
            ControlAction::Boundary(ControlBoundary::Trap { .. }) => ManualFlowClass::Trap,
            ControlAction::Boundary(ControlBoundary::Wait { .. }) => ManualFlowClass::Wait,
            ControlAction::Boundary(ControlBoundary::Stop { .. }) => ManualFlowClass::Stop,
            ControlAction::Boundary(ControlBoundary::ProfileExit { .. }) => {
                panic!("W65C816 opcode ${opcode:02X} unexpectedly exits its profile")
            }
        };
        assert_eq!(
            actual_flow,
            manual_flow_class(opcode),
            "WDC opcode ${opcode:02X} has the wrong control-flow class"
        );

        let form = WDC_OPCODE_MATRIX[usize::from(opcode)];
        if manual_form_reads_or_writes_data_memory(form) {
            assert!(
                semantics
                    .location_accesses
                    .iter()
                    .any(|access| matches!(access.location, Location::Memory(_))),
                "WDC opcode ${opcode:02X} omits its manual data-memory access"
            );
        }
        for status in manual_status_writes(form) {
            assert!(
                semantics.location_accesses.iter().any(|access| {
                    access.kind == AccessKind::Write && access.location == Location::Status(*status)
                }),
                "WDC opcode ${opcode:02X} omits manual status write {status:?}"
            );
        }
    }
}

#[test]
fn every_manual_selector_has_total_static_semantics_in_every_exact_width_profile() {
    verify_manual_semantic_denominator::<W65C816M8X8>(CpuMode::NATIVE_M8_X8);
    verify_manual_semantic_denominator::<W65C816M8X16>(CpuMode::NATIVE_M8_X16);
    verify_manual_semantic_denominator::<W65C816M16X8>(CpuMode::NATIVE_M16_X8);
    verify_manual_semantic_denominator::<W65C816M16X16>(CpuMode::NATIVE_M16_X16);
    verify_manual_semantic_denominator::<W65C816M8X8>(CpuMode::Emulation);
}

fn manual_form_reads_or_writes_data_memory(form: InstructionForm) -> bool {
    use AddressingMode::*;
    if matches!(
        form.mnemonic,
        Mnemonic::Jmp
            | Mnemonic::Jml
            | Mnemonic::Jsr
            | Mnemonic::Jsl
            | Mnemonic::Pea
            | Mnemonic::Per
    ) {
        return matches!(
            form.mode,
            AbsoluteIndirect | AbsoluteIndexedIndirectX | AbsoluteIndirectLong
        );
    }
    !matches!(
        form.mode,
        Implied | Accumulator | Immediate | ImmediateByte | Relative8 | Relative16 | BlockMove
    )
}

fn manual_status_writes(form: InstructionForm) -> &'static [StatusLocation] {
    use Mnemonic::*;
    use StatusLocation::*;
    match form.mnemonic {
        Adc | Sbc => &[Negative, Overflow, Zero, Carry],
        And | Asl | Dec | Eor | Inc | Lda | Ldx | Ldy | Lsr | Ora | Rol | Ror | Tax | Tay | Tcd
        | Tdc | Tsc | Tsx | Txa | Txy | Tya | Tyx | Xba => &[Negative, Zero],
        Bit if form.mode == AddressingMode::Immediate => &[Zero],
        Bit => &[Negative, Overflow, Zero],
        Cmp | Cpx | Cpy => &[Negative, Zero, Carry],
        Trb | Tsb => &[Zero],
        Clc | Sec => &[Carry],
        Cld | Sed => &[Decimal],
        Cli | Sei => &[InterruptDisable],
        Clv => &[Overflow],
        Brk | Cop => &[Decimal, InterruptDisable],
        Pla | Plb | Pld | Plx | Ply => &[Negative, Zero],
        _ => &[],
    }
}

#[test]
fn source_defined_aliases_and_mode_transitions_remain_sound() {
    let full = Location::Register(RegisterLocation::Accumulator(AccumulatorView::Full));
    let low = Location::Register(RegisterLocation::Accumulator(AccumulatorView::Low));
    let high = Location::Register(RegisterLocation::Accumulator(AccumulatorView::High));
    assert_eq!(
        W65C816M16X16::storage_relation(&full, &low),
        StorageRelation::Overlapping
    );
    assert_eq!(
        W65C816M16X16::storage_relation(&full, &high),
        StorageRelation::Overlapping
    );
    assert_eq!(
        W65C816M16X16::storage_relation(&low, &high),
        StorageRelation::Disjoint
    );
    assert_eq!(
        W65C816M16X16::storage_relation(
            &Location::Register(RegisterLocation::X(Width::Eight)),
            &Location::Register(RegisterLocation::X(Width::Sixteen)),
        ),
        StorageRelation::Overlapping
    );

    let at = AnalysisPoint::new(
        w65c816::CodeLocation::new(0x12, 0x8000),
        CpuMode::NATIVE_M16_X16,
    );
    let sep = decode_bytes(&[0xe2, 0x30], WidthState::M16_X16).unwrap();
    let semantics = W65C816M16X16::semantics(&sep.instruction, &at).unwrap();
    assert_eq!(
        semantics
            .control_flow
            .fallthrough
            .expect("SEP continues")
            .mode(),
        Some(CpuMode::NATIVE_M8_X8)
    );

    let xce = decode_bytes(&[0xfb], WidthState::M16_X16).unwrap();
    let semantics = W65C816M16X16::semantics(&xce.instruction, &at).unwrap();
    assert_eq!(
        semantics
            .control_flow
            .fallthrough
            .expect("XCE reaches a sequence boundary")
            .mode(),
        None
    );
}
