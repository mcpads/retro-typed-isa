# CPU Profile Semantics

This document applies the common [static semantics protocol](SEMANTICS.md) to every CPU profile in the workspace. It records architecture-specific design decisions without redefining the common concepts.

The unit of ownership is an exact CPU protocol profile, not a console. A console or game consumer supplies memory maps, executable-region ownership, ABI conventions, and device behavior outside the CPU architecture.

## Common profile requirements

Every profile design answers the same questions:

- Which state selects instruction identity before decode?
- Which non-value semantic context selects physical storage or successor state?
- What constitutes a direct analysis point and an indirect code target?
- Which architectural storage locations can an instruction read or write?
- Which control transfers, delay slots, waits, stops, and profile exits exist?
- Which non-control references can placement resolve without runtime values?
- Can a rejected encoding be assigned an exact source extent?
- Which source-derived denominator proves semantic coverage?

The classification policy names below use the definitions in [Rejected encoding classification](ISA.md#rejected-encoding-classification). Every profile in the matrix implements the common byte and static-semantics protocols. The policy column states the implemented rejected-encoding boundary; it does not widen the profile beyond its declared processor and extension scope.

## Profile matrix

| Crate | Explicit protocol profiles | Required semantic context | Instruction extent | Serialization | Classification policy |
| --- | --- | --- | --- | --- | --- |
| `w65c816` | four M/X width combinations | emulation/native state | variable, selected by opcode and M/X state | little-endian operands | Exact encoding |
| `psx-r3000a` | PlayStation R3000A core, COP0, COP2 transfers, and GTE commands | none | 4 bytes | little-endian | Exact encoding |
| `huc6280` | documented HuC6280 profile | T flag state | variable | little-endian operands | Selector classification |
| `rp2a03` | documented and physically decoded RP2A03 selectors | none | variable | little-endian operands | Exact encoding |
| `sh2` | SH-2 without SH-DSP or later extensions | none | 2 bytes | big-endian | Exact encoding |
| `z80` | Zilog-documented profile | none | variable and prefix-based | little-endian operands | Strict decode only |
| `sh4` | four FPSCR.PR/SZ profiles | SR.MD/RB and FPSCR.FR bank state | 2 bytes | little-endian | Exact encoding per FPSCR profile |
| `m68000` | Motorola MC68000 | active A7 stack bank | variable in 16-bit words | big-endian | Selector classification |
| `v30` | native NEC V30 | interrupt-recognition state | variable and prefix-based | little-endian operands | Strict decode only |
| `v30mz` | WonderSwan V30MZ subset | interrupt-recognition state | variable and prefix-based | little-endian operands | Strict decode only |
| `sm83` | Sharp SM83 | none | variable with a CB map | little-endian operands | Exact encoding |
| `v810` | NEC V810 | none | 2 or 4 bytes | little-endian halfwords | Selector classification |
| `arm7tdmi` | little-endian ARM and Thumb profiles | current processor mode | 4 bytes in ARM; 2 or 4 in Thumb | little-endian | Exact encoding in ARM; selector classification in Thumb |
| `arm946e-s` | little-endian ARM and Thumb ARMv5TE profiles | current processor mode | 4 bytes in ARM; 2 or 4 in Thumb | little-endian | Exact encoding in ARM; selector classification in Thumb |
| `tlcs900h` | Toshiba TLCS-900/H | current register bank | variable and map-prefix-based | little-endian operands | Strict decode only |
| `vr4300` | Status.FR=0 and Status.FR=1 CPU/FPU profiles | none | 4 bytes | big-endian | Exact encoding per FR profile |
| `powerpc-gekko-750cl` | IBM Gekko and PowerPC 750CL | none | 4 bytes | big-endian | Exact encoding per processor profile |
| `i386` | code-16 and code-32 profiles | interrupt-recognition state and SS address size | variable and prefix-based | little-endian operands | Strict decode only |
| `allegrex` | PSP Allegrex CPU, FPU, and VFPU | VFPU prefix state | 4 bytes | little-endian | Exact encoding |
| `r5900` | PS2 EE CPU, multimedia, COP0, COP1, and VU0 macro mode | none | 4 bytes | little-endian | Exact encoding |

## W65C816

**Profile boundary and state.** Instruction length is selected by the four accumulator/index-width profiles. Emulation/native state is separate required semantic context even when emulation instruction lengths equal the M8/X8 profile. REP and SEP can produce known successor width state; XCE, PLP, and RTI require source-proven successor state or make subsequent semantic analysis unavailable.

**Code locations and targets.** A direct code location contains a program bank and 16-bit program counter. Sequential analysis may not silently cross a program-bank boundary. Direct branch targets remain in the current program bank; long calls and jumps may select another bank. Indirect targets retain the pointer addressing form and whether the instruction preserves or replaces the program bank.

**Locations and flow.** The location vocabulary covers the accumulator halves, X, Y, direct-page register, stack pointer, data and program banks, processor-status fields, and symbolic memory. It records implicit stack accesses, block-move updates, and direct-page/index updates. Flow distinguishes conditional branches, BRL/BRA, near and long calls, direct and indirect jumps, dedicated RTS/RTL/RTI forms, BRK/COP traps, WAI as a wait, and STP as a stop.

**References and classification.** Relative code targets resolve within the program bank. Long absolute operands produce static references directly. Bank-relative and direct-page operands remain symbolic ISA accesses unless a separate value-analysis layer proves the required architectural base. All opcode selectors belong to the profile, and explicit width state determines assigned instruction extent. Truncation or unavailable width state remains an error.

**Semantic gate.** The source denominator covers all 256 selectors in every width profile, emulation-specific effects, status-width transitions, stack-width behavior, and every addressing-mode side effect.

## PlayStation R3000A

**Profile boundary and state.** `PsxR3000A` covers the declared MIPS I integer core, PlayStation COP0 forms, COP2 transfers and memory operations, and documented GTE commands. It has no implicit decode mode. A direct code location is an aligned 32-bit virtual address.

**Locations and flow.** Locations cover each GPR, HI, LO, implemented COP0 registers and status fields, GTE data and control registers, and symbolic memory. Register zero never reports an architectural write. Loads, unaligned merge operations, link instructions, multiply/divide results, and every GTE command include their implicit accesses. Flow distinguishes branches, jumps, link-producing transfers, SYSCALL and BREAK traps, and their mandatory delay slots. `JR $ra` remains an indirect jump; an ABI convention does not turn it into a dedicated return instruction.

**References and classification.** Placed branch and jump targets are code targets. Base-plus-offset memory operands remain symbolic ISA accesses; a separate value-analysis layer may refine them. Every 32-bit word receives an exact profile classification; reserved fields and out-of-profile selectors preserve the four source bytes only in the non-executable classification result.

**Semantic gate.** Independent entries cover the MIPS core, COP0, COP2 transfer, and every documented GTE command. The gate includes conditional link writes, delay-slot behavior, merge-load read-modify-write effects, and GTE accumulator and flag effects rather than inferring them from operand names.

## HuC6280

**Profile boundary and state.** `HuC6280` models only selectors assigned by the processor software manual. The architectural code location is a 16-bit CPU address; physical ROM mapping through memory-page registers is consumer context rather than a fabricated code address. T flag state is required semantic context because it redirects the affected accumulator operations to zero-page storage through X.

**Locations and flow.** Locations cover A, X, Y, stack pointer, status fields including the HuC6280 T behavior, memory-page registers, speed-control state, symbolic memory, and immediate I/O-port destinations. SET produces a known T-state successor; affected ALU forms use the context-selected accumulator or zero-page-X storage and clear T according to the source rule. Addressing modes contribute index and stack reads, while TMA/TAM and block transfers expose their implicit register and memory accesses. Flow covers conditional and unconditional branches, calls, direct and indirect jumps, dedicated returns and RTI, and the BRK trap boundary.

**References and classification.** Resolved branch and call destinations are control targets. Absolute data operands and block-transfer source and destination operands form static references when their immediate addresses establish them. The 22 manual-undefined selector bytes have a source classification, but the manual does not assign them an instruction extent; they therefore remain selector classifications rather than lossless stream units.

**Semantic gate.** The manual's 234 assigned selectors are the denominator. The gate separately covers MPR operations, T-mode effects, block-transfer updates, and every addressing-mode memory access.

## RP2A03

**Profile boundary and state.** `Rp2A03` includes all 256 selectors physically decoded by the processor, retaining unofficial selector identity. Documented and unofficial behavior require separate source attribution. A direct code location is a wrapping 16-bit CPU address.

**Locations and flow.** Locations cover A, X, Y, stack pointer, individual status fields, and symbolic memory. Read-modify-write selectors, stack operations, and unofficial combined operations report all component accesses. Flow covers branches, JSR, direct and indirect JMP, dedicated RTS and RTI forms, the BRK trap boundary, and KIL as a stop.

**References and classification.** Resolved branch, jump, and call destinations are control targets. Absolute data operands and statically located pointer bytes form static references; indirect JMP retains its pointer rule rather than pretending that the destination is known. Because every physical selector is typed and its operand extent is determined by the selected form, complete input has an exact encoding classification and truncation remains an error.

**Semantic gate.** The gate partitions documented and unofficial selectors, uses source-bound silicon findings for the latter, and verifies effects for every physical selector rather than reusing the production opcode table as its denominator.

## SH-2

**Profile boundary and state.** `Sh2` covers the fixed-width SH-2 profile and excludes SH-DSP and later SuperH forms. A code location is an aligned 32-bit address.

**Locations and flow.** Locations cover R0-R15, SR fields, GBR, VBR, MACH, MACL, PR, and symbolic memory. Pre-decrement, post-increment, MAC address updates, implicit R0 operands, and trap or exception stack updates are first-class accesses. Flow covers conditional branches with and without delayed execution, BRA/BSR, BRAF/BSRF, JMP/JSR, dedicated RTS and RTE forms, the TRAPA synchronous boundary, SLEEP as a wait, and the architecture's illegal delay-slot rules.

**References and classification.** PC-relative loads and MOVA produce static references under the architecture's alignment rules. Direct branches and linked transfers produce code targets. Every 16-bit word has an exact assigned-or-reserved classification, while placement errors remain errors.

**Semantic gate.** The manual instruction list supplies the form denominator and independent access facts. The gate includes every implicit register update and delay-slot restriction; an exhaustive match over the production instruction enum is only an implementation check.

## Z80

**Profile boundary and state.** `Z80` contains documented Zilog encodings and excludes redundant prefixes and undocumented silicon behavior. A code location is a wrapping 16-bit address.

**Locations and flow.** Locations cover the primary and alternate register banks, index registers and halves where documented, SP, I, R, interrupt state, status fields, symbolic memory, and the I/O port space. Aliased register pairs are views over their physical byte locations rather than independent storage. Flow covers direct and relative conditional transfers, register-indirect jumps, calls, restart-vector calls, conditional returns, RETI/RETN, and HALT as a wait.

**References and classification.** Immediate memory and port operands form static references; register-indirect operands remain symbolic accesses. Prefix sequences are variable length, and a rejected or redundant prefix sequence has no general architecture-defined recovery extent. The public profile therefore remains strict-decode-only even though selector namespaces have independent coverage tests.

**Semantic gate.** The Zilog instruction tables provide the denominator for base, CB, ED, DD, FD, DDCB, and FDCB namespaces. The gate explicitly covers alternate-bank exchanges, implicit flag and interrupt-state accesses, and prefix-selected register substitution.

## SH-4

**Profile boundary and state.** The four `Sh4Pr*Sz*` profiles make FPSCR.PR and FPSCR.SZ part of instruction identity. SR.MD/RB and FPSCR.FR are required semantic context because they select physical general and floating-register banks without changing instruction extent. A code location is an aligned 32-bit address. FSCHG and FRCHG produce known state transitions; a general SR, RTE, or FPSCR write makes the successor profile or bank context unavailable unless the written state is independently established. An uncomposed delay slot preserves the known context at its entry but leaves post-slot FPSCR and floating-bank context unknown because permitted slot instructions can change them. An RTE slot starts with unknown general-register bank context because its instruction executes using the SR restored from SSR.

**Locations and flow.** Locations cover physical general and banked registers, SR fields, GBR, VBR, SSR, SPC, SGR, DBR, MACH, MACL, PR, FPSCR fields, FPUL, scalar, double, extended, pair, and vector floating registers, plus symbolic memory. Physical R0-R7 bank selection and FR/XF overlap are resolved from explicit context or conservatively unioned when the context is unknown. Flow uses the same SuperH transfer and delay-slot concepts as SH-2 while retaining SH-4-specific trap, exception-return, wait, and state locations.

**References and classification.** PC-relative literal loads and MOVA produce static references with the SH-4 alignment rules. Every 16-bit word is classified under one explicit FPSCR profile; the same word may legitimately classify differently under another profile, and no implicit default is permitted.

**Semantic gate.** The SH-4 manual opcode space is evaluated independently for every FPSCR profile. Semantic coverage separately proves FPU register grouping, banked-register effects, FPSCR writes, delay slots, and implicit system-register accesses.

## MC68000

**Profile boundary and state.** `Mc68000` excludes later-generation instructions and addressing extensions. A direct analysis point preserves the even 32-bit logical PC together with the MC68000's 24-bit bus-address projection; consumers must not treat different upper logical bytes as different physical code storage. Instruction extent is a variable number of big-endian words.

**Locations and flow.** Locations cover D0-D7, A0-A7, the user and supervisor stack storage selected by A7, status and condition fields, symbolic memory, and architecture-defined stack effects. The active A7 stack bank is explicit semantic context; an unknown bank produces conservative overlap. Register masks expand into physical register accesses. Effective-address pre-decrement and post-increment, LINK/UNLK, MOVEM ordering, exception frames, and read-modify-write memory forms are explicit. Flow covers branches, DBcc, JSR/JMP, dedicated subroutine and exception returns, explicit synchronous boundaries including CHK, ILLEGAL, TRAP, and conditional TRAPV, STOP as an interrupt-resumable wait, and indirect effective-address targets.

**References and classification.** Absolute and PC-relative effective addresses may produce static references or code targets according to the instruction's use. A first opcode word does not establish the extent or validity of every extension-dependent instruction. The existing first-word census is therefore a selector classification and must be named as such; it must not share the lossless full-encoding meaning of fixed-width `DecodedWord` APIs.

**Semantic gate.** The source-derived first-word space remains the encoding denominator, while semantic coverage uses the complete manual instruction and effective-address matrices. Logical-PC arithmetic, the 24-bit bus projection, and upper-byte aliases are checked explicitly. Zero-filled extension words are not semantic evidence.

## NEC V30

**Profile boundary and state.** `V30` models native mode. A direct code location is a segmented address, not a flattened integer. Prefix order belongs to exact encoding identity, while semantic prefixes modify the meaning of the wrapped instruction. Interrupt recognition after STI and SS loads is explicit successor context; every taken or repeated direct edge carries the post-instruction state rather than the input state.

**Locations and flow.** Locations cover byte and word registers, segment registers, individual flags, symbolic segmented memory, ports, and documented external registers. Register aliases share physical storage. REP-family prefixes produce conditional repeated accesses without simulating iteration counts; segment overrides alter memory-address selection; bus lock does not create a second semantic table. Flow covers conditional branches and loops, near and far direct or indirect jumps and calls, dedicated returns, software-interrupt traps including conditional INTO, HLT as a wait, and BRKEM as an instruction-set exit.

**References and classification.** Near references retain the current segment; far references carry both segment and offset. Immediate ports and statically addressed memory form static references. Prefix, escape, and ModR/M failures do not define a safe recovery length, so the profile remains strict-decode-only. Raw source bytes must not remain an executable `Instruction` variant.

**Semantic gate.** The native primary, secondary, prefix, and ModR/M maps supply independent denominators. Every prefix-compatible form, implicit flag or stack access, string operation, and native-to-emulation exit is covered.

## NEC V30MZ

**Profile boundary and state.** `V30Mz` uses the V30 native semantic vocabulary only for forms proven to exist on V30MZ. The `0F` extension space and REPC/REPNC prefixes remain outside the profile. Code locations stay segmented, and the shared analysis-point shape retains the same interrupt-recognition successor context.

**Locations and flow.** Storage, flow, and reference shapes that are architecturally identical may reuse the first-party V-series types. Reuse does not import V30-only instructions or semantic entries. Every accepted wrapper instruction has the same effects as its source-proven V30MZ form.

**References and classification.** Near, far, memory, and port references follow the V30 design. Undefined primary bytes and invalid prefix sequences remain strict decode errors with no invented lossless extent.

**Semantic gate.** The V30MZ manual's independent primary-selector partition is the admission gate. Semantic coverage is derived for the exact subset rather than inferred from successful reuse of the V30 implementation.

## SM83

**Profile boundary and state.** `Sm83` covers all assigned base and CB-prefixed forms and rejects illegal selectors. A direct code location is a wrapping 16-bit address.

**Locations and flow.** Locations use physical byte registers and individual flag bits; 16-bit pairs are views rather than duplicate storage. SP, interrupt-master state, symbolic memory, and the high-page I/O addressing forms are included. HL increment/decrement transfers and stack instructions report implicit updates. Flow covers conditional branches, jumps, calls, dedicated returns and RETI, restart-vector calls, HALT as a wait, and STOP as a profile-owned stop boundary without importing platform device behavior.

**References and classification.** Relative, absolute, and restart-vector control destinations belong to control flow. Absolute data and high-page immediate operands form static references where their address is fixed. Illegal base selectors occupy one byte, while the complete CB map and assigned instructions have selector-determined extents, so complete input supports exact classification.

**Semantic gate.** The independent base and CB selector maps are extended with access and flow facts for every assigned form, including flag-only operations, pair aliases, implicit SP changes, and interrupt-state changes.

## V810

**Profile boundary and state.** `V810` covers the seven architectural formats and documented bit-string and floating-point subopcodes. A direct code location is a halfword-aligned 32-bit address.

**Locations and flow.** Locations cover R0-R31, implemented system registers and PSW fields, symbolic memory, and the distinct I/O address space. Register zero does not report architectural writes. Bit-string operations include their implicit register set and memory accesses. Flow covers conditional branches, JR, link-producing JAL, register-indirect JMP, the TRAP synchronous boundary, RETI as an exception return, and HALT as a wait.

**References and classification.** PC-relative branch and jump displacements produce code targets; memory and I/O operands remain symbolic unless placement and immediate fields establish a target. The first halfword identifies an instruction format for assigned opcodes, but reserved selectors do not yet justify a universal rejected extent. Coverage classification therefore remains selector-level until the manual establishes exact recovery units.

**Semantic gate.** Every primary opcode, reserved field rule, bit-string subopcode, floating subopcode, system register, and format-specific implicit effect is independently enumerated.

## ARM7TDMI

**Profile boundary and state.** `Arm7TdmiArm` and `Arm7TdmiThumb` are separate little-endian profiles. Direct code locations carry both address and instruction-set state when they cross a profile boundary. ARM locations require four-byte alignment; Thumb locations require two-byte alignment.

**Locations and flow.** Locations cover physical banked and unbanked R0-R14 storage, CPSR fields, the current SPSR where defined, symbolic memory, and register lists expanded into physical accesses. Current processor mode is explicit semantic context and is never guessed when selecting banked registers. Condition codes make non-PC accesses conditional. Any instruction that writes PC, including data processing, loads, block transfers, branches, BL, and BX, contributes control flow rather than an ordinary PC write. BX retains its runtime state-selection rule in the indirect target; software interrupts are synchronous trap boundaries. Return-looking operand patterns remain jumps unless the architecture defines a dedicated return form.

**References and classification.** Branches produce code targets and PC-relative loads produce static references using the architectural PC bias and alignment. ARM-state words have exact fixed-width classification. Thumb BL pairs and rejected halfword classes require selector classification unless their complete extent is source-proven.

**Semantic gate.** ARM and Thumb instruction tables have separate denominators. The gate covers predication, banked and status registers, register-list order, PC-as-operand rules, memory writeback, interworking, and every PC-writing form.

## ARM946E-S

**Profile boundary and state.** `Arm946EsArm` and `Arm946EsThumb` extend the ARMv4T profiles only with ARMv5TE forms admitted by the processor scope. They do not widen to ARMv5TEJ or ARMv6. Code locations and interworking targets follow the ARM7TDMI design.

**Locations and flow.** Source-identical ARMv4T forms may reuse location types and semantic helpers under the same explicit processor-mode context. ARMv5TE DSP status effects, enhanced multiply forms, doubleword transfers, CLZ, BKPT, BLX, and admitted coprocessor forms receive explicit entries. BLX carries its direct or indirect instruction-set-state transition, and BKPT is a synchronous trap boundary.

**References and classification.** ARM words use exact classification and Thumb uses selector classification under the same rules as ARM7TDMI. An encoding inherited from the base implementation remains rejected if it is outside the exact ARM946E-S profile.

**Semantic gate.** ARMv4T entries are revalidated against the ARM946E-S boundary, and the ARMv5TE extension denominator is independently source-derived. Shared code is not extension coverage.

## TLCS-900/H

**Profile boundary and state.** `Tlcs900H` covers the exact TLCS-900/H primary, register, source-memory, and destination-memory maps. A direct code location uses the architecture's 24-bit program address rather than an unconstrained host integer.

**Locations and flow.** Locations cover current-bank and general registers, status and control fields, stack state, symbolic memory, and architecture-defined register views without double-counting aliases. The current register bank is explicit semantic context; unknown bank state is represented conservatively rather than mapped to an arbitrary bank. Auto-increment, auto-decrement, stack, multiply/divide, and block operations expose implicit accesses. Flow covers relative and absolute branches, calls, dedicated returns, software-interrupt traps, and HALT as a wait.

**References and classification.** Fully determined code destinations become direct control targets. Immediate and PC-relative non-control addresses become static references. Map prefixes and variable operand extensions make recovery after a rejected sequence ambiguous, so the profile remains strict-decode-only even though each selector map has an independent census.

**Semantic gate.** Each manual map is an independent denominator. Semantic entries cover every size variant, current-register selection, memory extension, condition, implicit stack access, and dummy or reserved selector boundary.

## VR4300

**Profile boundary and state.** `Vr4300Fr0` and `Vr4300Fr1` cover the declared MIPS III CPU and the FPU combinations valid under the selected CP0 Status.FR state while excluding an opaque CP2 operation field. FR is decode-profile state because it changes FPU register validity and physical pairing. A direct code location is an aligned 64-bit virtual address. A general Status write makes the successor profile unavailable unless FR is independently established; ERET preserves the selected FR profile while restoring its architectural control state.

**Locations and flow.** Locations cover GPRs, HI, LO, LLbit, CP0 registers and fields, the physical FGR storage and FR-selected FPR views, FPU control, typed CP2 transfer registers, and symbolic memory. Register zero never reports a write. Loads, stores, atomic operations, TLB/system forms, floating operations, and coprocessor transfers include implicit locations. Flow covers normal and likely branches, coprocessor branches, jumps, link-producing transfers, synchronous traps including SYSCALL/BREAK, ERET as a dedicated exception return, and delay-slot execution or annulment. A jump through the conventional link register remains an indirect jump.

**References and classification.** Placed branches and jumps form code targets. Memory operands remain symbolic without register values. Every 32-bit word has an exact classification under the selected FR profile, and the rejected result cannot make an FR-invalid FPU form or excluded CP2 operation bits executable.

**Semantic gate.** CPU, COP0, and FPU tables remain independently sourced and are evaluated under both FR profiles. The semantic denominator includes FGR pairing, odd-register validity, branch-likely slots, 64-bit register effects, LL/SC state, FPU condition fields, and the explicit CP2 exclusion.

## Gekko and PowerPC 750CL

**Profile boundary and state.** `PowerPcGekko` and `PowerPc750Cl` share source-identical executable forms but have different special-purpose-register maps. A direct code location is an aligned 32-bit address. The selected processor profile owns SPR admission.

**Locations and flow.** Locations cover GPRs, FPRs, condition-register fields, XER fields, LR, CTR, FPSCR fields, time-base and admitted SPRs, GQRs, and symbolic memory. Update-address loads and stores, string/multiple transfers, record forms, overflow forms, paired-single operations, and locked-cache forms expose their implicit accesses. Flow covers conditional and unconditional relative or absolute branches, link-producing transfers, LR/CTR indirect targets, synchronous traps and system calls, and dedicated exception returns without delay slots. Return mnemonics that alias branch-to-LR forms remain indirect branches at the ISA layer.

**References and classification.** Direct branch displacements form code targets; absolute operands produce static references when the instruction form establishes an address. Every word is classified under the selected processor profile, so an SPR accepted by 750CL may remain outside Gekko.

**Semantic gate.** Shared instruction entries are checked against both primary manuals, and processor-specific SPR effects use separate denominators. Paired-single, quantized memory, condition-field, update-address, and link/CTR effects are explicit.

## Intel 80386

**Profile boundary and state.** `I386Code16` and `I386Code32` make the code-segment D-bit default explicit. A direct code location retains code-segment identity and offset; it is not flattened without a descriptor-derived base supplied by a higher layer. Operand- and address-size overrides remain instruction facts. The SS descriptor's address size selects physical stack-register and memory views. Interrupt recognition after STI and SS loads is explicit successor context, and every direct edge carries its post-instruction state.

**Locations and flow.** Locations cover physical general-register storage and its byte/word/dword views, segment, control, debug, test, EFLAGS fields, symbolic segmented memory, ports, implicit stack locations, and opaque external numeric-coprocessor state. The 80386 ESC interface retains its command and operand encoding but does not claim the external 80387 value-transfer or stack model. Explicit memory operands retain their architectural `MemoryWidth` object class, while instruction-defined contiguous stack regions use a semantic `MemoryExtent::KnownBytes` rather than borrowing an unrelated operand class. Prefixes modify the wrapped instruction's accesses rather than forming unrelated semantics. Flow covers conditional transfers, loops, near and far direct or indirect jumps and calls, dedicated returns and IRET, explicit software-interrupt and checked-trap forms, HLT and WAIT boundaries, task or gate boundaries represented by admitted forms, and every instruction form that writes the instruction pointer. Code-16 sequential and short-relative offsets wrap at 16 bits; code-32 offsets retain 32-bit wrapping.

**References and classification.** Near references retain the current code segment; far pointers retain selector and offset. Immediate ports and absolute displacements form static references only when the address model establishes them. Prefix and ModR/M parsing is variable length, and invalid sequences have no universal recovery extent, so both profiles remain strict-decode-only.

**Semantic gate.** Code-16 and code-32 use separate denominators over primary, secondary, prefix, and ModR/M maps. The semantic gate covers aliases, partial-register overlap, implicit stack and string registers, flag subsets, far control flow, external coprocessor command effects, and prefix legality. A separate 80387 profile would require its own source denominator before exposing x87 stack meaning.

## Allegrex

**Profile boundary and state.** `Allegrex` covers the declared PSP CPU, FPU, and VFPU manuals. A direct code location is an aligned 32-bit address. The current VFPU source, target, and destination prefixes are semantic context because they select source components and destination masks without changing instruction identity.

**Locations and flow.** Locations cover GPRs, HI, LO, admitted special and control registers, FPRs and FCRs, VFPU scalar/vector/matrix and control locations, prefix state, condition fields, accumulators, and symbolic memory. Register zero never reports a write. VFPU prefix consumption and implicit condition or control effects are explicit; known valid prefixes select component reads and masked destination writes, while unknown valid prefix state produces conservative conditional lane effects rather than a guessed value. Manual-prohibited or restriction-violating prefix combinations fail closed instead of assigning the unbounded hardware result a normal access set. Instructions for which prefixes have no effect preserve the pending prefix stacks. Flow covers MIPS branches including likely forms, VFPU branches, jumps, link-producing transfers, BREAK/SYSCALL trap boundaries, and delay-slot execution or annulment. A point after an unanalyzed delay slot carries unknown prefix context until sequence analysis composes the slot. A conventional jump through the link register is not a dedicated return instruction.

**References and classification.** Direct branches and jumps form code targets; memory operations remain symbolic ISA accesses unless a separate value-analysis layer refines them. Every word has an exact CPU/FPU/VFPU classification under the declared profile.

**Semantic gate.** CPU, FPU, and VFPU manual-form denominators remain separate and independent. The gate covers all 268 rows in the VFPU User's Manual prefix-validity table in addition to VFPU operand grouping, prefix-state reads and writes, condition codes, branch-likely slots, and implicit accumulator or control effects.

## R5900

**Profile boundary and state.** `R5900` covers the PS2 EE CPU, multimedia extensions, COP0, COP1, and VU0 macro-mode COP2 forms in the declared manuals. A direct code location is an aligned 32-bit EE address.

**Locations and flow.** Locations cover 128-bit GPR storage and its operand views, HI/LO banks, SA, CP0 and performance state, FPRs and floating control, VU0 vector floating and integer registers, ACC, scalar control values, vector status fields, and symbolic memory. Register zero does not report a write. Packed operations, vector lanes, accumulator effects, and memory-update forms expand to their architectural locations. Interlocked instruction forms use the same static storage facts as their non-interlocked counterparts; interlock timing is outside this protocol. Flow covers normal and likely branches, link-producing transfers, jumps, synchronous traps including SYSCALL/BREAK, ERET as a dedicated exception return, and delay-slot execution or annulment. A conventional jump through the link register remains an indirect jump.

**References and classification.** Direct branch and jump targets are code targets. CPU and VU memory operands remain symbolic ISA accesses; a separate value-analysis layer may refine them. Every word has an exact classification; reserved top-level, COP, and vector encodings remain non-executable rejected results.

**Semantic gate.** CPU, EE multimedia, COP0, FPU, and VU0 macro-mode manual chapters retain separate source-derived denominators. The semantic gate covers lane-level reads and writes, accumulator and scalar control effects, both HI/LO banks, conditional links, memory updates, and branch-likely slots.

## Cross-profile review gate

The profile implementations are accepted only after a workspace review confirms all of the following:

1. each profile uses the common result shape and names;
2. every direct successor analysis point preserves its architectural location, decode profile, and required semantic context;
3. profile-owned location and target types preserve architecture distinctions without changing common meanings;
4. partial registers, banks, stacks, and lanes have sound storage-overlap relations;
5. every typed form appears in a source-derived semantic denominator;
6. selector classification is not exposed as lossless full-instruction classification;
7. state transitions that make a successor point unknown fail closed at sequence boundaries;
8. existing profile-specific control-flow and register-write APIs are removed or become derived views in the same migration;
9. source-lowering and patch consumers use the common facts rather than matching a second private semantic table.

Any design amendment discovered during implementation invalidates this gate. The review restarts from the common protocol and re-evaluates every profile section, including profiles believed to be unaffected, before the implementation is accepted. The review may reject the amendment and require the implementation to conform to the prior design; adding a local paragraph or test is not a substitute.
