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
mod graphics;
mod gui;
mod wm;
mod tamzen_font;
mod idt;
mod mouse;
mod bmp;

use core::panic::PanicInfo;
use core::str;
use multiboot::MultibootInfo;
use graphics::{Graphics, Color};
use wm::{WindowManager, WindowState};
use bmp::BmpImage;
use ata::AtaDrive;

pub static mut VESA_INFO: Option<vesa::VesaInfo> = None;

static mut ATA_DRIVE: AtaDrive = AtaDrive::new();
static mut ATA_MOUNTED: bool = false;
static mut ATA_DIR_ENTRIES: [[u8; 32]; 64] = [[0; 32]; 64];
static mut ATA_DIR_COUNT: usize = 0;

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

static mut WALLPAPER: Color = Color::rgb(89, 0, 255);
static mut USE_BMP_WALLPAPER: bool = true;
static mut BMP_IMAGE: Option<BmpImage> = None;
static mut BMP_IMAGE2: Option<BmpImage> = None;
static mut BMP_IMAGE3: Option<BmpImage> = None;
static mut CURRENT_BMP_INDEX: usize = 0;

static mut ICON_TERMINAL: Option<BmpImage> = None;
static mut ICON_FILES: Option<BmpImage> = None;
static mut ICON_SETTINGS: Option<BmpImage> = None;

static mut THEME_IDX: usize = 0;
static mut TERM_HISTORY: [[u8; 80]; 20] = [[b' '; 80]; 20];
static mut TERM_HIST_LEN: usize = 0;

fn mount_ata_drive(vga: &mut crate::vga::VGA) {
    unsafe {
        let mut drive = AtaDrive::new();
        drive.init(vga);
        
        if drive.exists {
            ATA_DRIVE = drive;
            ATA_MOUNTED = true;
            vga.write_string("[ATA] Drive mounted successfully!\n");
            
            let mut buffer = [0u8; 512];
            if ATA_DRIVE.read_sector(1, &mut buffer) {
                parse_ata_directory(&buffer);
            }
        } else {
            vga.write_string("[ATA] No ATA drive detected!\n");
        }
    }
}

fn parse_ata_directory(data: &[u8; 512]) {
    unsafe {
        ATA_DIR_COUNT = 0;
        let mut offset = 0;
        
        while offset < 512 && ATA_DIR_COUNT < 64 {
            if data[offset] != 0 {
                let name_len = data[offset] as usize;
                if name_len > 0 && name_len <= 31 {
                    let start = offset + 1;
                    for i in 0..name_len {
                        if start + i < 512 {
                            ATA_DIR_ENTRIES[ATA_DIR_COUNT][i] = data[start + i];
                        }
                    }
                    ATA_DIR_COUNT += 1;
                    offset += 1 + name_len + 4;
                } else {
                    offset += 1;
                }
            } else {
                offset += 1;
            }
        }
    }
}

fn read_ata_file(filename: &str, buffer: &mut [u8]) -> usize {
    unsafe {
        if !ATA_MOUNTED { return 0; }
        
        for i in 0..ATA_DIR_COUNT {
            let entry_name = str::from_utf8(&ATA_DIR_ENTRIES[i]).unwrap_or("");
            let entry_name_trimmed = entry_name.trim_end_matches('\0');
            
            if entry_name_trimmed == filename {
                let mut sector_buf = [0u8; 512];
                if ATA_DRIVE.read_sector(2 + i as u32, &mut sector_buf) {
                    let size = sector_buf[0] as usize;
                    let copy_len = if size > buffer.len() { buffer.len() } else { size };
                    buffer[..copy_len].copy_from_slice(&sector_buf[1..1 + copy_len]);
                    return copy_len;
                }
                return 0;
            }
        }
        0
    }
}

fn list_ata_files() -> &'static str {
    unsafe {
        if !ATA_MOUNTED {
            return "ATA drive not mounted!";
        }
        "ATA files listed in terminal"
    }
}

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

