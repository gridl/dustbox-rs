use std::cell::RefCell;
use std::num::Wrapping;
use std::marker::PhantomData;

use cpu::CPU;
use memory::{MMU, MemoryAddress};
use gpu::palette;
use gpu::palette::ColorSpace;
use gpu::palette::ColorSpace::RGB;
use gpu::font;
use gpu::video_parameters;
use gpu::modes::GFXMode;
use gpu::modes::VideoModeBlock;
use gpu::graphic_card::GraphicCard;
use bios::BIOS;
use bios;
use gpu::crtc::CRTC;
use gpu::dac::DAC;
use gpu::dac;

#[cfg(test)]
#[path = "./render_test.rs"]
mod render_test;

const DEBUG_FONT: bool = false;
const DEBUG_INTERRUPTS: bool = false;

const ECHO_TELETYPE: bool = false; // if set, character output from dos programs will be echoed to stdout

const CGA_MASKS: [u8; 4]  = [0x3f, 0xcf, 0xf3, 0xfc];
const CGA_MASKS2: [u8; 8] = [0x7f, 0xbf, 0xdf, 0xef, 0xf7, 0xfb, 0xfd, 0xfe];

const ACTL_MAX_REG: u8 = 0x14;

pub static STATIC_FUNCTIONALITY: [u8; 0x10] = [
 /* 0 */ 0xff,  // All modes supported #1
 /* 1 */ 0xff,  // All modes supported #2
 /* 2 */ 0x0f,  // All modes supported #3
 /* 3 */ 0x00, 0x00, 0x00, 0x00,  // reserved
 /* 7 */ 0x07,  // 200, 350, 400 scan lines
 /* 8 */ 0x04,  // total number of character blocks available in text modes
 /* 9 */ 0x02,  // maximum number of active character blocks in text modes
 /* a */ 0xff,  // Misc Flags Everthing supported
 /* b */ 0x0e,  // Support for Display combination, intensity/blinking and video state saving/restoring
 /* c */ 0x00,  // reserved
 /* d */ 0x00,  // reserved
 /* e */ 0x00,  // Change to add new functions
 /* f */ 0x00,  // reserved
];

#[derive(Clone)]
pub struct GPU {
    pub scanline: u32,
    pub crtc: CRTC,
    pub dac: DAC,
    font_8_first: MemoryAddress,
    font_8_second: MemoryAddress,
    pub font_14: MemoryAddress,
    font_14_alternate: MemoryAddress,
    pub font_16: MemoryAddress,
    font_16_alternate: MemoryAddress,
    static_config: MemoryAddress,
    video_parameter_table: MemoryAddress,
    video_dcc_table: MemoryAddress,
    pub card: GraphicCard,
    pub mode: VideoModeBlock,
    modes: Vec<VideoModeBlock>,
}

impl GPU {
    pub fn default() -> Self {
        let generation = GraphicCard::VGA;
        let modes = VideoModeBlock::get_mode_block(&generation);
        let mode = modes[3].clone();
        GPU {
            scanline: 0,
            crtc: CRTC::default(),
            dac: DAC::default(),
            font_8_first: MemoryAddress::Unset,
            font_8_second: MemoryAddress::Unset,
            font_14: MemoryAddress::Unset,
            font_14_alternate: MemoryAddress::Unset,
            font_16: MemoryAddress::Unset,
            font_16_alternate: MemoryAddress::Unset,
            static_config: MemoryAddress::Unset,
            video_parameter_table: MemoryAddress::Unset,
            video_dcc_table: MemoryAddress::Unset,
            card: generation,
            mode,
            modes,
        }
    }

