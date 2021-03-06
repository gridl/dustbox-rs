use dustbox::machine::Machine;
use dustbox::cpu::{Decoder, R, Op, Parameter};
use dustbox::memory::MemoryAddress;

const DEBUG_TRACER: bool = false;

struct SeenDestination {
    /// segment:offset converted into real flat addresses
    address: MemoryAddress,

    sources: Vec<MemoryAddress>,

    visited: bool,
}

pub struct Tracer {
    seen_destinations: Vec<SeenDestination>,

    /// flat addresses of start of each visited opcode
    visited_addresses: Vec<MemoryAddress>,
}

impl Tracer {
    pub fn new() -> Self {
        Tracer {
            seen_destinations: Vec::new(),
            visited_addresses: Vec::new(),
        }
    }

    pub fn trace_execution(&mut self, machine: &mut Machine) {
        // tell tracer to start at CS:IP
        let ma = MemoryAddress::RealSegmentOffset(machine.cpu.get_r16(R::CS), machine.cpu.regs.ip);
        self.seen_destinations.push(SeenDestination{address: ma, visited: false, sources: Vec::new()});

        loop {
            self.trace_unvisited_destination(machine);
            if !self.has_any_unvisited_destinations() {
                // println!("exhausted all destinations, breaking!");
                break;
            }
        }
    }

    /// presents a flatish traced disassembly
    pub fn present_trace(&mut self, machine: &mut Machine) {
        // sort offsets
        self.visited_addresses.sort();

        let mut prev_end = 0;
        let mut decoder = Decoder::default();
        for ma in &self.visited_addresses {
            // decode op at this address and show it
            let ii = decoder.get_instruction_info(&mut machine.hw.mmu, ma.segment(), ma.offset());

            if prev_end != 0 {
                if prev_end != ma.value() {
                    // prev op was not aligned to this one, so show newline sepatator
                    println!("");
                }
            }

            // show branch cross references
            if let Some(mut sources) = self.get_sources_for_destination(*ma) {
                print!("; xref: ");
                sources.sort();
                let mut source_offsets = Vec::new();
                for src in &sources {
                    source_offsets.push(format!("{:04X}:{:04X}", src.segment(), src.offset()));
                }
                println!("{}", source_offsets.join(", "));
            }

            prev_end = MemoryAddress::RealSegmentOffset(ma.segment(), ma.offset() + ii.instruction.length as u16).value();
            println!("{}", ii);
        }
    }

    fn learn_destination(&mut self, seg: u16, offset: u16, src: MemoryAddress) {
        let ma = MemoryAddress::RealSegmentOffset(seg, offset);
        for seen in &mut self.seen_destinations {
            if seen.address.value() == ma.value() {
                // println!("address was already learned! [{:04X}:{:04X}] == {:06X}", seg, offset, dst);
                seen.sources.push(src);
                return;
            }
        }
        if DEBUG_TRACER {
            println!("learn_destination [{:04X}:{:04X}]", seg, offset);
        }
        self.seen_destinations.push(SeenDestination{address: ma, visited: false, sources: vec!(src)});
    }

    fn get_sources_for_destination(&self, ma: MemoryAddress) -> Option<Vec<MemoryAddress>> {
        for dst in &self.seen_destinations {
            if dst.address.value() == ma.value() {
                if dst.sources.len() == 0 {
                    return None;
                }
                return Some(dst.sources.clone());
            }
        }
        None
    }

    fn has_any_unvisited_destinations(&self) -> bool {
        for dst in &self.seen_destinations {
            if !dst.visited {
                return true;
            }
        }
        false
    }

    fn get_unvisited_destination(&self) -> Option<MemoryAddress> {
        for dst in &self.seen_destinations {
            if !dst.visited {
                return Some(dst.address);
            }
        }
        None
    }

    fn mark_destination_visited(&mut self, ma: MemoryAddress) {
         for dst in &mut self.seen_destinations {
            if dst.address == ma {
                // println!("XXX mark_destination_visited {:04X}:{:04X}", ma.segment(), ma.offset());
                dst.visited = true;
                return;
            }
        }
    }

