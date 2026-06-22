// src/graphics.rs
use core::slice;
use crate::tamzen_font;

pub static mut BACKBUFFER: [u32; 800 * 600] = [0; 800 * 600];

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8, pub g: u8, pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self { Self { r, g, b } }
    pub fn to_u32(&self) -> u32 { ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32) }
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const YELLOW: Color = Color::rgb(255, 255, 0);
    pub const DARK_GRAY: Color = Color::rgb(60, 60, 60);
    pub const GRAY: Color = Color::rgb(75, 75, 75);
    pub const LIGHT_GRAY: Color = Color::rgb(200, 200, 200);
    pub const TITLE_BLUE: Color = Color::rgb(89, 0, 255);
    pub const DESKTOP_BLUE: Color = Color::rgb(76, 0, 255);
}

pub struct Graphics {
    pub fb: &'static mut [u32],
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
}

impl Graphics {
    pub fn new(addr: u64, width: u32, height: u32, pitch: u32) -> Self {
        let size = (pitch as usize / 4) * height as usize;
        let fb = unsafe { slice::from_raw_parts_mut(addr as *mut u32, size) };
        Self { fb, width: width as usize, height: height as usize, pitch: pitch as usize }
    }

    #[inline]
    pub fn put_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            unsafe { BACKBUFFER[y * self.width + x] = color; }
        }
    }

    pub fn flush(&mut self) {
        for y in 0..self.height {
            let src = y * self.width;
            let dst = y * (self.pitch / 4);
            let src_slice = unsafe {
                core::slice::from_raw_parts(
                    BACKBUFFER.as_ptr().add(src),
                    self.width,
                )
            };
            self.fb[dst..dst + self.width].copy_from_slice(src_slice);
        }
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for dy in 0..h { for dx in 0..w { self.put_pixel(x + dx, y + dy, color); } }
    }

    pub fn draw_rect_border(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for dx in 0..w { self.put_pixel(x + dx, y, color); self.put_pixel(x + dx, y + h - 1, color); }
        for dy in 0..h { self.put_pixel(x, y + dy, color); self.put_pixel(x + w - 1, y + dy, color); }
    }

    pub fn draw_text(&mut self, x: usize, y: usize, text: &str, fg: u32, bg: u32) {
        for (i, ch) in text.chars().enumerate() {
            if ch < ' ' || ch > '~' { continue; }
            let char_index = (ch as usize) - 0x20;
            let base = char_index * tamzen_font::FONT_HEIGHT;
            for row in 0..tamzen_font::FONT_HEIGHT {
                let byte = tamzen_font::FONT[base + row];
                for col in 0..tamzen_font::FONT_WIDTH {
                    let pixel_x = x + i * tamzen_font::FONT_WIDTH + col;
                    let pixel_y = y + row;
                    if byte & (1 << (7 - col)) != 0 {
                        self.put_pixel(pixel_x, pixel_y, fg);
                    } else if fg != bg {
                        self.put_pixel(pixel_x, pixel_y, bg);
                    }
                }
            }
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.fill_rect(0, 0, self.width, self.height, color);
    }
}