    pub fn render_frame(&self, mmu: &MMU) -> Vec<u8> {
        let memory = mmu.dump_mem();
        match self.mode.mode {
            // 00: 40x25 Black and White text (CGA,EGA,MCGA,VGA)
            // 01: 40x25 16 color text (CGA,EGA,MCGA,VGA)
            // 02: 80x25 16 shades of gray text (CGA,EGA,MCGA,VGA)
            //0x03 => self.render_mode03_frame(memory), // 80x25 16 color text (CGA,EGA,MCGA,VGA)
            0x04 => self.render_mode04_frame(&memory), // 320x200 4 color graphics (CGA,EGA,MCGA,VGA)
            // 05: 320x200 4 color graphics (CGA,EGA,MCGA,VGA)
            //0x06 => self.render_mode06_frame(memory), // 640x200 B/W graphics (CGA,EGA,MCGA,VGA)
            // 07: 80x25 Monochrome text (MDA,HERC,EGA,VGA)
            // 08: 160x200 16 color graphics (PCjr)
            // 09: 320x200 16 color graphics (PCjr)
            // 0A: 640x200 4 color graphics (PCjr)
            // 0D: 320x200 16 color graphics (EGA,VGA)
            // 0E: 640x200 16 color graphics (EGA,VGA)
            // 0F: 640x350 Monochrome graphics (EGA,VGA)
            // 10: 640x350 16 color graphics (EGA or VGA with 128K)
            //     640x350 4 color graphics (64K EGA)
            //0x11 => self.render_mode11_frame(memory), // 640x480 B/W graphics (MCGA,VGA)
            //0x12 => self.render_mode12_frame(memory), // 640x480 16 color graphics (VGA)
            0x13 => self.render_mode13_frame(&memory), // 320x200 256 color graphics (MCGA,VGA)
            _ => {
                println!("XXX fixme render_frame for mode {:02x}", self.mode.mode);
                Vec::new()
            }
        }
    }
/*
    fn render_mode03_frame(&self, memory: &[u8]) -> Vec<u8> {
        // 03h = T  80x25  8x8   640x200   16       4   B800 CGA,PCjr,Tandy
        //     = T  80x25  8x14  640x350   16/64    8   B800 EGA
        //     = T  80x25  8x16  640x400   16       8   B800 MCGA
        //     = T  80x25  9x16  720x400   16       8   B800 VGA
        //     = T  80x43  8x8   640x350   16       4   B800 EGA,VGA [17]
        //     = T  80x50  8x8   640x400   16       4   B800 VGA [17]
        // XXX impl
        Vec::new()
    }
*/
    fn render_mode04_frame(&self, memory: &[u8]) -> Vec<u8> {
        // XXX palette selection is done by writes to cga registers
        // mappings to the cga palette
        let pal1_map: [usize; 4] = [0, 3, 5, 7];
        // let pal1_map: [u8; 3] = [11, 13, 15];
        // let pal0_map: [u8; 4] = [0, 2, 4, 6];
        // let pal0_map: [u8; 4] = [0, 10, 12, 14];

        // 04h = G  40x25  8x8   320x200    4       .   B800 CGA,PCjr,EGA,MCGA,VGA
        let mut buf = vec![0u8; (self.mode.swidth * self.mode.sheight * 3) as usize];
        // println!("cga draw {}x{}", self.mode.swidth, self.mode.sheight);
        for y in 0..self.mode.sheight {
            for x in 0..self.mode.swidth {
                // divide Y by 2
                // divide X by 4 (2 bits for each pixel)
                // 80 bytes per line (80 * 4 = 320), 4 pixels per byte
                let offset = (0xB_8000 + ((y%2) * 0x2000) + (80 * (y >> 1)) + (x >> 2)) as usize;
                let bits = (memory[offset] >> ((3 - (x & 3)) * 2)) & 3; // 2 bits: cga palette to use
                let pal = &self.dac.pal[pal1_map[bits as usize]];

                let dst = (((y * self.mode.swidth) + x) * 3) as usize;
                if let RGB(r, g, b) = *pal {
                    buf[dst] = r;
                    buf[dst+1] = g;
                    buf[dst+2] = b;
                }
            }
        }
        buf
    }
/*
    fn render_mode06_frame(&self, memory: &[u8]) -> Vec<u8> {
        // 06h = G  80x25  8x8   640x200    2       .   B800 CGA,PCjr,EGA,MCGA,VGA
        //     = G  80x25   .       .     mono      .   B000 HERCULES.COM on HGC [14]
        // XXX impl
        Vec::new()
    }

    fn render_mode11_frame(&self, memory: &[u8]) -> Vec<u8> {
        // 11h = G  80x30  8x16  640x480  mono      .   A000 VGA,MCGA,ATI EGA,ATI VIP
        // XXX impl
        Vec::new()
    }

    fn render_mode12_frame(&self, memory: &[u8]) -> Vec<u8> {
        // 12h = G  80x30  8x16  640x480   16/256K  .   A000 VGA,ATI VIP
        //     = G  80x30  8x16  640x480   16/64    .   A000 ATI EGA Wonder
        //     = G    .     .    640x480   16       .     .  UltraVision+256K EGA
        // XXX impl, planar mode
        Vec::new()
    }
*/
    fn render_mode13_frame(&self, memory: &[u8]) -> Vec<u8> {
        let mut buf = vec![0u8; (self.mode.swidth * self.mode.sheight * 3) as usize];
        for y in 0..self.mode.sheight {
            for x in 0..self.mode.swidth {
                let offset = 0xA_0000 + ((y * self.mode.swidth) + x) as usize;
                let byte = memory[offset];
                let pal = &self.dac.pal[byte as usize];
                let i = ((y * self.mode.swidth + x) * 3) as usize;
                if let RGB(r, g, b) = *pal {
                    buf[i] = r;
                    buf[i+1] = g;
                    buf[i+2] = b;
                }
            }
        }
        buf
    }