fn show_blue_screen_of_death(gfx: &mut Graphics) -> ! {
    gfx.clear(Color::rgb(0, 0, 128).to_u32());
    
    let screen_w = gfx.width;
    let screen_h = gfx.height;

    let title = "MiOS Mobile!";
    let title_width = title.len() * tamzen_font::FONT_WIDTH;
    let title_x = (screen_w - title_width) / 2;
    let title_y = screen_h / 2 - 60;
    gfx.draw_text(title_x, title_y, title, Color::WHITE.to_u32(), Color::rgb(0, 0, 128).to_u32());

    let error_msg = "Error code: 00000x3";
    let error_width = error_msg.len() * tamzen_font::FONT_WIDTH;
    let error_x = (screen_w - error_width) / 2;
    let error_y = screen_h / 2 - 20;
    gfx.draw_text(error_x, error_y, error_msg, Color::WHITE.to_u32(), Color::rgb(0, 0, 128).to_u32());

    let restart_msg = "Please restart your device!";
    let restart_width = restart_msg.len() * tamzen_font::FONT_WIDTH;
    let restart_x = (screen_w - restart_width) / 2;
    let restart_y = screen_h / 2 + 20;
    gfx.draw_text(restart_x, restart_y, restart_msg, Color::WHITE.to_u32(), Color::rgb(0, 0, 128).to_u32());
    
    gfx.flush();
    
    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

fn show_blue_screen_stop_gui(gfx: &mut Graphics) -> ! {
    gfx.clear(Color::rgb(0, 0, 128).to_u32());
    
    let screen_w = gfx.width;
    let screen_h = gfx.height;
    
    let title = "GUI STOP!";
    let title_width = title.len() * tamzen_font::FONT_WIDTH;
    let title_x = (screen_w - title_width) / 2;
    let title_y = screen_h / 2 - 60;
    gfx.draw_text(title_x, title_y, title, Color::WHITE.to_u32(), Color::rgb(0, 0, 128).to_u32());
    
    let error_msg = "Error code: 00000x4c";
    let error_width = error_msg.len() * tamzen_font::FONT_WIDTH;
    let error_x = (screen_w - error_width) / 2;
    let error_y = screen_h / 2 - 20;
    gfx.draw_text(error_x, error_y, error_msg, Color::WHITE.to_u32(), Color::rgb(0, 0, 128).to_u32());
    
    let restart_msg = "GUI system halted. System stopped.";
    let restart_width = restart_msg.len() * tamzen_font::FONT_WIDTH;
    let restart_x = (screen_w - restart_width) / 2;
    let restart_y = screen_h / 2 + 20;
    gfx.draw_text(restart_x, restart_y, restart_msg, Color::WHITE.to_u32(), Color::rgb(0, 0, 128).to_u32());
    
    gfx.flush();
    
    loop {
        unsafe { core::arch::asm!("hlt") }
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
            
            let mut vga = crate::vga::VGA::new();
            vga.clear();
            vga.write_string("MiOS Mobile booting...\n");
            vga.write_string("Initializing VESA framebuffer...\n");

            unsafe {
                BMP_IMAGE = BmpImage::from_bytes(IMG_BMP);
                BMP_IMAGE2 = BmpImage::from_bytes(IMG2_BMP);
                BMP_IMAGE3 = BmpImage::from_bytes(IMG3_BMP);
                ICON_TERMINAL = BmpImage::from_bytes(IC_BMP);
                ICON_FILES = BmpImage::from_bytes(IC0_BMP);
                ICON_SETTINGS = BmpImage::from_bytes(IC1_BMP);
            }

            vga.write_string("Loading ATA driver...\n");
            mount_ata_drive(&mut vga);
            
            vga.write_string("Initializing Window Manager in Mobile mode...\n");

            let mut wm = WindowManager::new();

            let screen_w = gfx.width;
            let screen_h = gfx.height;

            gfx.clear(Color::BLACK.to_u32());

            let title = "MiOS Mobile";
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
            fn clear_history() { 
                unsafe { 
                    TERM_HIST_LEN = 0;
                    for i in 0..20 {
                        for j in 0..80 {
                            TERM_HISTORY[i][j] = b' ';
                        }
                    }
                } 
            }
            
            fn exec_command(input: &str) -> &'static str {
                match input.trim() {
                    "help"   => "  System:        ver, uptime, mem, cls, about, tech, kernel\n  Files:         dir, fs, ata, tals\n  Dev:           devs, developers, mide\n  Fun:           dice\n  Advanced:      stop gui, del ram",
                    "ver"    => "MiOS Mobile 1.0",
                    "mem"    => "Memory: error",
                    "uptime" => "Uptime: 0 ticks",
                    "dice"   => "Dice: 4",
                    "fs"     => "FAT32",
                    "ata"    => {
                        if unsafe { ATA_MOUNTED } {
                            "ATA drive is mounted!"
                        } else {
                            "No ATA drive detected!"
                        }
                    },
                    "tals"   => list_ata_files(),
                    "devs"   => "MiOS Developers: MDEVS",
                    "developers"   => "MiOS Developers: MDEVS",
                    "kernel"   => "MiOS KERNEL:\nFILES KERNEL: lib.rs , wm.rs\nKERNEL BILDER: Mikhail\nKERNEL VER: v5.7\nMade in 2026",
                    "mide"   => "\nmmmmmmmmmmmmmmmmmmmmmmm          MiDE 5\nm MiDE         _ O X  m          MiDE FULL version: 5.3 \nmmmmmmmmmmmmmmmmmmmmmmm          MiOS Version: Mobile 1.0\nm                     m          Kernel MiDE: ver1.0\nm                     m          made in 2026\nm                     m          MiDE by MDEVS (C)\nmmmmmmmmmmmmmmmmmmmmmmm",  
                    "cls"    => "\x04",
                    "commands"  => "hello!",
                    "about"  => "MiOS Mobile 1.0 - Made in 2026",
                    "tech"   => "\nMM       MM     MiOS\nM  M   M  M     Version OS: MiOS Mobile 1.0\nM    M    M     Desktop: MiDE v5.3\nM         M     CPU: X64\nM         M     MiOS By MDEVS",
                    "dir"    => "\nC:/MiOS/\nkernel.bin - lib.rs\nmouse.rs - mouse USB\nvm.rs - MiDE 5.2\nvga.rs - vga mode\nbmp.rs - bmp img system\nfs>\n   fat32.rs\n   mod.rs\n   tar.rs",
                    "stop gui" => "\x06",
                    "del ram" => "\x05",
                    ""       => "",
                    _        => "Unknown command, error: 00000x5",
                }
            }

            fn draw_window_content(gfx: &mut Graphics, win: &wm::Window, _idx: usize) {
                if win.state == WindowState::Minimized {
                    return;
                }

                let t = wm::get_theme();
                let cx = win.x + 2;
                let cy = win.y + 35;
                
                match win.title {
                    "Terminal" => {
                        let history = unsafe { &TERM_HISTORY };
                        let hist_len = unsafe { TERM_HIST_LEN };
                        let bg_color = Color::rgb(0, 0, 0);
                        
                        gfx.fill_rect(cx, cy, win.width - 4, win.height - 37, bg_color.to_u32());
                        
                        let max_lines = (win.height - 37 - 20) / 16;
                        let start_row = if hist_len > max_lines { hist_len - max_lines } else { 0 };
                        
                        for row in start_row..hist_len {
                            let line = core::str::from_utf8(&history[row]).unwrap_or("");
                            let max_chars = (win.width - 4 - 20) / 8;
                            let display_line = if line.len() > max_chars { 
                                &line[..max_chars] 
                            } else { 
                                line 
                            };
                            gfx.draw_text(cx + 10, cy + 10 + (row - start_row) * 16, display_line, Color::WHITE.to_u32(), bg_color.to_u32());
                        }
                        let prompt_y = cy + 10 + (hist_len - start_row) * 16;
                        gfx.draw_text(cx + 10, prompt_y, "MiOS> ", Color::WHITE.to_u32(), bg_color.to_u32());
                    }
                    "Files" => {
                        gfx.fill_rect(cx, cy, win.width - 4, win.height - 37, Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 10, "File Manager", Color::BLACK.to_u32(), Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 30, "═══════════════════════", Color::BLACK.to_u32(), Color::WHITE.to_u32());
                        
                        gfx.draw_text(cx + 10, cy + 50, "System files:", Color::BLACK.to_u32(), Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 70, "  kernel.bin", Color::rgb(80, 80, 80).to_u32(), Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 90, "  mios.bin", Color::rgb(80, 80, 80).to_u32(), Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 110, "  lib.rs", Color::rgb(80, 80, 80).to_u32(), Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 130, "  wm.rs", Color::rgb(80, 80, 80).to_u32(), Color::WHITE.to_u32());
                        
                        unsafe {
                            if ATA_MOUNTED && ATA_DIR_COUNT > 0 {
                                gfx.draw_text(cx + 10, cy + 160, "ATA Drive:", Color::BLACK.to_u32(), Color::WHITE.to_u32());
                                for i in 0..ATA_DIR_COUNT.min(5) {
                                    let filename = str::from_utf8(&ATA_DIR_ENTRIES[i]).unwrap_or("");
                                    let filename_trimmed = filename.trim_end_matches('\0');
                                    let mut buf = [0u8; 32];
                                    let mut pos = 0;
                                    for ch in "  ".bytes() {
                                        if pos < 31 { buf[pos] = ch; pos += 1; }
                                    }
                                    for ch in filename_trimmed.bytes() {
                                        if pos < 31 { buf[pos] = ch; pos += 1; }
                                    }
                                    buf[pos] = 0;
                                    let text = core::str::from_utf8(&buf[..pos]).unwrap_or("");
                                    gfx.draw_text(cx + 10, cy + 180 + i * 20, text, Color::rgb(0, 0, 180).to_u32(), Color::WHITE.to_u32());
                                }
                            }
                        }
                    }
                    "Settings" => {
                        gfx.fill_rect(cx, cy, win.width - 4, win.height - 37, Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 10, "Settings", Color::BLACK.to_u32(), Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 30, "═══════════════════════", Color::BLACK.to_u32(), Color::WHITE.to_u32());

                        let btn_y1 = cy + 50;
                        wm::draw_raised_rect(gfx, cx + 10, btn_y1, win.width - 30, 30);
                        gfx.draw_text(cx + 20, btn_y1 + 8, "1 - Change Wallpaper", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

                        let btn_y2 = cy + 95;
                        wm::draw_raised_rect(gfx, cx + 10, btn_y2, win.width - 30, 30);
                        let bmp_status = if unsafe { USE_BMP_WALLPAPER } { "ON" } else { "OFF" };
                        let mut buf = [0u8; 32];
                        let mut pos = 0;
                        for ch in "2 - Wallpaper: ".bytes() {
                            if pos < 31 { buf[pos] = ch; pos += 1; }
                        }
                        for ch in bmp_status.bytes() {
                            if pos < 31 { buf[pos] = ch; pos += 1; }
                        }
                        buf[pos] = 0;
                        let display = core::str::from_utf8(&buf[..pos]).unwrap_or("2 - Wallpaper: ?");
                        gfx.draw_text(cx + 20, btn_y2 + 8, display, Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

                        let btn_y3 = cy + 140;
                        wm::draw_raised_rect(gfx, cx + 10, btn_y3, win.width - 30, 30);
                        gfx.draw_text(cx + 20, btn_y3 + 8, "3 - About OS", Color::BLACK.to_u32(), Color::rgb(212, 208, 200).to_u32());

                        let info_y = cy + 190;
                        gfx.draw_text(cx + 10, info_y, "Resolution: 800x600", Color::BLACK.to_u32(), Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, info_y + 25, "MiOS Mobile 1.0", Color::BLACK.to_u32(), Color::WHITE.to_u32());
                    }
                    "About OS" => {
                        gfx.fill_rect(cx, cy, win.width - 4, win.height - 37, Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 10, "About OS", Color::BLACK.to_u32(), Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 30, "═══════════════════════", Color::BLACK.to_u32(), Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 60, "MiOS Mobile 1.0", Color::BLACK.to_u32(), Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 90, "By MDEVS (C)", Color::BLACK.to_u32(), Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 120, "═══════════════════════", Color::BLACK.to_u32(), Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 150, "Mobile OS for x86", Color::BLACK.to_u32(), Color::WHITE.to_u32());
                        gfx.draw_text(cx + 10, cy + 180, "Made in 2026", Color::BLACK.to_u32(), Color::WHITE.to_u32());
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
                    _ => {}
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
                let taskbar_h = 35;
                let taskbar_y = 0;

                gfx.fill_rect(0, taskbar_y, gfx.width, taskbar_h, Color::rgb(30, 144, 255).to_u32());
                gfx.draw_rect_border(0, taskbar_y, gfx.width, taskbar_h, Color::rgb(0, 70, 150).to_u32());

                wm::draw_raised_rect(gfx, 4, taskbar_y + 3, 50, taskbar_h - 6);
                gfx.draw_text(12, taskbar_y + 10, "Start", Color::BLACK.to_u32(), Color::rgb(200, 200, 210).to_u32());

                let time_str = "16:20";
                let time_width = time_str.len() * 8;
                let time_x = (gfx.width - time_width) / 2;
                gfx.draw_text(time_x, taskbar_y + 10, time_str, Color::WHITE.to_u32(), Color::rgb(30, 144, 255).to_u32());

                let settings_w = 60;
                let settings_x = gfx.width - settings_w - 10;
                wm::draw_raised_rect(gfx, settings_x, taskbar_y + 3, settings_w, taskbar_h - 6);
                gfx.draw_text(settings_x + 8, taskbar_y + 10, "⚙", Color::BLACK.to_u32(), Color::rgb(200, 200, 210).to_u32());

                wm.start_menu.draw(gfx);

                wm.draw_all(gfx, &|gfx, win, _idx| {
                    draw_window_content(gfx, win, _idx);
                });

                if let Some(active) = wm.active_window {
                    if let Some(ref win) = wm.windows[active] {
                        if win.title == "Terminal" && win.state != WindowState::Minimized {
                            let cx = win.x + 2;
                            let cy = win.y + 35;
                            let hist_len = unsafe { TERM_HIST_LEN };
                            let prompt_y = cy + 10 + hist_len * 16;
                            if term_len > 0 {
                                let text = core::str::from_utf8(&term_input[..term_len]).unwrap_or("");
                                gfx.draw_text(cx + 10 + 6*8, prompt_y, text, Color::WHITE.to_u32(), Color::BLACK.to_u32());
                            }
                        }
                    }
                }
            }

            redraw_all(&mut gfx, &wm, &term_input, term_len);
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
                    if let Some(active) = wm.active_window {
                        if let Some(ref win) = wm.windows[active] {
                            if win.title == "Terminal" && win.state != WindowState::Minimized {
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
                                    0x02 => Some('1'), 0x03 => Some('2'), 0x04 => Some('3'),
                                    0x05 => Some('4'), 0x06 => Some('5'), 0x07 => Some('6'),
                                    0x08 => Some('7'), 0x09 => Some('8'), 0x0A => Some('9'),
                                    0x0B => Some('0'),
                                    0x33 => Some(','), 0x34 => Some('.'), 0x35 => Some('/'),
                                    0x27 => Some(';'), 0x28 => Some('\''), 0x2B => Some('\\'),
                                    0x29 => Some('`'), 0x0C => Some('-'), 0x0D => Some('='),
                                    0x1A => Some('['), 0x1B => Some(']'),
                                    _ => None,
                                };
                                if let Some(ch) = c {
                                    if ch == '\n' {
                                        let cmd = core::str::from_utf8(&term_input[..term_len]).unwrap_or("");
                                        let out = exec_command(cmd);
                                        if out == "\x04" {
                                            clear_history();
                                            push_history("MiOS> cls");
                                        } else if out == "\x05" {
                                            show_blue_screen_of_death(&mut gfx);
                                        } else if out == "\x06" {
                                            show_blue_screen_stop_gui(&mut gfx);
                                        } else {
                                            push_history("MiOS> ");
                                            if !cmd.is_empty() {
                                                push_history(cmd);
                                                if !out.is_empty() { 
                                                    for line in out.split('\n') {
                                                        push_history(line);
                                                    }
                                                }
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

                    if packet.left && !left_prev {
                        let taskbar_h = 35;
                        let taskbar_y = 0;

                        if cursor_y >= taskbar_y && cursor_y <= taskbar_y + taskbar_h {
                            if cursor_x >= 4 && cursor_x <= 54 {
                                wm.start_menu.open = !wm.start_menu.open;
                                redraw = true;
                            }
                            let settings_x = gfx.width - 70;
                            if cursor_x >= settings_x && cursor_x <= settings_x + 60 {
                                wm.create_window("Settings");
                                redraw = true;
                            }
                        } else {
                            if wm.start_menu.open && !wm.start_menu.contains(cursor_x, cursor_y) {
                                wm.start_menu.open = false;
                                redraw = true;
                            }
                        }

                        if wm.start_menu.open {
                            if let Some(action) = wm.start_menu.handle_click(cursor_x, cursor_y) {
                                match action {
                                    "Terminal" => { 
                                        wm.create_window("Terminal"); 
                                        term_len = 0; 
                                        clear_history(); 
                                    }
                                    "Files"    => { 
                                        wm.create_window("Files"); 
                                    }
                                    "Settings" => { 
                                        wm.create_window("Settings"); 
                                    }
                                    "Shutdown" => {
                                        gfx.clear(Color::BLACK.to_u32());
                                        gfx.draw_text(
                                            (gfx.width - 16 * 28) / 2,
                                            gfx.height / 2 - 8,
                                            "Now you can turn off the power",
                                            Color::WHITE.to_u32(),
                                            Color::BLACK.to_u32()
                                        );
                                        gfx.flush();
                                        loop { unsafe { core::arch::asm!("hlt") } }
                                    }
                                    _ => {}
                                }
                                wm.start_menu.open = false;
                                redraw = true;
                            }
                        }

                        if !wm.start_menu.open || !wm.start_menu.contains(cursor_x, cursor_y) {
                            if wm.handle_mouse_press(&mut gfx, cursor_x, cursor_y) {
                                redraw = true;
                            }
                        }

                        let settings_window = {
                            let mut found = None;
                            for i in 0..wm.window_count {
                                if let Some(ref win) = wm.windows[i] {
                                    if win.title == "Settings" && win.state != WindowState::Minimized {
                                        found = Some((win.x, win.y, win.width));
                                        break;
                                    }
                                }
                            }
                            found
                        };

                        if let Some((wx, wy, ww)) = settings_window {
                            let cy_offset = wy + 35;
                            let btn_y1 = cy_offset + 50;
                            if cursor_x >= wx + 10 && cursor_x <= wx + ww - 20 &&
                               cursor_y >= btn_y1 && cursor_y <= btn_y1 + 30 {
                                wm.create_window("Wallpaper Selector");
                                redraw = true;
                            }
                            
                            let btn_y2 = cy_offset + 95;
                            if cursor_x >= wx + 10 && cursor_x <= wx + ww - 20 &&
                               cursor_y >= btn_y2 && cursor_y <= btn_y2 + 30 {
                                unsafe { USE_BMP_WALLPAPER = !USE_BMP_WALLPAPER; }
                                redraw = true;
                            }
                            
                            let btn_y3 = cy_offset + 140;
                            if cursor_x >= wx + 10 && cursor_x <= wx + ww - 20 &&
                               cursor_y >= btn_y3 && cursor_y <= btn_y3 + 30 {
                                wm.create_window("About OS");
                                redraw = true;
                            }
                        }

                        let wallpaper_selector = {
                            let mut found = None;
                            for i in 0..wm.window_count {
                                if let Some(ref win) = wm.windows[i] {
                                    if win.title == "Wallpaper Selector" && win.state != WindowState::Minimized {
                                        found = Some((i, win.x, win.y));
                                        break;
                                    }
                                }
                            }
                            found
                        };

                        if let Some((idx, wx, wy)) = wallpaper_selector {
                            let colors = [
                                Color::rgb(89, 0, 255),
                                Color::rgb(0, 23, 128),
                                Color::rgb(192, 0, 0),
                                Color::rgb(192, 192, 0),
                                Color::rgb(0, 0, 128),
                                Color::rgb(128, 0, 128),
                            ];
                            for (j, color) in colors.iter().enumerate() {
                                let y_pos = wy + 30 + j * 30 + 35;
                                if cursor_x >= wx + 10 && cursor_x <= wx + 260 &&
                                   cursor_y >= y_pos && cursor_y <= y_pos + 24 {
                                    unsafe { 
                                        WALLPAPER = *color;
                                        USE_BMP_WALLPAPER = false;
                                    }
                                    for k in idx..wm.window_count - 1 {
                                        wm.windows[k] = wm.windows[k + 1].take();
                                    }
                                    wm.windows[wm.window_count - 1] = None;
                                    wm.window_count -= 1;
                                    if wm.active_window == Some(idx) || wm.active_window.is_none() {
                                        wm.active_window = if wm.window_count > 0 { Some(wm.window_count - 1) } else { None };
                                    }
                                    redraw = true;
                                    break;
                                }
                            }
                        }
                    } else if !packet.left && left_prev {
                        wm.handle_mouse_release();
                        redraw = true;
                    }

                    left_prev = packet.left;

                    if redraw {
                        redraw_all(&mut gfx, &wm, &term_input, term_len);
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