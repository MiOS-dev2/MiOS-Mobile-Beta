use crate::console::Console;
use crate::fs::FSManager;
use crate::ata::AtaDrive;

pub struct Commands;

impl Commands {
    pub const fn new() -> Self { Self }
    pub fn execute(&mut self, cmd: &str, args: Option<&str>, console: &mut dyn Console, fs: &mut FSManager, ata: &AtaDrive) -> bool {
        match cmd {
            "cls" => { console.clear(); true }
            "help" => { console.write_string("Commands: cls, help, ver, about, reboot, shutdown, uptime, mem, cpu, dice, flip, gui, dir, cd, read, write, create, mkdir, del\n"); true }
            "ver" => { console.write_string("CubeOS v0.7-dev\n"); true }
            "about" => { console.write_string("CubeOS by @cubedev\n"); true }
            "reboot" => { console.write_string("Rebooting...\n"); loop { unsafe { core::arch::asm!("cli; hlt"); } } }
            "shutdown" => { console.write_string("Shutting down...\n"); loop { unsafe { core::arch::asm!("hlt"); } } }
            "uptime" => { console.write_string("Uptime: 0 ticks\n"); true }
            "mem" => { console.write_string("Memory: 128 MB\n"); true }
            "cpu" => { console.write_string("CPU: x86_64 Long Mode\n"); true }
            "dice" => { console.write_string("Dice: 4\n"); true }
            "flip" => { console.write_string("Heads!\n"); true }
            "gui" => { console.write_string("Type 'gui' to launch graphical mode\n"); true }
            "dir" => {
                if fs.fat32.is_some() {
                    let _ = fs.dir(ata, console);
                    console.write_string("\n");
                } else { console.write_string("No FS mounted\n"); }
                true
            }
            "cd" => {
                if let Some(path) = args {
                    if fs.change_dir(ata, path) { console.write_string("OK\n"); }
                    else { console.write_string("Not found\n"); }
                } else { console.write_string("Usage: cd <dir>\n"); }
                true
            }
            "read" => {
                if let Some(path) = args {
                    let mut buf = [0u8; 512];
                    if let Some(size) = fs.read_file(ata, path, &mut buf) {
                        for i in 0..size { console.put_char(buf[i] as char); }
                        console.write_string("\n");
                    } else { console.write_string("Not found\n"); }
                } else { console.write_string("Usage: read <file>\n"); }
                true
            }
            "write" => {
                if let Some(path) = args {
                    let data = b"Hello from CubeOS!\n";
                    if fs.write_file(ata, path, data) { console.write_string("Written\n"); }
                    else { console.write_string("Failed\n"); }
                } else { console.write_string("Usage: write <file>\n"); }
                true
            }
            "create" => {
                if let Some(name) = args {
                    if fs.create_file(ata, name) { console.write_string("Created\n"); }
                    else { console.write_string("Failed\n"); }
                } else { console.write_string("Usage: create <file>\n"); }
                true
            }
            "mkdir" => {
                if let Some(name) = args {
                    if fs.create_dir(ata, name) { console.write_string("Created\n"); }
                    else { console.write_string("Failed\n"); }
                } else { console.write_string("Usage: mkdir <dir>\n"); }
                true
            }
            "del" => {
                if let Some(name) = args {
                    if fs.delete_file(ata, name) { console.write_string("Deleted\n"); }
                    else { console.write_string("Failed\n"); }
                } else { console.write_string("Usage: del <file>\n"); }
                true
            }
            _ => false,
        }
    }
}