    /// int 10h, ah = 00h
    /// SET VIDEO MODE
    pub fn set_mode(&mut self, mmu: &mut MMU, bios: &mut BIOS, mode: u8) {
        if DEBUG_INTERRUPTS {
            println!("int 10h, ah = 00h: set_mode {:02X}", mode);
        }

        let mut found = false;
        for block in &self.modes {
            if block.mode == u16::from(mode) {
                self.mode = block.clone();
                found = true;
            }
        }
        if !found {
            panic!("video mode not found: {:02X} in graphics compatibility {:?}", mode, self.card);
        }

        match self.mode.kind {
            GFXMode::TEXT => self.dac.pal = palette::text_palette().to_vec(),
            GFXMode::CGA2 => self.dac.pal = palette::cga_palette_2().to_vec(),
            GFXMode::CGA4 => self.dac.pal = palette::cga_palette().to_vec(), // XXX is this the right cga pal for this mode?
            GFXMode::EGA => self.dac.pal = palette::ega_palette().to_vec(),
            GFXMode::VGA => self.dac.pal = palette::vga_palette().to_vec(),
            _ => panic!("set_mode: unhandled palette for video mode {:?}", self.mode.kind),
        }

        let clear_mem = true;
        bios.set_video_mode(mmu, &self.mode, clear_mem);

        /*
        // Set cursor shape
        if self.current_mode.kind == M_TEXT {
            INT10_SetCursorShape(0x06, 07);
        }
        */
        // Set cursor pos for page 0..7
        for ct in 0..8 {
            self.set_cursor_pos(mmu, 0, 0, ct);
        }
        self.set_active_page(mmu, 0);

        // Set some interrupt vectors
        match self.mode.cheight {
            0...3 | 7 | 8 => mmu.write_vec(0x43, &self.font_8_first),
            14 => mmu.write_vec(0x43, &self.font_14),
            16 => mmu.write_vec(0x43, &self.font_16),
            _ => {},
        }
    }

    /// int 10h, ah = 05h
    /// SELECT ACTIVE DISPLAY PAGE
    pub fn set_active_page(&mut self, mmu: &mut MMU, page: u8) {
        if DEBUG_INTERRUPTS {
            println!("int 10h, ah = 05h: set_active_page");
        }
        if page > 7 {
            println!("error: int10_set_active_page page {}", page);
        }
        /*
        if IS_EGAVGA_ARCH && (svgaCard == SVGA_S3Trio) {
            page &= 7;
        }
        */
        let mut mem_address = u16::from(page) * mmu.read_u16(BIOS::DATA_SEG, BIOS::DATA_PAGE_SIZE);
        // write the new page start
        mmu.write_u16(BIOS::DATA_SEG, BIOS::DATA_CURRENT_START, mem_address);
        if self.card.is_ega_vga() {
            if self.mode.mode < 8 {
                mem_address >>= 1;
            }
            // rare alternative: if mode.kind == TEXT { mem_address >>= 1; }
        } else {
            mem_address >>= 1;
        }
        // write the new start address in vga hardware
        self.crtc.set_index(0x0C);
        self.crtc.write_current((mem_address >> 8) as u8);
        self.crtc.set_index(0x0D);
        self.crtc.write_current((mem_address) as u8);

        // and change the BIOS page
        mmu.write_u8(BIOS::DATA_SEG, BIOS::DATA_CURRENT_PAGE, page);

        let cur_row = bios::cursor_pos_row(mmu, page);
        let cur_col = bios::cursor_pos_col(mmu, page);
        self.set_cursor_pos(mmu, cur_row, cur_col, page);
    }

    /// returns the active display page value
    pub fn get_active_page(&self, mmu: &mut MMU) -> u8 {
        mmu.read_u8(BIOS::DATA_SEG, BIOS::DATA_CURRENT_PAGE)
    }

    /// int 10h, ah = 02h
    /// SET CURSOR POSITION
    pub fn set_cursor_pos(&mut self, mmu: &mut MMU, row: u8, col: u8, page: u8) {
        if DEBUG_INTERRUPTS {
            println!("int 10h, ah = 02h: set_cursor_pos");
        }
        // page = page number:
        //    0-3 in modes 2&3
        //    0-7 in modes 0&1
        //    0 in graphics modes
        // row = 0 is top
        // col = column (0 is left)
        if page > 7 {
            println!("error: set_cursor_pos page {}", page);
        }
        // BIOS cursor pos
        let cursor_ofs = u16::from(page) * 2;
        mmu.write_u8(BIOS::DATA_SEG, BIOS::DATA_CURSOR_POS + cursor_ofs, col);
        mmu.write_u8(BIOS::DATA_SEG, BIOS::DATA_CURSOR_POS + cursor_ofs + 1, row);

        if page == self.get_active_page(mmu) {
            // Set the hardware cursor
            let ncols = mmu.read_u16(BIOS::DATA_SEG, BIOS::DATA_NB_COLS);

            // Calculate the address knowing nbcols nbrows and page num
            // NOTE: OFFSET_CURRENT_START counts in colour/flag pairs
            let address = (ncols * u16::from(row)) + u16::from(col) + mmu.read_u16(BIOS::DATA_SEG, BIOS::DATA_CURRENT_START) / 2;
            self.crtc.set_index(0x0E);
            self.crtc.write_current((address >> 8) as u8);
            self.crtc.set_index(0x0F);
            self.crtc.write_current(address as u8);
        }
    }

