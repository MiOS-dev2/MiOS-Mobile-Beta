#![no_std]
#![allow(dead_code)]
#![feature(abi_x86_interrupt)]
#![allow(static_mut_refs)]

mod vga;
mod vesa;
mod multiboot;
mod keyboard;
mod shell;
mod commands;
mod ata;
mod fs;
mod console;
mod vesa_gui;
mod utils;
mod gui;
mod graphics;
mod wm;
mod tamzen_font;
mod idt;
mod mouse;
mod bmp;

use core::panic::PanicInfo;
use multiboot::MultibootInfo;
use graphics::{Graphics, Color};
use wm::{WindowManager, WindowState};
use bmp::BmpImage;

pub static mut VESA_INFO: Option<vesa::VesaInfo> = None;


#[link_section = ".rodata"]
#[used]
static IMG_BMP: &[u8] = include_bytes!("img.bmp");

#[link_section = ".rodata"]
#[used]
static IMG2_BMP: &[u8] = include_bytes!("img2.bmp");

#[link_section = ".rodata"]
#[used]
static IMG3_BMP: &[u8] = include_bytes!("img3.bmp");

#[link_section = ".rodata"]
#[used]
static IC_BMP: &[u8] = include_bytes!("ic.bmp");

#[link_section = ".rodata"]
#[used]
static IC0_BMP: &[u8] = include_bytes!("ic0.bmp");

#[link_section = ".rodata"]
#[used]
static IC1_BMP: &[u8] = include_bytes!("ic1.bmp");

#[link_section = ".rodata"]
#[used]
static IC2_BMP: &[u8] = include_bytes!("ic2.bmp");

#[link_section = ".rodata"]
#[used]
static IC3_BMP: &[u8] = include_bytes!("ic3.bmp");

#[link_section = ".rodata"]
#[used]
static IC4_BMP: &[u8] = include_bytes!("ic4.bmp");

#[link_section = ".rodata"]
#[used]
static IC5_BMP: &[u8] = include_bytes!("ic5.bmp");

#[link_section = ".rodata"]
#[used]
static ABOUT_BMP: &[u8] = include_bytes!("about.bmp");


static mut WALLPAPER: Color = Color::rgb(5, 148, 0);
static mut USE_BMP_WALLPAPER: bool = true;
static mut BMP_IMAGE: Option<BmpImage> = None;
static mut BMP_IMAGE2: Option<BmpImage> = None;
static mut BMP_IMAGE3: Option<BmpImage> = None;
static mut CURRENT_BMP_INDEX: usize = 0;


static mut ICON_TERMINAL: Option<BmpImage> = None;
static mut ICON_FILES: Option<BmpImage> = None;
static mut ICON_SETTINGS: Option<BmpImage> = None;
static mut ICON_ABOUT: Option<BmpImage> = None;
static mut ICON_NOTEPAD: Option<BmpImage> = None;
static mut ICON_SNAKE: Option<BmpImage> = None;
static mut ICON_PAINT: Option<BmpImage> = None;
static mut ABOUT_IMAGE: Option<BmpImage> = None;

static mut THEME_IDX: usize = 0;

static mut TERM_HISTORY: [[u8; 80]; 20] = [[b' '; 80]; 20];
static mut TERM_HIST_LEN: usize = 0;
static mut NOTEPAD_TEXT: [u8; 1024] = [0; 1024];
static mut NOTEPAD_LEN: usize = 0;
static mut SAVED_DOCUMENTS: [[u8; 512]; 5] = [[0; 512]; 5];
static mut SAVED_DOCUMENTS_LEN: [usize; 5] = [0; 5];
static mut SAVED_DOCUMENTS_COUNT: usize = 0;
static mut WALLPAPER_SELECTOR_OPEN: bool = false;
static mut THEME_SELECTOR_OPEN: bool = false;
static mut DOCUMENT_MANAGER_OPEN: bool = false;
static mut DOCUMENT_MANAGER_MODE: u8 = 0;
static mut GRADIENT_TOP_ACTIVE: Color = Color::rgb(90, 90, 90);
static mut GRADIENT_BOTTOM_ACTIVE: Color = Color::rgb(20, 20, 20);
static mut GRADIENT_TOP_INACTIVE: Color = Color::rgb(105, 105, 105);
static mut GRADIENT_BOTTOM_INACTIVE: Color = Color::rgb(50, 50, 50);

static mut LOGGED_IN: bool = false;

// Данные для Paint
static mut PAINT_CANVAS: [[u32; 400]; 300] = [[0; 400]; 300];
static mut PAINT_COLOR: u32 = 0x000000;
static mut PAINT_PEN_SIZE: usize = 1;
static mut PAINT_TOOL: u8 = 0;


fn draw_icon(gfx: &mut Graphics, x: usize, y: usize, icon: &Option<BmpImage>, size: usize) {
    if let Some(ref bmp) = icon {
        let row_size = ((bmp.width * 3 + 3) / 4) * 4;
        for dy in 0..size.min(bmp.height) {
            let bmp_y = (bmp.height - 1).saturating_sub(dy);
            let bmp_row_start = bmp_y * row_size;
            for dx in 0..size.min(bmp.width) {
                let pixel_offset = bmp_row_start + dx * 3;
                if pixel_offset + 2 < bmp.data.len() {
                    let b = bmp.data[pixel_offset] as u32;
                    let g = bmp.data[pixel_offset + 1] as u32;
                    let r = bmp.data[pixel_offset + 2] as u32;
                    let color = (r << 16) | (g << 8) | b;
                    if color != 0xFF00FF {
                        gfx.put_pixel(x + dx, y + dy, color);
                    }
                }
            }
        }
    }
}

// Функция для рисования линии
fn draw_line(gfx: &mut Graphics, x1: usize, y1: usize, x2: usize, y2: usize, color: u32) {
    let mut x = x1 as i32;
    let mut y = y1 as i32;
    let dx = (x2 as i32 - x1 as i32).abs();
    let dy = -(y2 as i32 - y1 as i32).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x >= 0 && x < gfx.width as i32 && y >= 0 && y < gfx.height as i32 {
            gfx.put_pixel(x as usize, y as usize, color);
        }
        if x == x2 as i32 && y == y2 as i32 { break; }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}


