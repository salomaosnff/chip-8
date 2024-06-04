const DISPLAY_WIDTH: usize = 64;
const DISPLAY_HEIGHT: usize = 32;
const DISPLAY_SIZE: usize = (DISPLAY_WIDTH / 8) * DISPLAY_HEIGHT;

pub enum KeyMask {
    Key0 = 1,
    Key1 = 1 << 1,
    Key2 = 1 << 2,
    Key3 = 1 << 3,
    Key4 = 1 << 4,
    Key5 = 1 << 5,
    Key6 = 1 << 6,
    Key7 = 1 << 7,
    Key8 = 1 << 8,
    Key9 = 1 << 9,
    KeyA = 1 << 10,
    KeyB = 1 << 11,
    KeyC = 1 << 12,
    KeyD = 1 << 13,
    KeyE = 1 << 14,
    KeyF = 1 << 15,
}

pub struct Chip8 {
    memory: [u8; 4096],
    v: [u8; 16],
    i: u16,
    pc: u16,
    stack: [u16; 16],
    sp: u8,
    delay_timer: u8,
    pub sound_timer: u8,
    pub keypad: u16,
    pub old_keypad: u16,
    pub display: [u8; DISPLAY_SIZE],
    pub halted: bool,
}

impl Chip8 {
    pub fn new() -> Chip8 {
        Chip8 {
            memory: [0; 4096],
            v: [0; 16],
            i: 0,
            pc: 0x200,
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            old_keypad: 0,
            keypad: 0,
            display: [0; DISPLAY_SIZE],
            halted: false,
        }
    }

    pub fn reset(&mut self) {
        println!("Resetando...");
        self.memory = [0; 4096];
        self.v = [0; 16];
        self.i = 0;
        self.pc = 0x200;
        self.stack = [0; 16];
        self.sp = 0;
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.keypad = 0;
        self.display = [0; DISPLAY_SIZE];
        self.halted = false;
    }

    pub fn halt(&mut self) {
        self.halted = true;
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.reset();
        println!("Carregando ROM...");
        for (i, &byte) in rom.iter().enumerate() {
            self.memory[i + 0x200] = byte;
        }
    }

    pub fn tick(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }

        self.emulate_cycle();
    }

    pub fn emulate_cycle(&mut self) {
        let opcode =
            (self.memory[self.pc as usize] as u16) << 8 | self.memory[self.pc as usize + 1] as u16;

        match opcode {
            0x00E0 => self.op_clr(),
            0x00EE => self.op_rts(),
            0x1000..=0x1FFF => self.op_jmp(opcode),
            0x2000..=0x2FFF => self.op_call(opcode),
            0x00C0..=0x00CF => self.op_scrd(opcode),
            0x0000..=0x0FFF => self.op_sys(),
            0x3000..=0x3FFF => self.op_ske(opcode),
            0x4000..=0x4FFF => self.op_skne(opcode),
            0x5000..=0x5FFF => self.op_skre(opcode),
            0x6000..=0x6FFF => self.op_load(opcode),
            0x7000..=0x7FFF => self.op_add(opcode),
            0x8000..=0x8FFF => match opcode & 0x000F {
                0x0000 => self.op_move(opcode),
                0x0001 => self.op_or(opcode),
                0x0002 => self.op_and(opcode),
                0x0003 => self.op_xor(opcode),
                0x0004 => self.op_addr(opcode),
                0x0005 => self.op_sub(opcode),
                0x0006 => self.op_shr(opcode),
                0x0007 => self.op_subn(opcode),
                0x000E => self.op_shl(opcode),
                _ => println!("Unknown opcode: {:#X}", opcode),
            },
            0x9000..=0x9FFF => self.op_skrne(opcode),
            0xA000..=0xAFFF => self.op_loadi(opcode),
            0xB000..=0xBFFF => self.op_jumpi(opcode),
            0xC000..=0xCFFF => self.op_rand(opcode),
            0xD000..=0xDFFF => self.op_draw(opcode),
            0xE000..=0xEFFF => match opcode & 0x00FF {
                0x009E => self.op_spr(opcode),
                0x00A1 => self.op_skup(opcode),
                _ => println!("Unknown opcode: {:#X}", opcode),
            },
            0xF000..=0xFFFF => match opcode & 0x00FF {
                0x07 => self.op_moved(opcode),
                0x0A => self.op_keyd(opcode),
                0x15 => self.op_loadd(opcode),
                0x18 => self.op_loads(opcode),
                0x1E => self.op_addi(opcode),
                0x29 => self.op_ldspr(opcode),
                0x33 => self.op_bcd(opcode),
                0x55 => self.op_stor(opcode),
                0x65 => self.op_read(opcode),
                _ => println!("Unknown opcode: {:#X}", opcode),
            },
        }

        self.old_keypad = self.keypad;
    }

    fn op_sys(&mut self) {
        self.pc += 2;
    }

    fn op_clr(&mut self) {
        self.pc += 2;
        self.display = [0; DISPLAY_SIZE];
    }

    fn op_rts(&mut self) {
        self.pc += 2;
        self.pc = self.stack[self.sp as usize];
        self.sp -= 1;
    }

    fn op_jmp(&mut self, opcode: u16) {
        self.pc += 2;
        self.pc = opcode & 0x0FFF;
    }

    fn op_call(&mut self, opcode: u16) {
        self.pc += 2;
        self.sp += 1;
        self.stack[self.sp as usize] = self.pc;
        self.pc = opcode & 0x0FFF;
    }

    fn op_ske(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let nn = (opcode & 0x00FF) as u8;

        if self.v[x] == nn {
            self.pc += 2;
        }
    }

    fn op_skne(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let nn = (opcode & 0x00FF) as u8;

        if self.v[x] != nn {
            self.pc += 2;
        }
    }

    fn op_skre(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;

        if self.v[x] == self.v[y] {
            self.pc += 2;
        }
    }

    fn op_load(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let nn = (opcode & 0x00FF) as u8;

        self.v[x] = nn;
    }

    fn op_add(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let nn = (opcode & 0x00FF) as u8;

        self.v[x] = self.v[x].wrapping_add(nn);
    }

    fn op_move(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;

        self.v[x] = self.v[y];
    }

    fn op_or(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;

        self.v[x] |= self.v[y];
    }

    fn op_and(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;

        self.v[x] &= self.v[y];
    }

    fn op_xor(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;

        self.v[x] ^= self.v[y];
    }

    fn op_addr(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;

        let (result, overflow) = self.v[x].overflowing_add(self.v[y]);
        self.v[x] = result;
        self.v[0xF] = overflow as u8;
    }

    fn op_sub(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;

        let (result, overflow) = self.v[x].overflowing_sub(self.v[y]);
        self.v[x] = result;
        self.v[0xF] = !overflow as u8;
    }

    fn op_subn(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;

        let (result, overflow) = self.v[y].overflowing_sub(self.v[x]);
        self.v[x] = result;
        self.v[0xF] = !overflow as u8;
    }

    fn op_shr(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;

        self.v[0xF] = self.v[x] & 0x1;
        self.v[x] >>= 1;
    }

    fn op_shl(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;

        self.v[0xF] = self.v[x] >> 7;
        self.v[x] <<= 1;
    }

    fn op_skrne(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;

        if self.v[x] != self.v[y] {
            self.pc += 2;
        }
    }

    fn op_loadi(&mut self, opcode: u16) {
        self.pc += 2;
        self.i = opcode & 0x0FFF;
    }

    fn op_jumpi(&mut self, opcode: u16) {
        self.pc += 2;
        self.pc = self.v[0] as u16 + (opcode & 0x0FFF);
    }

    fn op_rand(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let nn = (opcode & 0x00FF) as u8;

        self.v[x] = rand::random::<u8>() & nn;
    }

    fn op_draw(&mut self, opcode: u16) {
        self.pc += 2;
        let s = ((opcode & 0x0F00) >> 8) as usize;
        let t = ((opcode & 0x00F0) >> 4) as usize;
        let n = (opcode & 0x000F) as usize;

        let x = self.v[s] as usize;
        let y = self.v[t] as usize;

        self.v[0xF] = 0;

        for yline in 0..n {
            let pixel = self.memory[(self.i + yline as u16) as usize];
            for xline in 0..8 {
                if (pixel & (0x80 >> xline)) != 0 {
                    let x = (x + xline) % DISPLAY_WIDTH;
                    let y = (y + yline) % DISPLAY_HEIGHT;
                    let index = y * DISPLAY_WIDTH / 8 + x / 8;
                    let mask = 0x80 >> (x % 8);
                    if self.display[index] & mask != 0 {
                        self.v[0xF] = 1;
                    }
                    self.display[index] ^= mask;
                }
            }
        }
    }

    fn op_spr(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let is_pressed = self.keypad & (1 << self.v[x]) != 0;

        if is_pressed {
            self.pc += 2;
        }
    }

    fn op_skup(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let is_pressed = self.keypad & (1 << self.v[x]) != 0;

        if !is_pressed {
            self.pc += 2;
        }
    }

    fn op_moved(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;

        self.v[x] = self.delay_timer;
    }

    fn op_keyd(&mut self, opcode: u16) {
        if self.old_keypad == self.keypad {
            return;
        }

        let x = ((opcode & 0x0F00) >> 8) as usize;

        self.pc += 2;

        for i in 0..16 {
            if self.keypad & (1 << i) != 0 {
                self.v[x] = i as u8;
                return;
            }
        }
    }

    fn op_loadd(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;

        self.delay_timer = self.v[x];
    }

    fn op_loads(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;

        self.sound_timer = self.v[x];
    }

    fn op_addi(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;

        self.i += self.v[x] as u16;
    }

    fn op_ldspr(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;

        self.i = self.v[x] as u16 * 5;
    }

    fn op_bcd(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let mut value = self.v[x];

        for i in 0..3 {
            self.memory[(self.i + 2 - i) as usize] = value % 10;
            value /= 10;
        }
    }

    fn op_stor(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;

        for i in 0..=x {
            self.memory[self.i as usize + i] = self.v[i];
        }
    }

    fn op_read(&mut self, opcode: u16) {
        self.pc += 2;
        let x = ((opcode & 0x0F00) >> 8) as usize;

        for i in 0..=x {
            self.v[i] = self.memory[self.i as usize + i];
        }
    }

    pub fn op_scrd(&mut self, opcode: u16) {
        self.pc += 2;
        let n = (opcode & 0x000F) as usize;
        for i in 0..n {
            self.display[i] = 0;
        }
    }

    pub fn set_keypad(&mut self, keypad: u16) {
        self.old_keypad = self.keypad;
        self.keypad = keypad;
    }

    pub fn on_key_down(&mut self, keys: u16) {
        self.set_keypad(self.keypad | keys);
    }

    pub fn on_key_up(&mut self, keys: u16) {
        self.set_keypad(self.keypad & !keys);
    }
}