    /// int 10h, ah = 0Ah
    /// WRITE CHARACTER ONLY AT CURSOR POSITION
    pub fn write_char(&mut self, mut mmu: &mut MMU, chr: u16, attr: u8, mut page: u8, mut count: u16, mut showattr: bool) {
        if DEBUG_INTERRUPTS {
            println!("int 10h, ah = 0Ah: write_char");
        }
        if !self.mode.is_text() {
            showattr = true;
            match self.card {
                GraphicCard::EGA | GraphicCard::VGA => page %= self.mode.ptotal,
                GraphicCard::CGA => page = 0,
                _ => {},
            }
        }

        let mut cur_row = bios::cursor_pos_row(mmu, page);
        let mut cur_col = bios::cursor_pos_col(mmu, page);
        let ncols = mmu.read_u16(BIOS::DATA_SEG, BIOS::DATA_NB_COLS);

        while count > 0 {
            self.write_char_internal(&mut mmu, u16::from(cur_col), u16::from(cur_row), page, chr, attr, showattr);
            count -= 1;
            cur_col += 1;
            if u16::from(cur_col) == ncols {
                cur_col = 0;
                cur_row += 1;
            }
        }
    }

    /// int 10h, ah = 0Eh
    /// TELETYPE OUTPUT
    /// Display a character on the screen, advancing the cursor
    /// and scrolling the screen as necessary
    pub fn teletype_output(&mut self, mmu: &mut MMU, chr: u8, page: u8, attr: u8) {
        // BL = foreground color (graphics modes only)
        if DEBUG_INTERRUPTS {
            println!("int 10h, ah = 0Eh: teletype_output");
        }
        if ECHO_TELETYPE {
            print!("{}", chr as char);
        }
        let use_attr = self.mode.kind != GFXMode::TEXT;
        self.teletype_output_attr(mmu, chr, attr, page, use_attr);
    }

    fn teletype_output_attr(&mut self, mmu: &mut MMU, chr: u8, attr: u8, page: u8, use_attr: bool) {
        let ncols = mmu.read_u16(BIOS::DATA_SEG, BIOS::DATA_NB_COLS);
        let nrows = mmu.read_u16(BIOS::DATA_SEG, BIOS::DATA_NB_ROWS) + 1;
        let mut cur_row = u16::from(bios::cursor_pos_row(mmu, page));
        let mut cur_col = u16::from(bios::cursor_pos_col(mmu, page));
        match chr {
            /*
            7 => {
                // enable speaker
                hw.out_u8(0x61, IO_Read(0x61) | 0x3);
                for (Bitu i=0; i < 333; i++) {
                    CALLBACK_Idle();
                }
                hw.out_u8(0x61, IO_Read(0x61) & ~0x3);
            }
            */
            8 => {
                if cur_col > 0 {
                    cur_col -= 1;
                }
            }
            b'\r' => {
                cur_col = 0;
            }
            b'\n' => {
                // cur_col=0; //Seems to break an old chess game
                cur_row += 1;
            }
            /*
            b'\t' => {
                do {
                    INT10_TeletypeOutputAttr(' ',attr,useattr,page);
                    cur_row = cursor_pos_row(page);
                    cur_col = CURSOR_POS_COL(page);
                } while (cur_col % 8);
            }
            */
            _ => {
                self.write_char_internal(mmu, cur_col, cur_row, page, u16::from(chr), attr, use_attr);
                cur_col += 1;
            }
        }
        if cur_col == ncols {
            cur_col = 0;
            cur_row += 1;
        }
        // Do we need to scroll ?
        if cur_row == nrows {
            // Fill with black on non-text modes and with 0x7 on textmode

            // XXX in gpu branch:
            /*
            let fill = if self.mode.kind == GFXMode::TEXT {
                7
            } else {
                0
            };
            int10_scroll_window(hw, 0, 0, (nrows-1) as u8, (ncols-1) as u8, -1, fill, page);
            */
            cur_row -= 1;
        }
        self.set_cursor_pos(mmu, cur_row as u8, cur_col as u8, page);
    }

    fn write_char_internal(&mut self, mmu: &mut MMU, col: u16, row: u16, page: u8, mut chr: u16, mut attr: u8, use_attr: bool) {
        chr &= 0xFF;
        let cheight = mmu.read_u8(BIOS::DATA_SEG, BIOS::DATA_CHAR_HEIGHT);
        let (fontdata_seg, mut fontdata_off) = match self.mode.kind {
            GFXMode::TEXT => {
                let mut address = u32::from(u16::from(page) * mmu.read_u16(BIOS::DATA_SEG, BIOS::DATA_PAGE_SIZE));
                address += u32::from((row * mmu.read_u16(BIOS::DATA_SEG, BIOS::DATA_NB_COLS) + col) * 2);
                let dst = self.mode.pstart + address;
                mmu.memory.borrow_mut().write_u8(dst, chr as u8);
                if use_attr {
                    mmu.memory.borrow_mut().write_u8(dst + 1, attr);
                }
                (0, 0)
            }
            GFXMode::CGA4 | GFXMode::CGA2 | GFXMode::TANDY16 => {
                let (seg, off) = if chr < 0x80 {
                    mmu.read_vec(0x43)
                } else {
                    chr -= 0x80;
                    mmu.read_vec(0x1F)
                };
                (seg, off + (chr * u16::from(cheight)))
            }
            _ => {
                let (seg, off) = mmu.read_vec(0x43);
                (seg, off + (chr * u16::from(cheight)))
            }
        };

        if !use_attr {
            attr = match self.mode.kind {
                GFXMode::CGA4 => 0x3,
                GFXMode::CGA2 => 0x1,
                _ => 0x7,
            };
        }

        //Some weird behavior of mode 6
        //(same fix for 11 fixes vgatest2, but it's not entirely correct according to wd)
        if self.mode.mode == 0x6 {
            attr = (attr & 0x80) | 1;
        }

        let x = 8 * col;
        let mut y = u16::from(cheight) * row;
        let xor_mask = if self.mode.kind == GFXMode::VGA {
            0
        } else {
            0x80
        };
        /*
        if self.mode.kind == GFXMode::EGA {
            // enable all planes for EGA modes (Ultima 1 colour bug)
            // might be put into INT10_PutPixel but different vga bios
            // implementations have different opinions about this
            hw.out_u8(0x3C4, 0x2);
            hw.out_u8(0x3C5, 0xF);
        }
        */
        if DEBUG_FONT {
            println!("reading fontdata from {:04X}:{:04X}", fontdata_seg, fontdata_off);
        }
        for idx in 0..cheight {
            let mut bitsel = 128;
            let bitline = mmu.read_u8(fontdata_seg, fontdata_off);
            if DEBUG_FONT {
                println!("read fontdata {} = {:02x}", idx, bitline);
            }
            fontdata_off += 1;
            let mut tx = x as u16;
            while bitsel != 0 {
                if bitline & bitsel != 0 {
                    self.write_pixel(mmu, tx, y as u16, page, attr);
                } else {
                    self.write_pixel(mmu, tx, y as u16, page, attr & xor_mask);
                }
                tx += 1;
                bitsel >>= 1;
            }
            y += 1;
        }
    }