    fn has_visited_address(&self, ma: MemoryAddress) -> bool {
        for visited in &self.visited_addresses {
            if visited.value() == ma.value() {
                return true;
            }
        }
        false
    }

    /// traces along one execution path until we have to give up, marking it as visited when complete
    fn trace_unvisited_destination(&mut self, machine: &mut Machine) {
        // find a non-visited seen dest
        let ma = self.get_unvisited_destination();
        if let None = ma {
            println!("ERROR: no destinations to visit");
            return;
        }
        let mut ma = ma.unwrap();
        let start_ma = ma;

        // if destination has been visited, mark and return
        if self.has_visited_address(ma) {
            // println!("We've already visited {:04X}:{:04X} == {:06X}, marking destination visited!", ma.segment(), ma.offset(), ma.value());
            self.mark_destination_visited(start_ma);
            return;
        }

        if DEBUG_TRACER {
            println!("trace_destination starting at {:04X}:{:04X}", ma.segment(), ma.offset());
        }

        let mut decoder = Decoder::default();

        loop {
            let ii = decoder.get_instruction_info(&mut machine.hw.mmu, ma.segment(), ma.offset());
            if DEBUG_TRACER {
                println!("Found {}", ii);
            }

            if self.has_visited_address(ma) {
                // println!("XXX already been here! breaking");
                break;
            }
            // mark visited_address
            self.visited_addresses.push(ma);

            match ii.instruction.command {
                Op::Invalid(_, _) => println!("ERROR: invalid/unhandled op {}", ii.instruction),
                Op::RetImm16 => panic!("XXX unhandled {}", ii.instruction),
                Op::Retn | Op::Retf => break,
                Op::JmpNear | Op::JmpFar | Op::JmpShort => {
                    match ii.instruction.params.dst {
                        Parameter::Imm16(imm) => self.learn_destination(ma.segment(), imm, ma),
                        Parameter::Reg16(_) => {}, // ignore "jmp bx"
                        Parameter::Ptr16(_, _) => {}, // ignore "jmp [0x4422]"
                        Parameter::Ptr16Imm(_, _) => {}, // ignore "jmp far 0xFFFF:0x0000"
                        Parameter::Ptr16AmodeS8(_, _, _) => {}, // ignore "jmp [di+0x10]
                        Parameter::Ptr16AmodeS16(_, _, _) => {}, // ignore "jmp [si+0x662C]"
                        _ => println!("ERROR1: unhandled dst type {:?}: {}", ii.instruction, ii.instruction),
                    }
                    // if unconditional branch, abort trace this path
                    break;
                }
                Op::CallNear | Op::CallFar | Op::Loop | Op::Loope | Op::Loopne |
                Op::Ja | Op::Jc | Op::Jcxz | Op::Jg | Op::Jl |
                Op::Jna | Op::Jnc | Op::Jng | Op::Jnl | Op::Jno | Op::Jns | Op::Jnz |
                Op::Jo | Op::Jpe | Op::Jpo | Op::Js | Op::Jz => {
                    // if conditional branch, record dst offset for later
                    match ii.instruction.params.dst {
                        Parameter::Imm16(imm) => self.learn_destination(ma.segment(), imm, ma),
                        Parameter::Reg16(_) => {}, // ignore "call bp"
                        Parameter::Ptr16(_, _) => {}, // ignore "call [0x4422]"
                        Parameter::Ptr16AmodeS8(_, _, _) => {}, // ignore "call [di+0x10]
                        Parameter::Ptr16AmodeS16(_, _, _) => {}, // ignore "call [bx-0x67A0]"
                        _ => println!("ERROR2: unhandled dst type {:?}: {}", ii.instruction, ii.instruction),
                    }
                }
                _ => {},
            }
            ma.inc_n(ii.instruction.length as u16);
            if ma.value() as isize - machine.cpu.rom_base as isize >= machine.cpu.rom_length as isize {
                println!("XXX breaking because we reached end of file");
                break;
            }
        }
        self.mark_destination_visited(start_ma);
    }
}
