use core::arch::asm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Backspace,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Tab,
    None,
}

impl Key {
    pub fn from_scancode(scancode: u8) -> Self {
        match scancode {
            0x01 => Key::Escape,
            0x0E => Key::Backspace,
            0x1C => Key::Enter,
            0x0F => Key::Tab,
            
            0x48 => Key::Up,
            0x50 => Key::Down,
            0x4B => Key::Left,
            0x4D => Key::Right,
            
            0x39 => Key::Char(' '),
            
            // Цифры
            0x02 => Key::Char('1'),
            0x03 => Key::Char('2'),
            0x04 => Key::Char('3'),
            0x05 => Key::Char('4'),
            0x06 => Key::Char('5'),
            0x07 => Key::Char('6'),
            0x08 => Key::Char('7'),
            0x09 => Key::Char('8'),
            0x0A => Key::Char('9'),
            0x0B => Key::Char('0'),
            
            // Знаки
            0x34 => Key::Char('.'),
            0x33 => Key::Char(','),
            0x35 => Key::Char('/'),
            0x2B => Key::Char('\\'),
            0x0C => Key::Char('-'),
            0x0D => Key::Char('='),
            0x27 => Key::Char(';'),
            0x28 => Key::Char('\''),
            0x29 => Key::Char('`'),
            0x1A => Key::Char('['),
            0x1B => Key::Char(']'),
            
            // Буквы (только нижний регистр, shift пока не обрабатываем)
            0x10 => Key::Char('q'),
            0x11 => Key::Char('w'),
            0x12 => Key::Char('e'),
            0x13 => Key::Char('r'),
            0x14 => Key::Char('t'),
            0x15 => Key::Char('y'),
            0x16 => Key::Char('u'),
            0x17 => Key::Char('i'),
            0x18 => Key::Char('o'),
            0x19 => Key::Char('p'),
            0x1E => Key::Char('a'),
            0x1F => Key::Char('s'),
            0x20 => Key::Char('d'),
            0x21 => Key::Char('f'),
            0x22 => Key::Char('g'),
            0x23 => Key::Char('h'),
            0x24 => Key::Char('j'),
            0x25 => Key::Char('k'),
            0x26 => Key::Char('l'),
            0x2C => Key::Char('z'),
            0x2D => Key::Char('x'),
            0x2E => Key::Char('c'),
            0x2F => Key::Char('v'),
            0x30 => Key::Char('b'),
            0x31 => Key::Char('n'),
            0x32 => Key::Char('m'),
            
            _ => Key::None,
        }
    }
}

pub fn get_key() -> Key {
    let status: u8;
    unsafe { asm!("in al, 0x64", out("al") status); }
    
    if status & 1 == 0 {
        return Key::None;
    }
    
    let scancode: u8;
    unsafe { asm!("in al, 0x60", out("al") scancode); }
    
    if scancode & 0x80 != 0 {
        return Key::None;
    }
    
    Key::from_scancode(scancode)
}

pub fn wait_key() -> Key {
    loop {
        let key = get_key();
        if key != Key::None {
            return key;
        }
        unsafe { asm!("hlt"); }
    }
}

// ===================== ДОБАВЛЕНЫ ГОРЯЧИЕ КЛАВИШИ =====================

static mut LEFT_CTRL: bool = false;
static mut LEFT_ALT: bool = false;
static mut LEFT_SHIFT: bool = false;
static mut RIGHT_SHIFT: bool = false;

// Сканкоды для модификаторов
const SCANCODE_LEFT_CTRL: u8 = 0x1D;
const SCANCODE_LEFT_ALT: u8 = 0x38;
const SCANCODE_LEFT_SHIFT: u8 = 0x2A;
const SCANCODE_RIGHT_SHIFT: u8 = 0x36;
const SCANCODE_ALT_RELEASE: u8 = 0xB8;
const SCANCODE_CTRL_RELEASE: u8 = 0x9D;
const SCANCODE_LSHIFT_RELEASE: u8 = 0xAA;
const SCANCODE_RSHIFT_RELEASE: u8 = 0xB6;

// F4 сканкод (нажатие)
const SCANCODE_F4: u8 = 0x3E;
const SCANCODE_F4_RELEASE: u8 = 0xBE;

// Escape для комбинаций
const SCANCODE_ESC: u8 = 0x01;

/// Обновить состояние модификаторов
pub fn update_modifiers(scancode: u8) {
    unsafe {
        match scancode {
            SCANCODE_LEFT_CTRL => LEFT_CTRL = true,
            SCANCODE_CTRL_RELEASE => LEFT_CTRL = false,
            SCANCODE_LEFT_ALT => LEFT_ALT = true,
            SCANCODE_ALT_RELEASE => LEFT_ALT = false,
            SCANCODE_LEFT_SHIFT => LEFT_SHIFT = true,
            SCANCODE_LSHIFT_RELEASE => LEFT_SHIFT = false,
            SCANCODE_RIGHT_SHIFT => RIGHT_SHIFT = true,
            SCANCODE_RSHIFT_RELEASE => RIGHT_SHIFT = false,
            _ => {}
        }
    }
}

/// Проверить нажатие Alt+F4
pub fn is_alt_f4(scancode: u8) -> bool {
    unsafe {
        LEFT_ALT && scancode == SCANCODE_F4
    }
}

/// Проверить нажатие Ctrl+Shift+Esc (открыть терминал)
pub fn is_ctrl_shift_esc(scancode: u8) -> bool {
    unsafe {
        scancode == 0x01 && LEFT_CTRL && (LEFT_SHIFT || RIGHT_SHIFT)
    }
}

/// Проверить нажатие Alt+Shift (открыть настройки)
pub fn is_alt_shift(scancode: u8) -> bool {
    unsafe {
        (scancode == SCANCODE_LEFT_SHIFT || scancode == SCANCODE_RIGHT_SHIFT) && LEFT_ALT
    }
}

/// Получить сканкод из клавиатуры без обработки модификаторов
pub fn get_raw_scancode() -> Option<u8> {
    let status: u8;
    unsafe { asm!("in al, 0x64", out("al") status); }
    
    if status & 1 == 0 {
        return None;
    }
    
    let scancode: u8;
    unsafe { asm!("in al, 0x60", out("al") scancode); }
    
    Some(scancode)
}

/// Сбросить все модификаторы (при перезагрузке)
pub fn reset_modifiers() {
    unsafe {
        LEFT_CTRL = false;
        LEFT_ALT = false;
        LEFT_SHIFT = false;
        RIGHT_SHIFT = false;
    }
}