    /// int 10h, ah = 0Ch
    /// WRITE GRAPHICS PIXEL
    /// color: if bit 7 is set, value is XOR'ed onto screen except in 256-color modes
    pub fn write_pixel(&mut self, mmu: &mut MMU, x: u16, y: u16, _page: u8, mut color: u8) {
        if DEBUG_INTERRUPTS {
            println!("int 10h, ah = 0Ch: write_pixel");
        }
        match self.mode.kind {
            GFXMode::TEXT => {}, // Valid only in graphics modes
            GFXMode::CGA4 => {
                if mmu.read_u8(BIOS::DATA_SEG, BIOS::DATA_CURRENT_MODE) <= 5 {
                    // this is a 16k mode
                    let mut off = ((y >> 1) * 80 + (x >> 2)) as u16;
                    if y & 1 != 0 {
                        off += 8 * 1024;
                    }
                    let mut old = mmu.read_u8(0xB800, off);
                    if color & 0x80 != 0 {
                        color &= 3;
                        old ^= color << (2 * (3 - (x & 3)));
                    } else {
                        old = (old & CGA_MASKS[x as usize & 3]) | ((color & 3) << (2 * (3 - (x & 3))));
                    }
                    mmu.write_u8(0xB800, off, old);
                } else {
                    let seg: u16 = if self.card.is_pc_jr() {
                        // a 32k mode: PCJr special case (see M_TANDY16)
                        let cpupage = (mmu.read_u8(BIOS::DATA_SEG, BIOS::DATA_CRTCPU_PAGE) >> 3) & 0x7;
                        u16::from(cpupage) << 10 // A14-16 to addr bits 14-16
                    } else {
                        0xB800
                    };
                    let mut off = ((y >> 2) * 160 + ((x >> 2) & (!1))) as u16;
                    off += (8 * 1024) * (y & 3);

                    let mut old = mmu.read_u16(seg, off);
                    if color & 0x80 != 0 {
                        old ^=  (u16::from(color) & 1)       <<  (7 - (x & 7));
                        old ^= ((u16::from(color) & 2) >> 1) << ((7 - (x & 7)) + 8);
                    } else {
                        old = (old & (!(0x101              <<  (7 - (x & 7))))) |
                             ((u16::from(color) & 1)       <<  (7 - (x & 7))) |
                            (((u16::from(color) & 2) >> 1) << ((7 - (x & 7)) + 8));
                    }
                    mmu.write_u16(seg, off, old);
                }
            }
            GFXMode::VGA => mmu.write_u8(0xA000, y * 320 + x, color),
            _ => panic!("put_pixel TODO unimplemented mode {:?}", self.mode.kind),
        }
    }

    /// int 10h, ax = 1017h
    /// READ BLOCK OF DAC REGISTERS (VGA/MCGA)
    pub fn read_dac_block(&mut self, mmu: &mut MMU, index: u16, mut count: u16, seg: u16, mut off: u16) {
        if DEBUG_INTERRUPTS {
            println!("int 10h, ax = 1017h: read_dac_block");
        }
        // index = starting palette register
        // count = number of palette registers to read
        // seg:off -> buffer (3 * CX bytes in size) (see also AX=1012h)
        // Return: buffer filled with CX red, green and blue triples
        self.dac.set_pel_read_index(index as u8);
        while count > 0 {
            mmu.write_u8(seg, off, self.dac.get_pel_data());
            off += 1;
            mmu.write_u8(seg, off, self.dac.get_pel_data());
            off += 1;
            mmu.write_u8(seg, off, self.dac.get_pel_data());
            off += 1;
            count -= 1;
        }
    }