#[no_mangle]
pub extern "C" fn rust_main(magic: u32, info_addr: u32) -> ! {
    if magic != 0x36d76289 {
        loop {}
    }

    let mb = unsafe { MultibootInfo::parse(info_addr) };

    if let Some(fb) = mb.framebuffer {
        if fb.addr != 0 && fb.width > 0 && fb.height > 0 {
            let vesa_info = vesa::VesaInfo {
                addr: fb.addr as usize,
                width: fb.width as usize,
                height: fb.height as usize,
                pitch: fb.pitch as usize,
                bpp: fb.bpp,
            };
            unsafe { VESA_INFO = Some(vesa_info); }

            let mut gfx = Graphics::new(fb.addr, fb.width, fb.height, fb.pitch);

            unsafe {
                BMP_IMAGE = BmpImage::from_bytes(IMG_BMP);
                BMP_IMAGE2 = BmpImage::from_bytes(IMG2_BMP);
                BMP_IMAGE3 = BmpImage::from_bytes(IMG3_BMP);
                ICON_TERMINAL = BmpImage::from_bytes(IC_BMP);
                ICON_FILES = BmpImage::from_bytes(IC0_BMP);
                ICON_SETTINGS = BmpImage::from_bytes(IC1_BMP);
                ICON_ABOUT = BmpImage::from_bytes(IC2_BMP);
                ICON_NOTEPAD = BmpImage::from_bytes(IC3_BMP);
                ICON_SNAKE = BmpImage::from_bytes(IC4_BMP);
                ICON_PAINT = BmpImage::from_bytes(IC5_BMP);
                ABOUT_IMAGE = BmpImage::from_bytes(ABOUT_BMP);
                
                for y in 0..300 {
                    for x in 0..400 {
                        PAINT_CANVAS[y][x] = 0xFFFFFF;
                    }
                }
                PAINT_COLOR = 0x000000;
                PAINT_PEN_SIZE = 1;
                PAINT_TOOL = 0;
            }

            let mut wm = WindowManager::new();

            let screen_w = gfx.width;
            let screen_h = gfx.height;

            gfx.clear(Color::BLACK.to_u32());

            let title = "MiOS";
            let title_width = title.len() * tamzen_font::FONT_WIDTH;
            let title_x = (screen_w - title_width) / 2;
            let title_y = screen_h / 2 - 40;
            gfx.draw_text(title_x, title_y, title, Color::WHITE.to_u32(), Color::BLACK.to_u32());

            let bar_width = 300;
            let bar_height = 24;
            let bar_x = (screen_w - bar_width) / 2;
            let bar_y = screen_h / 2 + 10;

            gfx.draw_rect_border(bar_x - 1, bar_y - 1, bar_width + 2, bar_height + 2, Color::WHITE.to_u32());
            gfx.flush();

            let total_ticks = 400;
            for tick in 0..=total_ticks {
                let progress = (tick as usize * bar_width) / total_ticks as usize;
                if progress > 0 {
                    gfx.fill_rect(bar_x, bar_y, progress, bar_height, Color::WHITE.to_u32());
                    if progress < bar_width {
                        gfx.fill_rect(bar_x + progress, bar_y, bar_width - progress, bar_height, Color::BLACK.to_u32());
                    }
                }
                gfx.flush();
                for _ in 0..100000 {
                    unsafe { core::arch::asm!("nop") }
                }
            }

            for _ in 0..50000 {
                unsafe { core::arch::asm!("nop") }
            }

            let mut term_input: [u8; 256] = [0; 256];
            let mut term_len = 0;

            fn push_history(line: &str) {
                unsafe {
                    if TERM_HIST_LEN < 20 {
                        let bytes = line.as_bytes();
                        let len = if bytes.len() > 80 { 80 } else { bytes.len() };
                        TERM_HISTORY[TERM_HIST_LEN][..len].copy_from_slice(&bytes[..len]);
                        TERM_HIST_LEN += 1;
                    }
                }
            }
            fn clear_history() { unsafe { TERM_HIST_LEN = 0; } }
            fn exec_command(input: &str) -> &'static str {
                match input.trim() {
                    "help"   => "Commands: help, ver, mem, uptime, dice, cls, about",
                    "ver"    => "MiOS v5.5 beta",
                    "mem"    => "Memory: 128 MB (VESA 800x600x32)",
                    "uptime" => "Uptime: 0 ticks",
                    "dice"   => "Dice: 4",
                    "cls"    => "\x04",
                    "about"  => "MiOS v5.4 ALPHA - Made in 2026",
                    ""       => "",
                    _        => "Unknown command",
                }
            }

            fn draw_window_content(gfx: &mut Graphics, win: &wm::Window, idx: usize) {
                if win.state == WindowState::Minimized {
                    return;
                }

                let t = wm::get_theme();
                let cx = win.x + 2;
                let cy = win.y + 26;
                match win.title {
                    "Terminal" => {
                        let history = unsafe { &TERM_HISTORY };
                        let hist_len = unsafe { TERM_HIST_LEN };
                        for row in 0..hist_len {
                            let line = core::str::from_utf8(&history[row]).unwrap_or("");
                            gfx.draw_text(cx + 10, cy + 10 + row * 16, line, Color::BLACK.to_u32(), t.window_bg.to_u32());
                        }
                        let prompt_y = cy + 10 + hist_len * 16;
                        gfx.draw_text(cx + 10, prompt_y, "MiOS> ", Color::BLACK.to_u32(), t.window_bg.to_u32());
                    }
                    "AboutOS" => {
                        let about_img = unsafe { &ABOUT_IMAGE };
                        if let Some(ref bmp) = about_img {
                            let img_x = cx + (win.width - 4 - bmp.width) / 2;
                            let img_y = cy + 10;
                            let row_size = ((bmp.width * 3 + 3) / 4) * 4;
                            for dy in 0..bmp.height {
                                let bmp_y = (bmp.height - 1).saturating_sub(dy);
                                let bmp_row_start = bmp_y * row_size;
                                for dx in 0..bmp.width {
                                    let pixel_offset = bmp_row_start + dx * 3;
                                    if pixel_offset + 2 < bmp.data.len() {
                                        let b = bmp.data[pixel_offset] as u32;
                                        let g = bmp.data[pixel_offset + 1] as u32;
                                        let r = bmp.data[pixel_offset + 2] as u32;
                                        let color = (r << 16) | (g << 8) | b;
                                        gfx.put_pixel(img_x + dx, img_y + dy, color);
                                    }
                                }
                            }
                            let info_y = img_y + bmp.height + 20;
                            gfx.draw_text(cx + 10, info_y,        "═══════════════════════", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, info_y + 20,   "MiOS Beta",         Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, info_y + 40,   "Created in 2026",         Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, info_y + 60,   "Thank you for using!",    Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, info_y + 100,  "═══════════════════════", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        } else {
                            gfx.draw_text(cx + 10, cy + 10,  "About MiOS",              Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 30,  "═══════════════════════", Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 50,  "MiOS Beta",          Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 70,  "Created in 2026",          Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 90,  "Thank you for using!",     Color::BLACK.to_u32(), t.window_bg.to_u32());
                            gfx.draw_text(cx + 10, cy + 130, "═══════════════════════", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        }
                    }
                    "Settings" => {
                        gfx.draw_text(cx + 10, cy + 10, "Settings", Color::BLACK.to_u32(), t.window_bg.to_u32());

                        let btn_y = cy + 30;
                        wm::draw_raised_rect(gfx, cx + 10, btn_y, 180, 24);
                        gfx.draw_text(cx + 20, btn_y + 6, "Change Wallpaper", Color::BLACK.to_u32(), t.button_face.to_u32());

                        let theme_btn_y = cy + 60;
                        wm::draw_raised_rect(gfx, cx + 10, theme_btn_y, 180, 24);
                        gfx.draw_text(cx + 20, theme_btn_y + 6, "Change Theme", Color::BLACK.to_u32(), t.button_face.to_u32());

                        let gradient_btn_y = cy + 90;
                        wm::draw_raised_rect(gfx, cx + 10, gradient_btn_y, 180, 24);
                        gfx.draw_text(cx + 20, gradient_btn_y + 6, "Window Gradient", Color::BLACK.to_u32(), t.button_face.to_u32());

                        let bmp_btn_y = cy + 120;
                        wm::draw_raised_rect(gfx, cx + 10, bmp_btn_y, 180, 24);
                        let bmp_status = if unsafe { USE_BMP_WALLPAPER } { "BMP Wallpaper: ON" } else { "BMP Wallpaper: OFF" };
                        gfx.draw_text(cx + 20, bmp_btn_y + 6, bmp_status, Color::BLACK.to_u32(), t.button_face.to_u32());

                        let wallpaper_select_btn_y = cy + 150;
                        wm::draw_raised_rect(gfx, cx + 10, wallpaper_select_btn_y, 180, 24);
                        gfx.draw_text(cx + 20, wallpaper_select_btn_y + 6, "Change Wallpaper", Color::BLACK.to_u32(), t.button_face.to_u32());

                        gfx.draw_text(cx + 10, cy + 180, "Resolution: 800x600",  Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 200, "MiOS v5.4 (2026)",     Color::BLACK.to_u32(), t.window_bg.to_u32());

                        let tidx = unsafe { THEME_IDX };
                        let name = match tidx { 0 => "Warm", 1 => "Classic", 2 => "Cool", _ => "???" };
                        gfx.draw_text(cx + 10, cy + 220, "Current Theme: ", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10 + 14*8, cy + 220, name, Color::BLACK.to_u32(), t.window_bg.to_u32());
                    }
                    "Paint" => {
                        gfx.fill_rect(cx, cy, win.width - 4, 40, Color::rgb(200, 200, 200).to_u32());
                        gfx.fill_rect(cx, cy + 40, win.width - 4, 1, Color::rgb(128, 128, 128).to_u32());
                        
                        wm::draw_raised_rect(gfx, cx + 5, cy + 5, 30, 30);
                        gfx.fill_rect(cx + 10, cy + 15, 20, 10, unsafe { PAINT_COLOR });
                        
                        wm::draw_raised_rect(gfx, cx + 40, cy + 5, 30, 30);
                        draw_line(gfx, cx + 45, cy + 25, cx + 65, cy + 15, Color::BLACK.to_u32());
                        
                        wm::draw_raised_rect(gfx, cx + 75, cy + 5, 30, 30);
                        gfx.draw_rect_border(cx + 82, cy + 10, 16, 16, Color::BLACK.to_u32());
                        
                        let colors = [
                            0x000000, 0xFF0000, 0x00FF00, 0x0000FF,
                            0xFFFF00, 0xFF00FF, 0x00FFFF, 0xFFFFFF,
                        ];
                        for i in 0..8 {
                            let color_x = cx + 120 + i * 25;
                            wm::draw_raised_rect(gfx, color_x, cy + 5, 20, 30);
                            gfx.fill_rect(color_x + 3, cy + 8, 14, 24, colors[i]);
                        }
                        
                        let sizes = ["1", "3", "5"];
                        for i in 0..3 {
                            let size_x = cx + 330 + i * 30;
                            wm::draw_raised_rect(gfx, size_x, cy + 5, 25, 30);
                            gfx.draw_text(size_x + 8, cy + 15, sizes[i], Color::BLACK.to_u32(), Color::rgb(200, 200, 200).to_u32());
                        }
                        
                        let canvas_x = cx + 5;
                        let canvas_y = cy + 45;
                        let canvas_w = (win.width - 10).min(400);
                        let canvas_h = (win.height - 55).min(300);
                        
                        gfx.fill_rect(canvas_x, canvas_y, canvas_w, canvas_h, 0xFFFFFF);
                        gfx.draw_rect_border(canvas_x, canvas_y, canvas_w, canvas_h, Color::BLACK.to_u32());
                        
                        unsafe {
                            for y in 0..canvas_h.min(300) {
                                for x in 0..canvas_w.min(400) {
                                    gfx.put_pixel(canvas_x + x, canvas_y + y, PAINT_CANVAS[y][x]);
                                }
                            }
                        }
                        
                        let tool_name = unsafe {
                            match PAINT_TOOL {
                                0 => "Brush",
                                1 => "Line",
                                2 => "Rect",
                                _ => "Brush",
                            }
                        };
                        let color_val = unsafe { PAINT_COLOR };
                        let color_r = (color_val >> 16) & 0xFF;
                        let color_g = (color_val >> 8) & 0xFF;
                        let color_b = color_val & 0xFF;
                        
                        let mut color_str = [0u8; 20];
                        color_str[0] = b'C';
                        color_str[1] = b'o';
                        color_str[2] = b'l';
                        color_str[3] = b'o';
                        color_str[4] = b'r';
                        color_str[5] = b':';
                        color_str[6] = b' ';
                        color_str[7] = b'#';
                        let hex_chars = b"0123456789ABCDEF";
                        color_str[8] = hex_chars[(color_r >> 4) as usize];
                        color_str[9] = hex_chars[(color_r & 0xF) as usize];
                        color_str[10] = hex_chars[(color_g >> 4) as usize];
                        color_str[11] = hex_chars[(color_g & 0xF) as usize];
                        color_str[12] = hex_chars[(color_b >> 4) as usize];
                        color_str[13] = hex_chars[(color_b & 0xF) as usize];
                        color_str[14] = 0;
                        
                        let color_info = core::str::from_utf8(&color_str[..14]).unwrap_or("Color: #------");
                        gfx.draw_text(cx + 5, cy + win.height - 25, tool_name, Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 100, cy + win.height - 25, color_info, Color::BLACK.to_u32(), t.window_bg.to_u32());
                    }
                    "Wallpaper Selector" => {
                        gfx.draw_text(cx + 10, cy + 10, "Select Wallpaper Color:", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        let colors = [
                            ("Sky Blue",  Color::rgb(89, 0, 255)),
                            ("Dark Teal", Color::rgb(0, 23, 128)),
                            ("Red",       Color::rgb(192, 0, 0)),
                            ("Yellow",    Color::rgb(192, 192, 0)),
                            ("Dark Blue", Color::rgb(0, 0, 128)),
                            ("Purple",    Color::rgb(128, 0, 128)),
                        ];
                        for (i, (name, color)) in colors.iter().enumerate() {
                            let y_pos = cy + 30 + i * 30;
                            wm::draw_raised_rect(gfx, cx + 10, y_pos, 250, 24);
                            gfx.fill_rect(cx + 15, y_pos + 4, 16, 16, color.to_u32());
                            gfx.draw_text(cx + 40, y_pos + 6, name, Color::BLACK.to_u32(), t.button_face.to_u32());
                        }
                    }
                    "BMP Wallpaper Selector" => {
                        gfx.draw_text(cx + 10, cy + 10, "Select BMP Wallpaper:", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        
                        let wallpapers = [
                            ("Wallpaper 1", 0),
                            ("Wallpaper 2", 1),
                            ("Wallpaper 3", 2),
                        ];
                        
                        for (i, (name, idx)) in wallpapers.iter().enumerate() {
                            let y_pos = cy + 30 + i * 60;
                            wm::draw_raised_rect(gfx, cx + 10, y_pos, 250, 50);
                            
                            let preview_bmp = match idx {
                                0 => unsafe { &BMP_IMAGE },
                                1 => unsafe { &BMP_IMAGE2 },
                                2 => unsafe { &BMP_IMAGE3 },
                                _ => unsafe { &BMP_IMAGE },
                            };
                            
                            if let Some(ref bmp) = preview_bmp {
                                let row_size = ((bmp.width * 3 + 3) / 4) * 4;
                                for dy in 0..40.min(bmp.height) {
                                    let bmp_y = (bmp.height - 1).saturating_sub(dy);
                                    let bmp_row_start = bmp_y * row_size;
                                    for dx in 0..40.min(bmp.width) {
                                        let pixel_offset = bmp_row_start + dx * 3;
                                        if pixel_offset + 2 < bmp.data.len() {
                                            let b = bmp.data[pixel_offset] as u32;
                                            let g = bmp.data[pixel_offset + 1] as u32;
                                            let r = bmp.data[pixel_offset + 2] as u32;
                                            let color = (r << 16) | (g << 8) | b;
                                            gfx.put_pixel(cx + 15 + dx, y_pos + 5 + dy, color);
                                        }
                                    }
                                }
                            }
                            
                            gfx.draw_text(cx + 65, y_pos + 20, name, Color::BLACK.to_u32(), t.button_face.to_u32());
                        }
                    }
                    "Theme Selector" => {
                        gfx.draw_text(cx + 10, cy + 10, "Select Window Theme:", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        let themes = [("Warm", 0usize), ("Classic", 1), ("Cool (Light)", 2)];
                        for (i, (name, _)) in themes.iter().enumerate() {
                            let y_pos = cy + 30 + i * 30;
                            wm::draw_raised_rect(gfx, cx + 10, y_pos, 250, 24);
                            gfx.draw_text(cx + 20, y_pos + 6, name, Color::BLACK.to_u32(), t.button_face.to_u32());
                        }
                    }
                    "Window Gradient" => {
                        gfx.draw_text(cx + 10, cy + 10, "Window Titlebar Gradient", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 30, "Active window:", Color::BLACK.to_u32(), t.window_bg.to_u32());

                        let preview_w = 200;
                        let preview_h = 20;
                        let preview_x = cx + 10;
                        let preview_y = cy + 50;
                        let top_active    = unsafe { GRADIENT_TOP_ACTIVE };
                        let bottom_active = unsafe { GRADIENT_BOTTOM_ACTIVE };
                        for row in 0..preview_h {
                            let t_val = row as f32 / (preview_h as f32 - 1.0).max(1.0);
                            let r = (top_active.r as f32 + (bottom_active.r as f32 - top_active.r as f32) * t_val) as u8;
                            let g = (top_active.g as f32 + (bottom_active.g as f32 - top_active.g as f32) * t_val) as u8;
                            let b = (top_active.b as f32 + (bottom_active.b as f32 - top_active.b as f32) * t_val) as u8;
                            let color = Color::rgb(r, g, b).to_u32();
                            for dx in 0..preview_w { gfx.put_pixel(preview_x + dx, preview_y + row, color); }
                        }

                        let colors_avail = [
                            ("Red",    Color::rgb(255, 0, 0)),
                            ("Green",  Color::rgb(0, 255, 0)),
                            ("Blue",   Color::rgb(0, 0, 255)),
                            ("White",  Color::WHITE),
                            ("Black",  Color::BLACK),
                            ("Gray",   Color::rgb(128, 128, 128)),
                            ("Orange", Color::rgb(255, 165, 0)),
                            ("Purple", Color::rgb(128, 0, 128)),
                        ];

                        gfx.draw_text(cx + 10, cy + 80, "Top color:", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        let mut btn_x = cx + 100;
                        for (name, color) in colors_avail.iter() {
                            if btn_x + 50 > cx + win.width - 4 { break; }
                            wm::draw_raised_rect(gfx, btn_x, cy + 76, 45, 18);
                            gfx.fill_rect(btn_x + 2, cy + 78, 14, 14, color.to_u32());
                            gfx.draw_text(btn_x + 18, cy + 79, name, Color::BLACK.to_u32(), t.button_face.to_u32());
                            btn_x += 50;
                        }

                        gfx.draw_text(cx + 10, cy + 105, "Bottom color:", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        btn_x = cx + 110;
                        for (name, color) in colors_avail.iter() {
                            if btn_x + 50 > cx + win.width - 4 { break; }
                            wm::draw_raised_rect(gfx, btn_x, cy + 101, 45, 18);
                            gfx.fill_rect(btn_x + 2, cy + 103, 14, 14, color.to_u32());
                            gfx.draw_text(btn_x + 18, cy + 104, name, Color::BLACK.to_u32(), t.button_face.to_u32());
                            btn_x += 50;
                        }

                        gfx.draw_text(cx + 10, cy + 130, "Inactive window:", Color::BLACK.to_u32(), t.window_bg.to_u32());

                        let preview_y2 = cy + 150;
                        let top_inactive    = unsafe { GRADIENT_TOP_INACTIVE };
                        let bottom_inactive = unsafe { GRADIENT_BOTTOM_INACTIVE };
                        for row in 0..preview_h {
                            let t_val = row as f32 / (preview_h as f32 - 1.0).max(1.0);
                            let r = (top_inactive.r as f32 + (bottom_inactive.r as f32 - top_inactive.r as f32) * t_val) as u8;
                            let g = (top_inactive.g as f32 + (bottom_inactive.g as f32 - top_inactive.g as f32) * t_val) as u8;
                            let b = (top_inactive.b as f32 + (bottom_inactive.b as f32 - top_inactive.b as f32) * t_val) as u8;
                            let color = Color::rgb(r, g, b).to_u32();
                            for dx in 0..preview_w { gfx.put_pixel(preview_x + dx, preview_y2 + row, color); }
                        }

                        gfx.draw_text(cx + 10, cy + 180, "Top color:", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        btn_x = cx + 100;
                        for (name, color) in colors_avail.iter() {
                            if btn_x + 50 > cx + win.width - 4 { break; }
                            wm::draw_raised_rect(gfx, btn_x, cy + 176, 45, 18);
                            gfx.fill_rect(btn_x + 2, cy + 178, 14, 14, color.to_u32());
                            gfx.draw_text(btn_x + 18, cy + 179, name, Color::BLACK.to_u32(), t.button_face.to_u32());
                            btn_x += 50;
                        }

                        gfx.draw_text(cx + 10, cy + 205, "Bottom color:", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        btn_x = cx + 110;
                        for (name, color) in colors_avail.iter() {
                            if btn_x + 50 > cx + win.width - 4 { break; }
                            wm::draw_raised_rect(gfx, btn_x, cy + 201, 45, 18);
                            gfx.fill_rect(btn_x + 2, cy + 203, 14, 14, color.to_u32());
                            gfx.draw_text(btn_x + 18, cy + 204, name, Color::BLACK.to_u32(), t.button_face.to_u32());
                            btn_x += 50;
                        }
                    }
                    "Manage Documents" => {
                        let mode = unsafe { DOCUMENT_MANAGER_MODE };
                        let mode_text = if mode == 0 { "Save Document" } else { "Load Document" };
                        gfx.draw_text(cx + 10, cy + 10, mode_text, Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 30, "═══════════════════════", Color::BLACK.to_u32(), t.window_bg.to_u32());

                        let count = unsafe { SAVED_DOCUMENTS_COUNT };
                        if count == 0 {
                            gfx.draw_text(cx + 10, cy + 50, "No documents saved yet", Color::rgb(128, 128, 128).to_u32(), t.window_bg.to_u32());
                        } else {
                            for i in 0..count {
                                let y_pos = cy + 50 + i * 30;
                                wm::draw_raised_rect(gfx, cx + 10, y_pos, 250, 24);
                                let label = ["Document 1", "Document 2", "Document 3", "Document 4", "Document 5"];
                                gfx.draw_text(cx + 20, y_pos + 6, label[i], Color::BLACK.to_u32(), t.button_face.to_u32());
                                let doc     = unsafe { &SAVED_DOCUMENTS[i] };
                                let doc_len = unsafe { SAVED_DOCUMENTS_LEN[i] };
                                let preview_len = if doc_len < 20 { doc_len } else { 20 };
                                let preview = core::str::from_utf8(&doc[..preview_len]).unwrap_or("");
                                gfx.draw_text(cx + 120, y_pos + 6, preview, Color::rgb(100, 100, 100).to_u32(), t.button_face.to_u32());
                            }
                        }

                        if mode == 0 && count < 5 {
                            let new_y = cy + 50 + count * 30;
                            wm::draw_raised_rect(gfx, cx + 10, new_y, 250, 24);
                            gfx.draw_text(cx + 20, new_y + 6, "Save as new document...", Color::BLACK.to_u32(), t.button_face.to_u32());
                        }

                        let close_y = cy + 200;
                        wm::draw_raised_rect(gfx, cx + 10, close_y, 100, 24);
                        gfx.draw_text(cx + 20, close_y + 6, "Close", Color::BLACK.to_u32(), t.button_face.to_u32());
                    }
                    "Notepad" => {
                        gfx.fill_rect(cx, cy, win.width - 4, 28, Color::rgb(236, 233, 216).to_u32());
                        gfx.fill_rect(cx, cy + 28, win.width - 4, 1, Color::rgb(172, 168, 153).to_u32());

                        wm::draw_raised_rect(gfx, cx + 10, cy + 4, 60, 20);
                        gfx.draw_text(cx + 20, cy + 7, "Save", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

                        wm::draw_raised_rect(gfx, cx + 80, cy + 4, 60, 20);
                        gfx.draw_text(cx + 90, cy + 7, "Load", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

                        let text_y = cy + 32;
                        gfx.draw_text(cx + 10, text_y + 10, "Notepad",                  Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, text_y + 30, "═══════════════════════", Color::BLACK.to_u32(), t.window_bg.to_u32());

                        let notepad_len  = unsafe { NOTEPAD_LEN };
                        let notepad_text = unsafe { &NOTEPAD_TEXT };
                        let mut line_y = text_y + 50;
                        let mut col = 0usize;
                        for i in 0..notepad_len {
                            if notepad_text[i] == b'\n' || col >= 40 {
                                line_y += 16;
                                col = 0;
                                if notepad_text[i] == b'\n' { continue; }
                            }
                            if line_y < text_y + 300 {
                                let ch = [notepad_text[i]];
                                let s = core::str::from_utf8(&ch).unwrap_or(" ");
                                gfx.draw_text(cx + 10 + col * 8, line_y, s, Color::BLACK.to_u32(), t.window_bg.to_u32());
                            }
                            col += 1;
                        }
                        if notepad_len > 0 && notepad_text[notepad_len - 1] != b'\n' {
                            line_y += 16;
                        }
                        gfx.draw_text(cx + 10, line_y, "|", Color::BLACK.to_u32(), t.window_bg.to_u32());
                    }
                    "File Manager" => {
                        let win_width  = win.width;
                        let win_height = win.height;

                        gfx.fill_rect(cx, cy, win_width - 4, win_height - 30, Color::WHITE.to_u32());
                        gfx.fill_rect(cx, cy, win_width - 4, 30, Color::rgb(236, 233, 216).to_u32());

                        wm::draw_raised_rect(gfx, cx + 10, cy + 4, 80, 22);
                        gfx.draw_text(cx + 18, cy + 8, "About", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

                        gfx.fill_rect(cx, cy + 30, win_width - 4, 1, Color::rgb(172, 168, 153).to_u32());
                        gfx.fill_rect(cx, cy + 31, win_width - 4, 24, Color::rgb(236, 233, 216).to_u32());
                        gfx.draw_text(cx + 10, cy + 35, "C:/MiOS/", Color::BLACK.to_u32(), Color::rgb(236, 233, 216).to_u32());
                        gfx.fill_rect(cx, cy + 55, win_width - 4, 1, Color::rgb(172, 168, 153).to_u32());

                        let content_y      = cy + 60;
                        let content_height = win_height - 90;
                        gfx.fill_rect(cx, content_y, win_width - 4, content_height, Color::WHITE.to_u32());

                        let doc_count = unsafe { SAVED_DOCUMENTS_COUNT };
                        if doc_count == 0 {
                            let empty_text = "This folder is empty";
                            let text_x = cx + (win_width - 4 - empty_text.len() * 8) / 2;
                            let text_y = content_y + content_height / 2 - 8;
                            gfx.draw_text(text_x, text_y, empty_text, Color::rgb(128, 128, 128).to_u32(), Color::WHITE.to_u32());
                        } else {
                            for i in 0..doc_count {
                                let y_pos = content_y + 10 + i * 30;
                                wm::draw_raised_rect(gfx, cx + 10, y_pos, win_width - 24, 24);
                                let label = ["Document 1", "Document 2", "Document 3", "Document 4", "Document 5"];
                                gfx.draw_text(cx + 20, y_pos + 6, label[i], Color::BLACK.to_u32(), t.button_face.to_u32());
                                let doc_len     = unsafe { SAVED_DOCUMENTS_LEN[i] };
                                let preview_len = if doc_len < 30 { doc_len } else { 30 };
                                let doc         = unsafe { &SAVED_DOCUMENTS[i] };
                                let preview     = core::str::from_utf8(&doc[..preview_len]).unwrap_or("");
                                gfx.draw_text(cx + 140, y_pos + 6, preview, Color::rgb(100, 100, 100).to_u32(), t.button_face.to_u32());
                            }
                        }

                        let status_y = cy + win_height - 30;
                        gfx.fill_rect(cx, status_y,     win_width - 4, 1,  Color::rgb(172, 168, 153).to_u32());
                        gfx.fill_rect(cx, status_y + 1, win_width - 4, 20, Color::rgb(236, 233, 216).to_u32());
                        if doc_count == 0 {
                            gfx.draw_text(cx + 4, status_y + 3, "0 objects",   Color::BLACK.to_u32(), Color::rgb(236, 233, 216).to_u32());
                        } else {
                            gfx.draw_text(cx + 4, status_y + 3, "Documents:",  Color::BLACK.to_u32(), Color::rgb(236, 233, 216).to_u32());
                        }
                    }
                    "About Files" => {
                        gfx.draw_text(cx + 10, cy + 10,  "About Files",              Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 30,  "═══════════════════════",  Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 50,  "File Manager Application", Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 70,  "Created by MIkhail",       Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 90,  "Year: 2026",               Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 110, "═══════════════════════",  Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 130, "MiOS v5.2",                Color::BLACK.to_u32(), t.window_bg.to_u32());
                    }
                    _ => {
                        gfx.draw_text(cx + 10, cy + 10, "File Manager",                          Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 30, "File System: FAT32",                    Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 46, "All system files:",                     Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 62, "Kernel.bin , MiDE.bin , kursor.bin",    Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 78, "mouse.rs , boot.cfg , config(GRUB)",    Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 88, "shell.rs , gui , vesa.rs",              Color::BLACK.to_u32(), t.window_bg.to_u32());
                        gfx.draw_text(cx + 10, cy + 97, "boot.cfg , termenal.rs , pakeg.zip",    Color::BLACK.to_u32(), t.window_bg.to_u32());
                    }
                }
            }

            fn redraw_all(gfx: &mut Graphics, wm: &WindowManager, term_input: &[u8], term_len: usize) {
                let use_bmp    = unsafe { USE_BMP_WALLPAPER };
                let current_idx = unsafe { CURRENT_BMP_INDEX };
                
                if use_bmp {
                    let bmp_to_use = match current_idx {
                        0 => unsafe { &BMP_IMAGE },
                        1 => unsafe { &BMP_IMAGE2 },
                        2 => unsafe { &BMP_IMAGE3 },
                        _ => unsafe { &BMP_IMAGE },
                    };
                    
                    if let Some(ref bmp) = bmp_to_use {
                        bmp.draw_as_wallpaper(gfx);
                    } else {
                        let wallpaper = unsafe { WALLPAPER };
                        gfx.clear(wallpaper.to_u32());
                    }
                } else {
                    let wallpaper = unsafe { WALLPAPER };
                    gfx.clear(wallpaper.to_u32());
                }

                let t = wm::get_theme();
                let taskbar_h = 30;
                let taskbar_y = gfx.height - taskbar_h;

                gfx.fill_rect(0, taskbar_y, gfx.width, taskbar_h, t.taskbar_bg.to_u32());
                wm::draw_raised_rect(gfx, 2, taskbar_y + 2, 50, 26);
                gfx.draw_text(6, taskbar_y + 8, "Menu", t.button_text.to_u32(), t.button_face.to_u32());

                let mut btn_x = 58;
                for i in 0..wm.window_count {
                    if let Some(title) = wm.get_window_title(i) {
                        let btn_w = 100;
                        if btn_x + btn_w < gfx.width - 100 {
                            let is_active = wm.active_window == Some(i) &&
                                wm.windows[i].as_ref().map(|w| w.state != WindowState::Minimized).unwrap_or(false);
                            if is_active {
                                wm::draw_sunken_rect(gfx, btn_x, taskbar_y + 3, btn_w, 24);
                            } else {
                                wm::draw_raised_rect(gfx, btn_x, taskbar_y + 3, btn_w, 24);
                            }
                            gfx.draw_text(btn_x + 6, taskbar_y + 9, title, t.button_text.to_u32(), t.button_face.to_u32());
                            btn_x += btn_w + 4;
                        }
                    }
                }

                let icon_size = 64;
                let icon_y    = 50;
                let icon_bg_color = if use_bmp { Color::rgb(0, 0, 0) } else { unsafe { WALLPAPER } };

                let icons_data: [(usize, &str, &Option<BmpImage>); 6] = [
                    (50,  "Terminal", unsafe { &ICON_TERMINAL }),
                    (150, "Files",    unsafe { &ICON_FILES }),
                    (250, "Settings", unsafe { &ICON_SETTINGS }),
                    (350, "AboutOS",  unsafe { &ICON_ABOUT }),
                    (450, "Notepad",  unsafe { &ICON_NOTEPAD }),
                    (550, "Paint",    unsafe { &ICON_PAINT }),
                ];

                for (x, lbl, icon) in icons_data.iter() {
                    wm::draw_raised_rect(gfx, *x, icon_y, icon_size, icon_size);
                    draw_icon(gfx, *x + 4, icon_y + 4, icon, icon_size - 8);
                    gfx.draw_text(*x + (icon_size - lbl.len() * 8) / 2, icon_y + icon_size + 5, lbl, t.desktop_icon_text.to_u32(), icon_bg_color.to_u32());
                }

                wm.start_menu.draw(gfx);

                wm.draw_all_with_content(gfx, &|gfx, win, idx| {
                    draw_window_content(gfx, win, idx);
                });

                if let Some(active) = wm.active_window {
                    if let Some(ref win) = wm.windows[active] {
                        if win.title == "Terminal" && win.state != WindowState::Minimized {
                            let cx = win.x + 2;
                            let cy = win.y + 26;
                            let hist_len = unsafe { TERM_HIST_LEN };
                            let prompt_y = cy + 10 + hist_len * 16;
                            if term_len > 0 {
                                let text = core::str::from_utf8(&term_input[..term_len]).unwrap_or("");
                                gfx.draw_text(cx + 10 + 6*8, prompt_y, text, Color::BLACK.to_u32(), t.window_bg.to_u32());
                            }
                        }
                    }
                }
            }

            fn draw_login_screen(gfx: &mut Graphics) {
                let screen_w = gfx.width;
                let screen_h = gfx.height;

                let use_bmp    = unsafe { USE_BMP_WALLPAPER };
                let current_idx = unsafe { CURRENT_BMP_INDEX };

                if use_bmp {
                    let bmp_to_use = match current_idx {
                        0 => unsafe { &BMP_IMAGE },
                        1 => unsafe { &BMP_IMAGE2 },
                        2 => unsafe { &BMP_IMAGE3 },
                        _ => unsafe { &BMP_IMAGE },
                    };
                    
                    if let Some(ref bmp) = bmp_to_use {
                        bmp.draw_as_wallpaper(gfx);
                    } else {
                        let wallpaper = unsafe { WALLPAPER };
                        gfx.clear(wallpaper.to_u32());
                    }
                } else {
                    let wallpaper = unsafe { WALLPAPER };
                    gfx.clear(wallpaper.to_u32());
                }

                let login_w = 300;
                let login_h = 180;
                let login_x = (screen_w - login_w) / 2;
                let login_y = (screen_h - login_h) / 2;

                gfx.fill_rect(login_x, login_y, login_w, login_h, Color::rgb(235, 235, 235).to_u32());
                gfx.draw_rect_border(login_x, login_y, login_w, login_h, Color::rgb(87, 87, 87).to_u32());
                gfx.draw_rect_border(login_x + 1, login_y + 1, login_w - 2, login_h - 2, Color::WHITE.to_u32());

                let title_bar_h = 30;
                gfx.fill_rect(login_x + 2, login_y + 2, login_w - 4, title_bar_h, Color::rgb(90, 90, 90).to_u32());
                gfx.draw_text(login_x + 10, login_y + 6, "User Login", Color::WHITE.to_u32(), Color::rgb(90, 90, 90).to_u32());

                let welcome_y = login_y + title_bar_h + 10;
                gfx.draw_text(login_x + 20, welcome_y, "Welcome to MiOS", Color::BLACK.to_u32(), Color::rgb(235, 235, 235).to_u32());

                let user_y = welcome_y + 25;
                gfx.draw_text(login_x + 20, user_y, "User:", Color::BLACK.to_u32(), Color::rgb(235, 235, 235).to_u32());
                gfx.draw_text(login_x + 80, user_y, "User", Color::rgb(0, 0, 180).to_u32(), Color::rgb(235, 235, 235).to_u32());

                let btn_w = 80;
                let btn_h = 30;
                let btn_x = login_x + (login_w - btn_w) / 2;
                let btn_y = user_y + 40;

                wm::draw_raised_rect(gfx, btn_x, btn_y, btn_w, btn_h);
                let text_w = "Login".len() * 8;
                let text_x = btn_x + (btn_w - text_w) / 2;
                gfx.draw_text(text_x, btn_y + 8, "Login", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());
            }

            draw_login_screen(&mut gfx);
            gfx.flush();

            unsafe { idt::init_idt(); }
            mouse::init_ps2_mouse();
            unsafe { mouse::init_mouse_interrupts(); }

            let mut cursor_x = 400usize;
            let mut cursor_y = 300usize;
            let cursor_size = 23;
            let mut cursor_backup = [[0u32; 23]; 23];
            let mut left_prev = false;

            let cursor_shape: [[u32; 23]; 23] = [
                [1,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [1,2,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [1,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [1,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [1,2,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [1,2,2,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [1,2,2,2,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [1,2,2,2,2,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [1,2,2,2,2,2,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [1,2,2,2,2,2,2,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0],
                [1,2,2,2,2,2,2,2,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0],
                [1,2,2,2,2,2,2,2,2,2,2,2,1,0,0,0,0,0,0,0,0,0,0],
                [1,2,2,2,2,2,2,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0],
                [1,2,2,2,1,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [1,2,2,1,0,1,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [1,2,1,0,0,1,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0,0],
                [1,1,0,0,0,0,1,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0],
                [0,0,0,0,0,0,1,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0,0],
                [0,0,0,0,0,0,0,1,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0],
                [0,0,0,0,0,0,0,1,2,2,2,1,0,0,0,0,0,0,0,0,0,0,0],
                [0,0,0,0,0,0,0,0,1,2,2,2,1,0,0,0,0,0,0,0,0,0,0],
                [0,0,0,0,0,0,0,0,1,2,2,2,1,0,0,0,0,0,0,0,0,0,0],
                [0,0,0,0,0,0,0,0,0,1,1,1,1,0,0,0,0,0,0,0,0,0,0],
            ];

            loop {
                let mut need_flush = false;

                if let Some(sc) = mouse::get_key() {
                    unsafe {
                        if LOGGED_IN {
                            if sc & 0x80 == 0 {
                                if let Some(active) = wm.active_window {
                                    if let Some(ref win) = wm.windows[active] {
                                        let is_term    = win.title == "Terminal" && win.state != WindowState::Minimized;
                                        let is_notepad = win.title == "Notepad"  && win.state != WindowState::Minimized;

                                        if is_term {
                                            let c = match sc {
                                                0x1E => Some('a'), 0x30 => Some('b'), 0x2E => Some('c'),
                                                0x20 => Some('d'), 0x12 => Some('e'), 0x21 => Some('f'),
                                                0x22 => Some('g'), 0x23 => Some('h'), 0x17 => Some('i'),
                                                0x24 => Some('j'), 0x25 => Some('k'), 0x26 => Some('l'),
                                                0x32 => Some('m'), 0x31 => Some('n'), 0x18 => Some('o'),
                                                0x19 => Some('p'), 0x10 => Some('q'), 0x13 => Some('r'),
                                                0x1F => Some('s'), 0x14 => Some('t'), 0x16 => Some('u'),
                                                0x2F => Some('v'), 0x11 => Some('w'), 0x2D => Some('x'),
                                                0x15 => Some('y'), 0x2C => Some('z'),
                                                0x39 => Some(' '),
                                                0x1C => Some('\n'),
                                                0x0E => Some('\x08'),
                                                _ => None,
                                            };
                                            if let Some(ch) = c {
                                                if ch == '\n' {
                                                    let cmd = core::str::from_utf8(&term_input[..term_len]).unwrap_or("");
                                                    let out = exec_command(cmd);
                                                    if out == "\x04" {
                                                        clear_history();
                                                        push_history("MiOS> cls");
                                                    } else {
                                                        push_history("MiOS> ");
                                                        if !cmd.is_empty() {
                                                            push_history(cmd);
                                                            if !out.is_empty() { push_history(out); }
                                                        }
                                                    }
                                                    term_len = 0;
                                                } else if ch == '\x08' && term_len > 0 {
                                                    term_len -= 1;
                                                } else if (ch as u8) >= 0x20 && term_len < 255 {
                                                    term_input[term_len] = ch as u8;
                                                    term_len += 1;
                                                }
                                                redraw_all(&mut gfx, &wm, &term_input, term_len);
                                                need_flush = true;
                                            }
                                        } else if is_notepad {
                                            let c = match sc {
                                                0x1E => Some('a'), 0x30 => Some('b'), 0x2E => Some('c'),
                                                0x20 => Some('d'), 0x12 => Some('e'), 0x21 => Some('f'),
                                                0x22 => Some('g'), 0x23 => Some('h'), 0x17 => Some('i'),
                                                0x24 => Some('j'), 0x25 => Some('k'), 0x26 => Some('l'),
                                                0x32 => Some('m'), 0x31 => Some('n'), 0x18 => Some('o'),
                                                0x19 => Some('p'), 0x10 => Some('q'), 0x13 => Some('r'),
                                                0x1F => Some('s'), 0x14 => Some('t'), 0x16 => Some('u'),
                                                0x2F => Some('v'), 0x11 => Some('w'), 0x2D => Some('x'),
                                                0x15 => Some('y'), 0x2C => Some('z'),
                                                0x39 => Some(' '),
                                                0x1C => Some('\n'),
                                                0x0E => Some('\x08'),
                                                _ => None,
                                            };
                                            if let Some(ch) = c {
                                                if ch == '\x08' && NOTEPAD_LEN > 0 {
                                                    NOTEPAD_LEN -= 1;
                                                } else if ch == '\n' && NOTEPAD_LEN < 1023 {
                                                    NOTEPAD_TEXT[NOTEPAD_LEN] = b'\n';
                                                    NOTEPAD_LEN += 1;
                                                } else if (ch as u8) >= 0x20 && NOTEPAD_LEN < 1023 {
                                                    NOTEPAD_TEXT[NOTEPAD_LEN] = ch as u8;
                                                    NOTEPAD_LEN += 1;
                                                }
                                                redraw_all(&mut gfx, &wm, &term_input, term_len);
                                                need_flush = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(packet) = mouse::get_mouse_packet() {
                    let old_x = cursor_x;
                    let old_y = cursor_y;
                    cursor_x = (cursor_x as i32 + packet.dx).max(0).min((gfx.width  - cursor_size) as i32) as usize;
                    cursor_y = (cursor_y as i32 + packet.dy).max(0).min((gfx.height - cursor_size) as i32) as usize;

                    for dy in 0..cursor_size {
                        for dx in 0..cursor_size {
                            if old_x + dx < gfx.width && old_y + dy < gfx.height {
                                gfx.put_pixel(old_x + dx, old_y + dy, cursor_backup[dy][dx]);
                            }
                        }
                    }

                    let mut redraw = false;
                    let mut paint_update = false;

                    unsafe {
                        if !LOGGED_IN {
                            if packet.left && !left_prev {
                                let screen_w = gfx.width;
                                let screen_h = gfx.height;
                                let login_w  = 300;
                                let login_h  = 180;
                                let login_x  = (screen_w - login_w) / 2;
                                let login_y  = (screen_h - login_h) / 2;
                                let btn_w    = 80;
                                let btn_h    = 30;
                                let btn_x    = login_x + (login_w - btn_w) / 2;
                                let btn_y    = login_y + 95;

                                if cursor_x >= btn_x && cursor_x <= btn_x + btn_w &&
                                   cursor_y >= btn_y && cursor_y <= btn_y + btn_h {
                                    LOGGED_IN = true;
                                    redraw_all(&mut gfx, &wm, &term_input, term_len);
                                    redraw = true;
                                }
                            }
                        } else {
                            if let Some(active) = wm.active_window {
                                if let Some(ref win) = wm.windows[active] {
                                    if win.title == "Paint" && win.state != WindowState::Minimized {
                                        let wx = win.x + 2;
                                        let wy = win.y + 26;
                                        let canvas_x = wx + 5;
                                        let canvas_y = wy + 45;
                                        let canvas_w = (win.width - 10).min(400);
                                        let canvas_h = (win.height - 55).min(300);
                                        
                                        if cursor_x >= canvas_x && cursor_x < canvas_x + canvas_w &&
                                           cursor_y >= canvas_y && cursor_y < canvas_y + canvas_h {
                                            
                                            let canvas_cx = cursor_x - canvas_x;
                                            let canvas_cy = cursor_y - canvas_y;
                                            
                                            if packet.left {
                                                let size = PAINT_PEN_SIZE;
                                                let color = PAINT_COLOR;
                                                let tool = PAINT_TOOL;
                                                
                                                match tool {
                                                    0 => {
                                                        for dy in 0..size {
                                                            for dx in 0..size {
                                                                let x = canvas_cx + dx - size/2;
                                                                let y = canvas_cy + dy - size/2;
                                                                if x < canvas_w && y < canvas_h && x < 400 && y < 300 {
                                                                    PAINT_CANVAS[y][x] = color;
                                                                }
                                                            }
                                                        }
                                                        paint_update = true;
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                        
                                        if packet.left && !left_prev {
                                            if cursor_x >= wx + 5 && cursor_x <= wx + 35 &&
                                               cursor_y >= wy + 5 && cursor_y <= wy + 35 {
                                                PAINT_TOOL = 0;
                                                redraw = true;
                                            }
                                            if cursor_x >= wx + 40 && cursor_x <= wx + 70 &&
                                               cursor_y >= wy + 5 && cursor_y <= wy + 35 {
                                                PAINT_TOOL = 1;
                                                redraw = true;
                                            }
                                            if cursor_x >= wx + 75 && cursor_x <= wx + 105 &&
                                               cursor_y >= wy + 5 && cursor_y <= wy + 35 {
                                                PAINT_TOOL = 2;
                                                redraw = true;
                                            }
                                            for i in 0..8 {
                                                let color_x = wx + 120 + i * 25;
                                                if cursor_x >= color_x && cursor_x <= color_x + 20 &&
                                                   cursor_y >= wy + 5 && cursor_y <= wy + 35 {
                                                    let colors = [0x000000, 0xFF0000, 0x00FF00, 0x0000FF,
                                                                  0xFFFF00, 0xFF00FF, 0x00FFFF, 0xFFFFFF];
                                                    PAINT_COLOR = colors[i];
                                                    redraw = true;
                                                }
                                            }
                                            for i in 0..3 {
                                                let size_x = wx + 330 + i * 30;
                                                if cursor_x >= size_x && cursor_x <= size_x + 25 &&
                                                   cursor_y >= wy + 5 && cursor_y <= wy + 35 {
                                                    PAINT_PEN_SIZE = i * 2 + 1;
                                                    redraw = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            if paint_update {
                                redraw = true;
                            }
                            
                            if packet.left && !left_prev {
                                let taskbar_y = gfx.height - 30;
                                if cursor_y >= taskbar_y + 3 && cursor_y < taskbar_y + 27 {
                                    let mut bx = 58;
                                    for i in 0..wm.window_count {
                                        if let Some(_) = wm.get_window_title(i) {
                                            let bw = 100;
                                            if cursor_x >= bx && cursor_x < bx + bw {
                                                if let Some(ref win) = wm.windows[i] {
                                                    if win.state == WindowState::Minimized {
                                                        wm.restore_window(i);
                                                    } else {
                                                        wm.active_window = Some(i);
                                                    }
                                                    redraw = true;
                                                }
                                                break;
                                            }
                                            bx += bw + 4;
                                        }
                                    }
                                }

                                if !redraw {
                                    if wm.start_menu.open {
                                        if let Some(action) = wm.start_menu.handle_click(&gfx, cursor_x, cursor_y) {
                                            match action {
                                                "Terminal" => { wm.create_window("Terminal", 100, 100, 500, 350); term_len = 0; clear_history(); }
                                                "Files"    => { wm.create_window("File Manager", 150, 120, 500, 350); }
                                                "Settings" => { wm.create_window("Settings", 200, 180, 450, 370); }
                                                "Notepad"  => { wm.create_window("Notepad", 100, 100, 500, 400); }
                                                "Paint"    => { wm.create_window("Paint", 100, 100, 420, 400); }
                                                "Reboot"   => {
                                                    let screen_w = gfx.width;
                                                    let screen_h = gfx.height;
                                                    gfx.clear(Color::BLACK.to_u32());
                                                    let title = "MiOS";
                                                    let title_width = title.len() * tamzen_font::FONT_WIDTH;
                                                    let title_x = (screen_w - title_width) / 2;
                                                    let title_y = screen_h / 2 - 40;
                                                    gfx.draw_text(title_x, title_y, title, Color::WHITE.to_u32(), Color::BLACK.to_u32());
                                                    let bar_width  = 300;
                                                    let bar_height = 24;
                                                    let bar_x = (screen_w - bar_width) / 2;
                                                    let bar_y = screen_h / 2 + 10;
                                                    gfx.draw_rect_border(bar_x - 1, bar_y - 1, bar_width + 2, bar_height + 2, Color::WHITE.to_u32());
                                                    gfx.flush();
                                                    let total_ticks = 400;
                                                    for tick in 0..=total_ticks {
                                                        let progress = (tick as usize * bar_width) / total_ticks as usize;
                                                        if progress > 0 {
                                                            gfx.fill_rect(bar_x, bar_y, progress, bar_height, Color::WHITE.to_u32());
                                                            if progress < bar_width {
                                                                gfx.fill_rect(bar_x + progress, bar_y, bar_width - progress, bar_height, Color::BLACK.to_u32());
                                                            }
                                                        }
                                                        gfx.flush();
                                                        for _ in 0..100000 { unsafe { core::arch::asm!("nop") } }
                                                    }
                                                    for _ in 0..50000 { unsafe { core::arch::asm!("nop") } }

                                                    WALLPAPER = Color::rgb(89, 0, 255);
                                                    THEME_IDX = 0;
                                                    TERM_HIST_LEN = 0;
                                                    NOTEPAD_LEN = 0;
                                                    WALLPAPER_SELECTOR_OPEN = false;
                                                    THEME_SELECTOR_OPEN = false;
                                                    DOCUMENT_MANAGER_OPEN = false;
                                                    DOCUMENT_MANAGER_MODE = 0;
                                                    SAVED_DOCUMENTS_COUNT = 0;
                                                    for i in 0..5 { SAVED_DOCUMENTS_LEN[i] = 0; }
                                                    GRADIENT_TOP_ACTIVE      = Color::rgb(90, 90, 90);
                                                    GRADIENT_BOTTOM_ACTIVE   = Color::rgb(20, 20, 20);
                                                    GRADIENT_TOP_INACTIVE    = Color::rgb(105, 105, 105);
                                                    GRADIENT_BOTTOM_INACTIVE = Color::rgb(50, 50, 50);
                                                    USE_BMP_WALLPAPER = true;
                                                    CURRENT_BMP_INDEX = 0;
                                                    LOGGED_IN   = false;
                                                }
                                                "Shutdown" => {
                                                    gfx.clear(Color::BLACK.to_u32());
                                                    gfx.draw_text(
                                                        (gfx.width - 16 * 17) / 2,
                                                        gfx.height / 2 - 8,
                                                        "Now you can turn off the power",
                                                        Color::WHITE.to_u32(),
                                                        Color::BLACK.to_u32()
                                                    );
                                                    gfx.flush();
                                                    loop { core::arch::asm!("hlt") }
                                                }
                                                _ => {}
                                            }
                                            wm.start_menu.open = false;
                                            redraw = true;
                                        } else if !wm.start_menu.contains(cursor_x, cursor_y, &gfx) {
                                            wm.start_menu.open = false;
                                            redraw = true;
                                        }
                                    } else if cursor_y >= taskbar_y && cursor_x < 52 {
                                        wm.start_menu.open = true;
                                        redraw = true;
                                    } else if wm.handle_mouse_press(&mut gfx, cursor_x, cursor_y) {
                                        redraw = true;
                                    } else {
                                        let mut bmp_selected = false;
                                        for i in 0..wm.window_count {
                                            if let Some(ref win) = wm.windows[i] {
                                                if win.title == "BMP Wallpaper Selector" && win.state != WindowState::Minimized {
                                                    let wx = win.x + 2;
                                                    let wy = win.y + 26;
                                                    
                                                    for j in 0..3 {
                                                        let y_pos = wy + 30 + j * 60;
                                                        if cursor_x >= wx + 10 && cursor_x <= wx + 260 &&
                                                           cursor_y >= y_pos && cursor_y <= y_pos + 50 {
                                                            CURRENT_BMP_INDEX = j;
                                                            USE_BMP_WALLPAPER = true;
                                                            bmp_selected = true;
                                                            for k in i..wm.window_count - 1 {
                                                                wm.windows[k] = wm.windows[k + 1].take();
                                                            }
                                                            wm.windows[wm.window_count - 1] = None;
                                                            wm.window_count -= 1;
                                                            if wm.active_window == Some(i) || wm.active_window.is_none() {
                                                                wm.active_window = if wm.window_count > 0 { Some(wm.window_count - 1) } else { None };
                                                            }
                                                            break;
                                                        }
                                                    }
                                                    if bmp_selected { break; }
                                                }
                                            }
                                        }
                                        
                                        if bmp_selected {
                                            redraw = true;
                                            continue;
                                        }

                                        let mut doc_action_done = false;
                                        for i in 0..wm.window_count {
                                            if let Some(ref win) = wm.windows[i] {
                                                if win.title == "Manage Documents" && win.state != WindowState::Minimized {
                                                    let wx = win.x + 2;
                                                    let wy = win.y + 26;

                                                    let close_y = wy + 200;
                                                    if cursor_x >= wx + 10 && cursor_x <= wx + 110 &&
                                                       cursor_y >= close_y && cursor_y <= close_y + 24 {
                                                        for k in i..wm.window_count - 1 { wm.windows[k] = wm.windows[k + 1].take(); }
                                                        wm.windows[wm.window_count - 1] = None;
                                                        wm.window_count -= 1;
                                                        if wm.active_window == Some(i) || wm.active_window.is_none() {
                                                            wm.active_window = if wm.window_count > 0 { Some(wm.window_count - 1) } else { None };
                                                        }
                                                        DOCUMENT_MANAGER_OPEN = false;
                                                        doc_action_done = true;
                                                        redraw = true;
                                                        break;
                                                    }

                                                    let doc_count = SAVED_DOCUMENTS_COUNT;
                                                    let mode      = DOCUMENT_MANAGER_MODE;

                                                    for j in 0..doc_count {
                                                        let y_pos = wy + 50 + j * 30;
                                                        if cursor_x >= wx + 10 && cursor_x <= wx + 260 &&
                                                           cursor_y >= y_pos && cursor_y <= y_pos + 24 {
                                                            if mode == 0 {
                                                                let text_len = NOTEPAD_LEN;
                                                                let copy_len = if text_len > 512 { 512 } else { text_len };
                                                                SAVED_DOCUMENTS[j][..copy_len].copy_from_slice(&NOTEPAD_TEXT[..copy_len]);
                                                                SAVED_DOCUMENTS_LEN[j] = copy_len;
                                                            } else {
                                                                let doc_len  = SAVED_DOCUMENTS_LEN[j];
                                                                let copy_len = if doc_len > 1024 { 1024 } else { doc_len };
                                                                NOTEPAD_TEXT[..copy_len].copy_from_slice(&SAVED_DOCUMENTS[j][..copy_len]);
                                                                NOTEPAD_LEN = copy_len;
                                                            }
                                                            for k in i..wm.window_count - 1 { wm.windows[k] = wm.windows[k + 1].take(); }
                                                            wm.windows[wm.window_count - 1] = None;
                                                            wm.window_count -= 1;
                                                            if wm.active_window == Some(i) || wm.active_window.is_none() {
                                                                wm.active_window = if wm.window_count > 0 { Some(wm.window_count - 1) } else { None };
                                                            }
                                                            DOCUMENT_MANAGER_OPEN = false;
                                                            doc_action_done = true;
                                                            redraw = true;
                                                            break;
                                                        }
                                                    }

                                                    if !doc_action_done && mode == 0 && doc_count < 5 {
                                                        let new_y = wy + 50 + doc_count * 30;
                                                        if cursor_x >= wx + 10 && cursor_x <= wx + 260 &&
                                                           cursor_y >= new_y && cursor_y <= new_y + 24 {
                                                            let text_len = NOTEPAD_LEN;
                                                            let copy_len = if text_len > 512 { 512 } else { text_len };
                                                            let idx = SAVED_DOCUMENTS_COUNT;
                                                            SAVED_DOCUMENTS[idx][..copy_len].copy_from_slice(&NOTEPAD_TEXT[..copy_len]);
                                                            SAVED_DOCUMENTS_LEN[idx] = copy_len;
                                                            SAVED_DOCUMENTS_COUNT += 1;
                                                            for k in i..wm.window_count - 1 { wm.windows[k] = wm.windows[k + 1].take(); }
                                                            wm.windows[wm.window_count - 1] = None;
                                                            wm.window_count -= 1;
                                                            if wm.active_window == Some(i) || wm.active_window.is_none() {
                                                                wm.active_window = if wm.window_count > 0 { Some(wm.window_count - 1) } else { None };
                                                            }
                                                            DOCUMENT_MANAGER_OPEN = false;
                                                            doc_action_done = true;
                                                            redraw = true;
                                                        }
                                                    }
                                                    break;
                                                }
                                            }
                                        }

                                        if !doc_action_done {
                                            let mut about_files_opened = false;
                                            for i in 0..wm.window_count {
                                                if let Some(ref win) = wm.windows[i] {
                                                    if win.title == "File Manager" && win.state != WindowState::Minimized {
                                                        let wx = win.x + 2;
                                                        let wy = win.y + 26;
                                                        if cursor_x >= wx + 10 && cursor_x <= wx + 90 &&
                                                           cursor_y >= wy + 4  && cursor_y <= wy + 26 {
                                                            wm.create_window("About Files", 250, 200, 300, 180);
                                                            about_files_opened = true;
                                                            redraw = true;
                                                        }
                                                        break;
                                                    }
                                                }
                                            }

                                            if !about_files_opened {
                                                let mut file_opened = false;
                                                for i in 0..wm.window_count {
                                                    if let Some(ref win) = wm.windows[i] {
                                                        if win.title == "File Manager" && win.state != WindowState::Minimized {
                                                            let wx        = win.x + 2;
                                                            let wy        = win.y + 26;
                                                            let doc_count = SAVED_DOCUMENTS_COUNT;
                                                            for j in 0..doc_count {
                                                                let y_pos = wy + 70 + j * 30;
                                                                if cursor_x >= wx + 10 && cursor_x <= wx + win.width - 26 &&
                                                                   cursor_y >= y_pos && cursor_y <= y_pos + 24 {
                                                                    let doc_len  = SAVED_DOCUMENTS_LEN[j];
                                                                    let copy_len = if doc_len > 1024 { 1024 } else { doc_len };
                                                                    NOTEPAD_TEXT[..copy_len].copy_from_slice(&SAVED_DOCUMENTS[j][..copy_len]);
                                                                    NOTEPAD_LEN = copy_len;
                                                                    wm.create_window("Notepad", 100, 100, 500, 400);
                                                                    file_opened = true;
                                                                    redraw = true;
                                                                    break;
                                                                }
                                                            }
                                                            break;
                                                        }
                                                    }
                                                }

                                                if !file_opened {
                                                    let mut wallpaper_changed = false;
                                                    for i in 0..wm.window_count {
                                                        if let Some(ref win) = wm.windows[i] {
                                                            if win.title == "Wallpaper Selector" && win.state != WindowState::Minimized {
                                                                let wx = win.x + 2;
                                                                let wy = win.y + 26;
                                                                let colors = [
                                                                    Color::rgb(89, 0, 255),
                                                                    Color::rgb(0, 23, 128),
                                                                    Color::rgb(192, 0, 0),
                                                                    Color::rgb(192, 192, 0),
                                                                    Color::rgb(0, 0, 128),
                                                                    Color::rgb(128, 0, 128),
                                                                ];
                                                                for (j, _) in colors.iter().enumerate() {
                                                                    let y_pos = wy + 30 + j * 30;
                                                                    if cursor_x >= wx + 10 && cursor_x <= wx + 260 &&
                                                                       cursor_y >= y_pos && cursor_y <= y_pos + 24 {
                                                                        WALLPAPER = colors[j];
                                                                        USE_BMP_WALLPAPER = false;
                                                                        wallpaper_changed = true;
                                                                        for k in i..wm.window_count - 1 { wm.windows[k] = wm.windows[k + 1].take(); }
                                                                        wm.windows[wm.window_count - 1] = None;
                                                                        wm.window_count -= 1;
                                                                        if wm.active_window == Some(i) || wm.active_window.is_none() {
                                                                            wm.active_window = if wm.window_count > 0 { Some(wm.window_count - 1) } else { None };
                                                                        }
                                                                        break;
                                                                    }
                                                                }
                                                                if wallpaper_changed { break; }
                                                            }
                                                        }
                                                    }

                                                    if !wallpaper_changed {
                                                        let mut theme_changed = false;
                                                        for i in 0..wm.window_count {
                                                            if let Some(ref win) = wm.windows[i] {
                                                                if win.title == "Theme Selector" && win.state != WindowState::Minimized {
                                                                    let wx = win.x + 2;
                                                                    let wy = win.y + 26;
                                                                    for j in 0..3 {
                                                                        let y_pos = wy + 30 + j * 30;
                                                                        if cursor_x >= wx + 10 && cursor_x <= wx + 260 &&
                                                                           cursor_y >= y_pos && cursor_y <= y_pos + 24 {
                                                                            THEME_IDX = j;
                                                                            wm::set_theme(j);
                                                                            theme_changed = true;
                                                                            for k in i..wm.window_count - 1 { wm.windows[k] = wm.windows[k + 1].take(); }
                                                                            wm.windows[wm.window_count - 1] = None;
                                                                            wm.window_count -= 1;
                                                                            if wm.active_window == Some(i) || wm.active_window.is_none() {
                                                                                wm.active_window = if wm.window_count > 0 { Some(wm.window_count - 1) } else { None };
                                                                            }
                                                                            break;
                                                                        }
                                                                    }
                                                                    if theme_changed { break; }
                                                                }
                                                            }
                                                        }

                                                        if !theme_changed {
                                                            let mut gradient_changed = false;
                                                            for i in 0..wm.window_count {
                                                                if let Some(ref win) = wm.windows[i] {
                                                                    if win.title == "Window Gradient" && win.state != WindowState::Minimized {
                                                                        let wx = win.x + 2;
                                                                        let wy = win.y + 26;
                                                                        let colors_avail = [
                                                                            ("Red",    Color::rgb(255, 0, 0)),
                                                                            ("Green",  Color::rgb(0, 255, 0)),
                                                                            ("Blue",   Color::rgb(0, 0, 255)),
                                                                            ("White",  Color::WHITE),
                                                                            ("Black",  Color::BLACK),
                                                                            ("Gray",   Color::rgb(128, 128, 128)),
                                                                            ("Orange", Color::rgb(255, 165, 0)),
                                                                            ("Purple", Color::rgb(128, 0, 128)),
                                                                        ];

                                                                        let mut btn_x = wx + 100;
                                                                        for (_, color) in colors_avail.iter() {
                                                                            if btn_x + 50 > wx + win.width - 4 { break; }
                                                                            if cursor_x >= btn_x && cursor_x <= btn_x + 45 &&
                                                                               cursor_y >= wy + 76 && cursor_y <= wy + 94 {
                                                                                GRADIENT_TOP_ACTIVE = *color;
                                                                                gradient_changed = true;
                                                                                break;
                                                                            }
                                                                            btn_x += 50;
                                                                        }
                                                                        if !gradient_changed {
                                                                            btn_x = wx + 110;
                                                                            for (_, color) in colors_avail.iter() {
                                                                                if btn_x + 50 > wx + win.width - 4 { break; }
                                                                                if cursor_x >= btn_x && cursor_x <= btn_x + 45 &&
                                                                                   cursor_y >= wy + 101 && cursor_y <= wy + 119 {
                                                                                    GRADIENT_BOTTOM_ACTIVE = *color;
                                                                                    gradient_changed = true;
                                                                                    break;
                                                                                }
                                                                                btn_x += 50;
                                                                            }
                                                                        }
                                                                        if !gradient_changed {
                                                                            btn_x = wx + 100;
                                                                            for (_, color) in colors_avail.iter() {
                                                                                if btn_x + 50 > wx + win.width - 4 { break; }
                                                                                if cursor_x >= btn_x && cursor_x <= btn_x + 45 &&
                                                                                   cursor_y >= wy + 176 && cursor_y <= wy + 194 {
                                                                                    GRADIENT_TOP_INACTIVE = *color;
                                                                                    gradient_changed = true;
                                                                                    break;
                                                                                }
                                                                                btn_x += 50;
                                                                            }
                                                                        }
                                                                        if !gradient_changed {
                                                                            btn_x = wx + 110;
                                                                            for (_, color) in colors_avail.iter() {
                                                                                if btn_x + 50 > wx + win.width - 4 { break; }
                                                                                if cursor_x >= btn_x && cursor_x <= btn_x + 45 &&
                                                                                   cursor_y >= wy + 201 && cursor_y <= wy + 219 {
                                                                                    GRADIENT_BOTTOM_INACTIVE = *color;
                                                                                    gradient_changed = true;
                                                                                    break;
                                                                                }
                                                                                btn_x += 50;
                                                                            }
                                                                        }
                                                                        if gradient_changed { redraw = true; }
                                                                        break;
                                                                    }
                                                                }
                                                            }

                                                            if !gradient_changed {
                                                                if let Some(active) = wm.active_window {
                                                                    if let Some(ref win) = wm.windows[active] {
                                                                        if win.title == "Settings" && win.state != WindowState::Minimized {
                                                                            let wx = win.x + 2;
                                                                            let wy = win.y + 26;
                                                                            let btn_wallpaper_y = wy + 30;
                                                                            if cursor_x >= wx + 10 && cursor_x <= wx + 190 &&
                                                                               cursor_y >= btn_wallpaper_y && cursor_y <= btn_wallpaper_y + 24 {
                                                                                wm.create_window("Wallpaper Selector", 250, 200, 300, 250);
                                                                                redraw = true;
                                                                            }
                                                                            let btn_theme_y = wy + 60;
                                                                            if cursor_x >= wx + 10 && cursor_x <= wx + 190 &&
                                                                               cursor_y >= btn_theme_y && cursor_y <= btn_theme_y + 24 {
                                                                                wm.create_window("Theme Selector", 250, 200, 300, 180);
                                                                                redraw = true;
                                                                            }
                                                                            let btn_gradient_y = wy + 90;
                                                                            if cursor_x >= wx + 10 && cursor_x <= wx + 190 &&
                                                                               cursor_y >= btn_gradient_y && cursor_y <= btn_gradient_y + 24 {
                                                                                wm.create_window("Window Gradient", 200, 180, 500, 350);
                                                                                redraw = true;
                                                                            }
                                                                            let btn_bmp_y = wy + 120;
                                                                            if cursor_x >= wx + 10 && cursor_x <= wx + 190 &&
                                                                               cursor_y >= btn_bmp_y && cursor_y <= btn_bmp_y + 24 {
                                                                                USE_BMP_WALLPAPER = !USE_BMP_WALLPAPER;
                                                                                redraw = true;
                                                                            }
                                                                            let btn_wallpaper_select_y = wy + 150;
                                                                            if cursor_x >= wx + 10 && cursor_x <= wx + 190 &&
                                                                               cursor_y >= btn_wallpaper_select_y && cursor_y <= btn_wallpaper_select_y + 24 {
                                                                                wm.create_window("BMP Wallpaper Selector", 200, 150, 300, 270);
                                                                                redraw = true;
                                                                            }
                                                                        } else if win.title == "Notepad" && win.state != WindowState::Minimized {
                                                                            let wx = win.x + 2;
                                                                            let wy = win.y + 26;
                                                                            if cursor_x >= wx + 10 && cursor_x <= wx + 70 &&
                                                                               cursor_y >= wy + 4  && cursor_y <= wy + 24 {
                                                                                DOCUMENT_MANAGER_MODE = 0;
                                                                                DOCUMENT_MANAGER_OPEN = true;
                                                                                wm.create_window("Manage Documents", 200, 150, 300, 280);
                                                                                redraw = true;
                                                                            }
                                                                            if cursor_x >= wx + 80 && cursor_x <= wx + 140 &&
                                                                               cursor_y >= wy + 4  && cursor_y <= wy + 24 {
                                                                                DOCUMENT_MANAGER_MODE = 1;
                                                                                DOCUMENT_MANAGER_OPEN = true;
                                                                                wm.create_window("Manage Documents", 200, 150, 300, 280);
                                                                                redraw = true;
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }

                                                            if !redraw && !wallpaper_changed && !theme_changed && !gradient_changed {
                                                                let icon_y    = 50;
                                                                let icon_size = 64;
                                                                if cursor_y >= icon_y && cursor_y <= icon_y + icon_size {
                                                                    if cursor_x >= 50 && cursor_x <= 50 + icon_size {
                                                                        wm.create_window("Terminal", 100, 100, 500, 350);
                                                                        term_len = 0; clear_history();
                                                                        redraw = true;
                                                                    } else if cursor_x >= 150 && cursor_x <= 150 + icon_size {
                                                                        wm.create_window("File Manager", 150, 120, 500, 350);
                                                                        redraw = true;
                                                                    } else if cursor_x >= 250 && cursor_x <= 250 + icon_size {
                                                                        wm.create_window("Settings", 200, 180, 450, 370);
                                                                        redraw = true;
                                                                    } else if cursor_x >= 350 && cursor_x <= 350 + icon_size {
                                                                        wm.create_window("AboutOS", 200, 100, 400, 350);
                                                                        redraw = true;
                                                                    } else if cursor_x >= 450 && cursor_x <= 450 + icon_size {
                                                                        wm.create_window("Notepad", 100, 100, 500, 400);
                                                                        redraw = true;
                                                                    } else if cursor_x >= 550 && cursor_x <= 550 + icon_size {
                                                                        wm.create_window("Paint", 100, 100, 420, 400);
                                                                        redraw = true;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if !packet.left && left_prev {
                                wm.handle_mouse_release(&mut gfx);
                                redraw = true;
                            } else if packet.left && (packet.dx != 0 || packet.dy != 0) {
                                wm.handle_mouse_move(&mut gfx, cursor_x, cursor_y);
                            }
                        }
                    }

                    left_prev = packet.left;

                    if redraw {
                        unsafe {
                            if LOGGED_IN {
                                redraw_all(&mut gfx, &wm, &term_input, term_len);
                            } else {
                                draw_login_screen(&mut gfx);
                            }
                        }
                    }

                    for dy in 0..cursor_size {
                        for dx in 0..cursor_size {
                            if cursor_x + dx < gfx.width && cursor_y + dy < gfx.height {
                                let offset = (cursor_y + dy) * gfx.width + (cursor_x + dx);
                                cursor_backup[dy][dx] = unsafe { graphics::BACKBUFFER[offset] };
                            }
                        }
                    }

                    for dy in 0..cursor_size {
                        for dx in 0..cursor_size {
                            let pixel = cursor_shape[dy][dx];
                            let color = match pixel {
                                1 => Color::BLACK.to_u32(),
                                2 => Color::WHITE.to_u32(),
                                _ => continue,
                            };
                            gfx.put_pixel(cursor_x + dx, cursor_y + dy, color);
                        }
                    }

                    need_flush = true;
                }

                if need_flush {
                    gfx.flush();
                }

                for _ in 0..5000 { unsafe { core::arch::asm!("nop") } }
            }
        }
    }

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}