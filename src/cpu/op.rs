use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// ASCII Adjust After Addition
    Aaa,

    /// ASCII Adjust AX Before Division
    Aad,

    /// ASCII Adjust AX After Multiply
    Aam,

    /// ASCII Adjust AL After Subtraction
    Aas,

    Adc8, Adc16, Adc32,
    Add8, Add16, Add32,
    And8, And16, And32,

    /// Adjust RPL Field of Segment Selector
    Arpl,

    /// Check Array Index Against Bounds
    Bound,

    /// Bit Scan Forward
    Bsf,

    /// Bit Test
    Bt,

    Bts,
    CallNear, CallFar,

    /// Convert Byte to Word
    Cbw,

    /// Clear Carry Flag
    Clc,

    /// Clear Direction Flag
    Cld,

    /// Clear Interrupt Flag
    Cli,

    /// Complement Carry Flag
    Cmc,

    Cmp8, Cmp16, Cmp32,
    Cmpsb, Cmpsw,

    /// Convert Word to Doubleword
    Cwd16, Cwde32,

    /// Decimal Adjust AL after Addition
    Daa,

    /// Decimal Adjust AL after Subtraction
    Das,

    Dec8, Dec16, Dec32,
    Div8, Div16, Div32,

    Enter,
    Hlt,

    Idiv8, Idiv16, Idiv32,
    Imul8, Imul16, Imul32,

    /// Input from Port
    In8, In16,

    Inc8, Inc16, Inc32,

    /// Input from Port to String
    Insb, Insw,

    Int,
    Into,
    Iret,

    /// Jump if above (CF=0 and ZF=0).    (alias: jnbe)
    Ja,

    /// Jump if carry (CF=1).    (alias: jb, jnae)
    Jc,

    /// Jump if CX register is 0.
    Jcxz,

    /// Jump if greater (ZF=0 and SF=OF).    (alias: jnle)
    Jg,

    /// Jump if less (SF ≠ OF).    (alias: jnge)
    Jl,

    JmpShort, JmpNear, JmpFar,

    /// Jump if not above (CF=1 or ZF=1).    (alias: jbe)
    Jna,

    /// Jump if not carry (CF=0).    (alias: jae, jnb)
    Jnc,

    /// Jump if not greater (ZF=1 or SF ≠ OF).    (alias: jle)
    Jng,

    /// Jump if not less (SF=OF).    (alias: jge)
    Jnl,

    /// Jump if not overflow (OF=0).
    Jno,

    /// Jump if not sign (SF=0).
    Jns,

    /// Jump if not zero (ZF=0).    (alias: jne)
    Jnz,

    /// Jump if overflow (OF=1).
    Jo,

    /// Jump short if parity even (PF=1)
    Jpe,

    /// Jump short if parity odd (PF=0).
    Jpo,

    /// Jump if sign (SF=1).
    Js,

    /// Jump if zero (ZF ← 1).    (alias: je)
    Jz,

    /// Load Status Flags into AH Register
    Lahf,

    Lds,

    /// Load Effective Address
    Lea16,

    Leave,

    Les,

    Lodsb, Lodsw, Lodsd,

    Loop, Loope, Loopne,

    Mov8, Mov16, Mov32,
    Movsb, Movsw, Movsd,

    /// Move with Sign-Extension
    Movsx16, Movsx32,

    /// Move with Zero-Extend
    Movzx16, Movzx32,

    Mul8, Mul16, Mul32,
    Neg8, Neg16, Neg32,
    Nop,
    Not8, Not16, Not32,
    Or8, Or16, Or32,
    Out8, Out16,
    Outsb, Outsw,
    Pop16, Pop32,

    /// Pop DI, SI, BP, BX, DX, CX, and AX.
    Popa16,

    /// Pop EDI, ESI, EBP, EBX, EDX, ECX, and EAX.
    Popad32,

    /// Pop top of stack into lower 16 bits of EFLAGS.
    Popf,

    Push16, Push32,

    /// Push AX, CX, DX, BX, original SP, BP, SI, and DI.
    Pusha16,

    /// Push EAX, ECX, EDX, EBX, original ESP, EBP, ESI, and EDI.
    Pushad32,

    /// push 16 bit FLAGS register onto stack
    Pushf,

    Rcl8, Rcl16, Rcl32,
    Rcr8, Rcr16, Rcr32,

    Retn, Retf, RetImm16,

    Rol8, Rol16, Rol32,
    Ror8, Ror16, Ror32,

    /// Store AH into Flags
    Sahf,

    /// "salc", or "setalc" is a undocumented Intel instruction
    /// http://ref.x86asm.net/coder32.html#gen_note_u_SALC_D6
    /// http://www.rcollins.org/secrets/opcodes/SALC.html
    /// used by dos-software-decoding/demo-256/luminous/luminous.com
    Salc,

    Sar8, Sar16, Sar32,

    /// Integer Subtraction with Borrow
    Sbb8, Sbb16, Sbb32,

    Scasb, Scasw,

    /// setc: Set byte if carry (CF=1).
    /// alias setb: Set byte if below (CF=1).
    Setc,

    /// setg: Set byte if greater (ZF=0 and SF=OF).
    /// alias setnle: Set byte if not less or equal (ZF=0 and SF=OF).
    Setg,

    /// setnz: Set byte if not zero (ZF=0).
    /// alias setne: Set byte if not equal (ZF=0).
    Setnz,

    Shl8, Shl16, Shl32,

    /// Double Precision Shift Left
    Shld,

    Shr8, Shr16, Shr32,

    /// Double Precision Shift Right
    Shrd,

    Sldt,

    // Set Carry Flag
    Stc,

    /// Set Direction Flag
    Std,

    /// Set Interrupt Flag
    Sti,

    Stosb, Stosw, Stosd,
    Sub8, Sub16, Sub32,
    Test8, Test16, Test32,

    /// Exchange Register/Memory with Register
    Xchg8, Xchg16, Xchg32,

    Xlatb,

    Xor8, Xor16, Xor32,

    Uninitialized,
    Invalid(Vec<u8>, Invalid),
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Op::Invalid(bytes, _) => {
                let mut x = Vec::new();
                for b in bytes {
                    x.push(format!("{:02X}", b));
                }
                write!(f, "INVALID {}", x.join(", "))
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

impl Op {
    pub fn is_valid(&self) -> bool {
        match *self {
            Op::Uninitialized | Op::Invalid(_, _) => false,
            _ => true,
        }
    }

    /// used by encoder
    pub fn f6_index(&self) -> u8 {
        match *self {
            Op::Test8 => 0,
            Op::Not8  => 2,
            Op::Neg8  => 3,
            Op::Mul8  => 4,
            Op::Imul8 => 5,
            Op::Div8  => 6,
            Op::Idiv8 => 7,
            _ => panic!("f6_index {:?}", self),
        }
    }

    /// used by encoder
    pub fn feff_index(&self) -> u8 {
        match *self {
            Op::Inc8 | Op::Inc16 | Op::Inc32 => 0,
            Op::Dec8 | Op::Dec16 | Op::Dec32 => 1,
            Op::CallNear => 2,
            // 3 => call far
            Op::JmpNear => 4,
            // 5 => jmp far
            Op::Push16 => 6,
            _ => panic!("feff_index {:?}", self),
        }
    }
}

/// the class of instruction decode error that occured
#[derive(Clone, Debug, PartialEq)]
pub enum Invalid {
    /// a reg value was unhandled / invalid
    Reg(u8),

    /// unimplemented / invalid CPU instr
    Op,

    /// unimplemented / invalid FPU instr
    FPUOp,
}