    /// int 10h, ax = 1124h
    /// GRAPH-MODE CHARGEN - LOAD 8x16 GRAPHICS CHARS (VGA,MCGA)
    pub fn load_graphics_chars(&mut self, mmu: &mut MMU, row: u8, dl: u8) {
        if DEBUG_INTERRUPTS {
            println!("int 10h, ax = 1124h: load_graphics_chars");
        }
        if !self.card.is_vga() {
            return;
        }
        mmu.write_vec(0x43, &self.font_16);
        mmu.write_u16(BIOS::DATA_SEG, BIOS::DATA_CHAR_HEIGHT, 16);
        let val = match row {
            0x00 => dl - 1, // row 0 = user specified in DL
            0x01 => 13,
            0x03 => 42,
            0x02 | _ => 24,
        };
        mmu.write_u8(BIOS::DATA_SEG, BIOS::DATA_NB_ROWS, val);
    }

    /// int 10h, ah = 13h
    /// WRITE STRING (AT and later,EGA)
    pub fn write_string(&mut self, mmu: &mut MMU, mut row: u8, mut col: u8, flag: u8, mut attr: u8, str_seg: u16, mut str_off: u16, mut count: u16, page: u8) {
        if DEBUG_INTERRUPTS {
            println!("int 10h, ah = 13h: write_string");
        }
        let cur_row = bios::cursor_pos_row(mmu, page);
        let cur_col = bios::cursor_pos_col(mmu, page);
        if row == 0xFF {
            // use current cursor position
            row = cur_row;
            col = cur_col;
        }
        self.set_cursor_pos(mmu, row, col, page);
        while count > 0 {
            let chr = mmu.read_u8(str_seg, str_off);
            str_off += 1;
            if flag & 2 != 0 {
                attr = mmu.read_u8(str_seg, str_off);
                str_off += 1;
            };
            self.teletype_output_attr(mmu, chr, attr, page, true);
            count -= 1;
        }
        if flag & 1 == 0 {
            self.set_cursor_pos(mmu, cur_row, cur_col, page);
        }
    }

    /// int 10h, ax = 1007h
    /// GET INDIVIDUAL PALETTE REGISTER (VGA,UltraVision v2+)
    pub fn get_individual_palette_register(&self, _reg: u8) -> u8 {
        if DEBUG_INTERRUPTS {
            println!("int 10h, ax = 1007h: get_individual_palette_register");
        }
        panic!("todo");
        /*
        const VGAREG_ACTL_ADDRESS: u16    = 0x3C0;
        const VGAREG_ACTL_WRITE_DATA: u16 = 0x3C0;
        const VGAREG_ACTL_READ_DATA: u16  = 0x3C1;

        if reg <= ACTL_MAX_REG {
            self.reset_actl();
            IO_Write(VGAREG_ACTL_ADDRESS, reg + 32);
            let ret = IO_Read(VGAREG_ACTL_READ_DATA);
            IO_Write(VGAREG_ACTL_WRITE_DATA, ret);
            ret
        }
        0
        */
    }

    /// int 10h, ax = 1010h
    /// SET INDIVIDUAL DAC REGISTER (VGA/MCGA)
    /// color components in 6-bit values (0-63)
    pub fn set_individual_dac_register(&mut self, mmu: &mut MMU, index: u8, r: u8, g: u8, b: u8) {
        if DEBUG_INTERRUPTS {
            println!("int 10h, ax = 1010h: set_individual_dac_register: index {:02X}, rgb = {:02X}, {:02X}, {:02X}", index, r, g, b);
        }
        self.dac.set_pel_write_index(index);
        if (mmu.read_u8(BIOS::DATA_SEG, BIOS::DATA_MODESET_CTL) & 0x06) == 0 {
            self.dac.set_pel_data(r);
            self.dac.set_pel_data(g);
            self.dac.set_pel_data(b);
        } else {
            // calculate clamped intensity, taken from VGABIOS
            let i = (( 77 * u32::from(r) + 151 * u32::from(g) + 28 * u32::from(b) ) + 0x80) >> 8;
            let ic = if i > 0x3F {
                0x3F
            } else {
                i as u8
            };
            self.dac.set_pel_data(ic);
            self.dac.set_pel_data(ic);
            self.dac.set_pel_data(ic);
        }
    }

    /// int 10, ax = 1012h
    /// SET BLOCK OF DAC REGISTERS (VGA/MCGA)
    pub fn set_dac_block(&mut self, mmu: &mut MMU, index: u16, mut count: u16, seg: u16, mut off: u16) {
        if DEBUG_INTERRUPTS {
            println!("int 10h, ax = 1012h: set_dac_block: index {:04X}, count {} at {:04X}:{:04X}", index, count, seg, off);
        }
        // index = starting color register
        // count = number of registers to set
        // seg:off -> table of 3*CX bytes where each 3 byte group represents one byte each of red, green and blue (0-63)

        self.dac.set_pel_write_index(index as u8);

        if (mmu.read_u8(BIOS::DATA_SEG, BIOS::DATA_MODESET_CTL) & 0x06) == 0 {
            while count > 0 {
                let r = mmu.read_u8(seg, off); off += 1;
                self.dac.set_pel_data(r);

                let g = mmu.read_u8(seg, off); off += 1;
                self.dac.set_pel_data(g);

                let b = mmu.read_u8(seg, off); off += 1;
                self.dac.set_pel_data(b);
                count -= 1;
            }
        } else {
            while count > 0 {
                let r = mmu.read_u8(seg, off); off += 1;
                let g = mmu.read_u8(seg, off); off += 1;
                let b = mmu.read_u8(seg, off); off += 1;

                // calculate clamped intensity, taken from VGABIOS
                let i = (( 77 * u32::from(r) + 151 * u32::from(g) + 28 * u32::from(b) ) + 0x80) >> 8;
                let ic = if i > 0x3F {
                    0x3F
                } else {
                    i as u8
                };
                self.dac.set_pel_data(ic);
                self.dac.set_pel_data(ic);
                self.dac.set_pel_data(ic);
                count -= 1;
            }
        }
    }

    /// int 10h, ax = 1015h
    /// READ INDIVIDUAL DAC REGISTER (VGA/MCGA)
    pub fn get_individual_dac_register(&mut self, reg: u8) -> (u8, u8, u8) {
        if DEBUG_INTERRUPTS {
            println!("int 10h, ax = 1015h: get_individual_dac_register: reg {:02X}", reg);
        }
        self.dac.set_pel_read_index(reg);
        let r = self.dac.get_pel_data();
        let g = self.dac.get_pel_data();
        let b = self.dac.get_pel_data();
        (r, g, b)
    }

    fn reset_actl(&self) {
        // 03BA  R-  CRT status register (see #P0656)
        // 03DA  R-  CGA status register (see #P0818)
        /*
        match self.mode.crtc_address() + 6 {
            0x3BA => panic!("xxx"),
            0x3DA => self.read_cga_status_register(),
        }
        */
    }

    /// HACK to have a source of info to toggle CGA status register
    pub fn progress_scanline(&mut self) {
        self.scanline += 1;
        if self.scanline > self.mode.swidth {
            self.scanline = 0;
        }
    }

    /// CGA status register (0x03DA)
    /// color EGA/VGA: input status 1 register
    pub fn read_cga_status_register(&self) -> u8 {
        // Bitfields for CGA status register:
        // Bit(s)	Description	(Table P0818)
        // 7-6	not used
        // 7	(C&T Wingine) vertical sync in progress (if enabled by XR14)
        // 5-4	color EGA, color ET4000, C&T: diagnose video display feedback, select
        //      from color plane enable
        // 3	in vertical retrace
        //      (C&T Wingine) video active (retrace/video selected by XR14)
        // 2	(CGA,color EGA) light pen switch is off
        //      (MCGA,color ET4000) reserved (0)
        //      (VGA) reserved (1)
        // 1	(CGA,color EGA) positive edge from light pen has set trigger
        //      (VGA,MCGA,color ET4000) reserved (0)
        // 0	horizontal retrace in progress
        //    =0  do not use memory
        //    =1  memory access without interfering with display
        //        (VGA,Genoa SuperEGA) horizontal or vertical retrace
        //    (C&T Wingine) display enabled (retrace/DE selected by XR14)
        let mut flags = 0;

        // FIXME REMOVE THIS HACK: fake bit 0 and 3 (retrace in progress)
        if self.scanline == 0 {
            flags |= 0b0000_0001; // set bit 0
            flags |= 0b0000_1000; // set bit 3
        } else {
            flags &= 0b1111_1110; // clear bit 0
            flags &= 0b1111_0111; // clear bit 3
        }

        // println!("read_cga_status_register: returns {:02X}", flags);

        flags
    }

    fn setup_video_parameter_table(&mut self, mmu: &mut MMU, addr: &mut MemoryAddress) -> u16 {
        let base = addr.offset();
        if self.card.is_vga() {
            for (i, b) in video_parameters::TABLE_VGA.iter().enumerate() {
                addr.set_offset(base + i as u16);
                mmu.write_u8(addr.segment(), addr.offset(), *b);
            }
            return video_parameters::TABLE_VGA.len() as u16;
        }
        for (i, b) in video_parameters::TABLE_EGA.iter().enumerate() {
            addr.set_offset(base + i as u16);
            mmu.write_u8(addr.segment(), addr.offset(), *b);
        }

        video_parameters::TABLE_EGA.len() as u16
    }

    fn video_bios_size(&self) -> u16 {
        0x8000 // XXX
    }

    pub fn init(&mut self, mut mmu: &mut MMU) {
        let mut addr = MemoryAddress::RealSegmentOffset(0xC000, 3);
        //let seg = 0xC000;

        let video_bios_size = self.video_bios_size();

        //let mut pos = 3;
        if self.card.is_ega_vga() {
            // ROM signature
            mmu.write_u16_inc(&mut addr, 0xAA55);
            mmu.write_u8_inc(&mut addr, (video_bios_size >> 9) as u8);

            // entry point
            mmu.write_u8_inc(&mut addr, 0xFE); // Callback instruction
            mmu.write_u8_inc(&mut addr, 0x38);
            mmu.write_u16_inc(&mut addr, 0 /* XXX VGA_ROM_BIOS_ENTRY_cb */);
            mmu.write_u8_inc(&mut addr, 0xCB); // retf

            // VGA BIOS copyright
            if self.card.is_vga() {
                mmu.write(addr.segment(), 0x1E, b"IBM compatible VGA BIOS\0");
            } else {
                mmu.write(addr.segment(), 0x1E, b"IBM compatible EGA BIOS\0");
            }

            addr.set_offset(0x100);
        }

        // cga font
        self.font_8_first = addr.clone();
        if DEBUG_FONT {
            println!("font_8_first = {:04X}:{:04X}", self.font_8_first.segment(), self.font_8_first.offset());
        }
        for i in 0..(128 * 8) {
            mmu.write_u8_inc(&mut addr, font::FONT_08[i]);
        }

        if self.card.is_ega_vga() {
            // cga second half
            self.font_8_second = addr.clone();
            if DEBUG_FONT {
                println!("font_8_second = {:04X}:{:04X}", self.font_8_second.segment(), self.font_8_second.offset());
            }
            for i in 0..(128 * 8) {
                mmu.write_u8_inc(&mut addr, font::FONT_08[i + (128 * 8)]);
            }
        }

        if self.card.is_ega_vga() {
            // ega font
            self.font_14 = addr.clone();
            if DEBUG_FONT {
                println!("font_14 = {:04X}:{:04X}", self.font_14.segment(), self.font_14.offset());
            }
            for i in 0..(256 * 14) {
                mmu.write_u8_inc(&mut addr, font::FONT_14[i]);
            }
        }

        if self.card.is_vga() {
            // vga font
            self.font_16 = addr.clone();
            if DEBUG_FONT {
                println!("font_16 = {:04X}:{:04X}", self.font_16.segment(), self.font_16.offset());
            }
            for i in 0..(256 * 16) {
                mmu.write_u8_inc(&mut addr, font::FONT_16[i]);
            }

            self.static_config = addr.clone();
            for item in STATIC_FUNCTIONALITY.iter().take(0x10) {
                mmu.write_u8_inc(&mut addr, *item);
            }
        }

        mmu.write_vec(0x1F, &self.font_8_second);
        self.font_14_alternate = addr.clone();
        self.font_16_alternate = addr.clone();

        mmu.write_u8_inc(&mut addr, 0x00); // end of table (empty)

        if self.card.is_ega_vga() {
            self.video_parameter_table = addr.clone();
            self.setup_video_parameter_table(&mut mmu, &mut addr);

            let mut video_save_pointer_table: u32 = 0;
            if self.card.is_vga() {
                self.video_dcc_table = addr.clone();
                mmu.write_u8_inc(&mut addr, 0x10); // number of entries
                mmu.write_u8_inc(&mut addr, 1);    // version number
                mmu.write_u8_inc(&mut addr, 8);    // maximum display code
                mmu.write_u8_inc(&mut addr, 0);    // reserved

                // display combination codes
                mmu.write_u16_inc(&mut addr, 0x0000);
                mmu.write_u16_inc(&mut addr, 0x0100);
                mmu.write_u16_inc(&mut addr, 0x0200);
                mmu.write_u16_inc(&mut addr, 0x0102);
                mmu.write_u16_inc(&mut addr, 0x0400);
                mmu.write_u16_inc(&mut addr, 0x0104);
                mmu.write_u16_inc(&mut addr, 0x0500);
                mmu.write_u16_inc(&mut addr, 0x0502);
                mmu.write_u16_inc(&mut addr, 0x0600);
                mmu.write_u16_inc(&mut addr, 0x0601);
                mmu.write_u16_inc(&mut addr, 0x0605);
                mmu.write_u16_inc(&mut addr, 0x0800);
                mmu.write_u16_inc(&mut addr, 0x0801);
                mmu.write_u16_inc(&mut addr, 0x0700);
                mmu.write_u16_inc(&mut addr, 0x0702);
                mmu.write_u16_inc(&mut addr, 0x0706);

                video_save_pointer_table = addr.value();
                mmu.write_u16_inc(&mut addr, 0x1A); // length of table
                mmu.write_u32_inc(&mut addr, self.video_dcc_table.value());
                mmu.write_u32_inc(&mut addr, 0); // alphanumeric charset override
                mmu.write_u32_inc(&mut addr, 0); // user palette table
                mmu.write_u32_inc(&mut addr, 0);
                mmu.write_u32_inc(&mut addr, 0);
                mmu.write_u32_inc(&mut addr, 0);
            }

            mmu.write_u32(BIOS::DATA_SEG, BIOS::DATA_VS_POINTER, addr.value());

            mmu.write_u32_inc(&mut addr, self.video_parameter_table.value());
            mmu.write_u32_inc(&mut addr, 0); // dynamic save area pointer
            mmu.write_u32_inc(&mut addr, 0); // alphanumeric character set override
            mmu.write_u32_inc(&mut addr, 0); // graphics character set override
            if self.card.is_vga() {
                mmu.write_u32_inc(&mut addr, video_save_pointer_table);
            }
            addr.inc_u32(); // skip value
            mmu.write_u32_inc(&mut addr, 0);
            mmu.write_u32_inc(&mut addr, 0);
        }

        if self.card.is_tandy() {
            mmu.write_vec(0x44, &self.font_8_first);
        }
    }
}
