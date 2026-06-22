use crate::graphics::{self, Graphics, Color};
use crate::bmp::BmpImage;

static mut BG_BUF: [u32; 800 * 600] = [0; 800 * 600];

pub struct Theme {
    pub window_bg: Color,
    pub taskbar_bg: Color,
    pub start_menu_bg: Color,
    pub taskbar_text: Color,
    pub desktop_icon_text: Color,
    pub button_face: Color,
}

pub static THEMES: [Theme; 3] = [
    Theme {
        window_bg: Color::rgb(248, 248, 252),
        taskbar_bg: Color::rgb(30, 144, 255),
        start_menu_bg: Color::rgb(245, 245, 250),
        taskbar_text: Color::WHITE,
        desktop_icon_text: Color::WHITE,
        button_face: Color::rgb(200, 200, 210),
    },
    Theme {
        window_bg: Color::rgb(35, 35, 40),
        taskbar_bg: Color::rgb(50, 50, 55),
        start_menu_bg: Color::rgb(45, 45, 50),
        taskbar_text: Color::WHITE,
        desktop_icon_text: Color::WHITE,
        button_face: Color::rgb(80, 80, 90),
    },
    Theme {
        window_bg: Color::rgb(240, 240, 245),
        taskbar_bg: Color::rgb(170, 170, 180),
        start_menu_bg: Color::rgb(235, 235, 240),
        taskbar_text: Color::BLACK,
        desktop_icon_text: Color::BLACK,
        button_face: Color::rgb(180, 180, 190),
    },
];

static mut CURRENT_THEME: usize = 0;

pub fn set_theme(idx: usize) {
    unsafe { CURRENT_THEME = idx % THEMES.len(); }
}

pub fn get_theme() -> &'static Theme {
    &THEMES[unsafe { CURRENT_THEME }]
}

pub fn draw_raised_rect(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    if w == 0 || h == 0 { return; }
    let face = Color::rgb(200, 200, 210);
    let light = Color::rgb(255, 255, 255);
    let shadow = Color::rgb(80, 80, 90);
    let dark_shadow = Color::rgb(40, 40, 45);

    gfx.fill_rect(x, y, w, h, face.to_u32());

    for dx in 0..w { gfx.put_pixel(x + dx, y, light.to_u32()); }
    for dy in 1..h { gfx.put_pixel(x, y + dy, light.to_u32()); }
    for dy in 1..h { gfx.put_pixel(x + w - 1, y + dy, shadow.to_u32()); }
    for dx in 0..w { gfx.put_pixel(x + dx, y + h - 1, shadow.to_u32()); }

    gfx.put_pixel(x + w - 1, y + h - 1, dark_shadow.to_u32());
}

pub fn draw_sunken_rect(gfx: &mut Graphics, x: usize, y: usize, w: usize, h: usize) {
    if w == 0 || h == 0 { return; }
    let face = Color::rgb(200, 200, 210);
    let light = Color::rgb(255, 255, 255);
    let shadow = Color::rgb(50, 50, 55);
    let dark_shadow = Color::rgb(30, 30, 35);

    gfx.fill_rect(x, y, w, h, face.to_u32());

    for dx in 0..w { gfx.put_pixel(x + dx, y, shadow.to_u32()); }
    for dy in 1..h { gfx.put_pixel(x, y + dy, shadow.to_u32()); }
    for dy in 1..h { gfx.put_pixel(x + w - 1, y + dy, light.to_u32()); }
    for dx in 0..w { gfx.put_pixel(x + dx, y + h - 1, light.to_u32()); }

    gfx.put_pixel(x, y, dark_shadow.to_u32());
}

#[derive(Clone, Copy, PartialEq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
}

pub struct Window {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub title: &'static str,
    pub is_dragging: bool,
    pub drag_off_x: usize,
    pub drag_off_y: usize,
    pub state: WindowState,
    pub restore_rect: (usize, usize, usize, usize),
}

impl Window {
    pub fn new(x: usize, y: usize, w: usize, h: usize, title: &'static str) -> Self {
        Self {
            x, y, width: w, height: h, title,
            is_dragging: false,
            drag_off_x: 0, drag_off_y: 0,
            state: WindowState::Normal,
            restore_rect: (x, y, w, h),
        }
    }

    pub fn draw_mobile(&self, gfx: &mut Graphics, active: bool) {
        if self.state == WindowState::Minimized { return; }
        let t = get_theme();
        let (x, y, w, h) = (self.x, self.y, self.width, self.height);
        let titlebar_h = 35;

        let bg_color = t.window_bg;

        gfx.fill_rect(x, y + titlebar_h, w, h - titlebar_h, bg_color.to_u32());

        for row in 0..titlebar_h {
            let color = bg_color.to_u32();
            for dx in 0..w {
                gfx.put_pixel(x + dx, y + row, color);
            }
        }

        let title_len = self.title.len() * 8;
        let title_x = x + (w - title_len) / 2;
        let text_color = if active { Color::WHITE } else { Color::rgb(150, 150, 160) };
        gfx.draw_text(title_x, y + 10, self.title, text_color.to_u32(), bg_color.to_u32());

        let btn_size = 24;
        let btn_x = x + w - btn_size - 10;
        let btn_y = y + (titlebar_h - btn_size) / 2;
        
        gfx.fill_rect(btn_x, btn_y, btn_size, btn_size, Color::rgb(200, 50, 50).to_u32());
        gfx.draw_rect_border(btn_x, btn_y, btn_size, btn_size, Color::rgb(150, 30, 30).to_u32());
        
        let half = btn_size / 2;
        let cross_len = 12;
        let start = (btn_size - cross_len) / 2;
        for i in 0..cross_len {
            gfx.put_pixel(btn_x + start + i, btn_y + half - 1, Color::WHITE.to_u32());
            gfx.put_pixel(btn_x + start + i, btn_y + half, Color::WHITE.to_u32());
            gfx.put_pixel(btn_x + half - 1, btn_y + start + i, Color::WHITE.to_u32());
            gfx.put_pixel(btn_x + half, btn_y + start + i, Color::WHITE.to_u32());
        }

        gfx.draw_rect_border(x, y, w, h, Color::rgb(80, 80, 100).to_u32());
    }

    pub fn contains(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
    
    pub fn is_titlebar(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + 35
    }
    
    pub fn is_close_button(&self, x: usize, y: usize) -> bool {
        let btn_size = 24;
        let btn_x = self.x + self.width - btn_size - 10;
        let btn_y = self.y + (35 - btn_size) / 2;
        x >= btn_x && x < btn_x + btn_size && y >= btn_y && y < btn_y + btn_size
    }
}

pub struct Menu {
    pub open: bool,
    pub items: [&'static str; 4],
}

impl Menu {
    pub const fn new() -> Self {
        Self { open: false, items: ["Terminal", "Files", "Settings", "Shutdown"] }
    }
    
    pub fn draw(&self, gfx: &mut Graphics) {
        if !self.open { return; }
        let t = get_theme();
        let x = 10;
        let y = 45;
        let w = 180;
        let h = 115;
        gfx.fill_rect(x, y, w, h, t.start_menu_bg.to_u32());
        gfx.draw_rect_border(x, y, w, h, Color::rgb(80, 80, 90).to_u32());
        for (i, item) in self.items.iter().enumerate() {
            gfx.draw_text(x + 12, y + 12 + i * 22, item, Color::BLACK.to_u32(), t.start_menu_bg.to_u32());
        }
    }
    
    pub fn contains(&self, x: usize, y: usize) -> bool {
        if !self.open { return false; }
        let sx = 10;
        let sy = 45;
        x >= sx && x < sx + 180 && y >= sy && y < sy + 115
    }
    
    pub fn handle_click(&self, cx: usize, cy: usize) -> Option<&'static str> {
        if !self.open { return None; }
        let x = 10;
        let y = 45;
        if cx < x || cx > x + 180 || cy < y || cy > y + 115 { return None; }
        let idx = (cy - y - 12) / 22;
        if idx < self.items.len() {
            Some(self.items[idx])
        } else { None }
    }
}

pub struct WindowManager {
    pub windows: [Option<Window>; 8],
    pub window_count: usize,
    pub dragged_window: Option<usize>,
    pub active_window: Option<usize>,
    pub start_menu: Menu,
    pub mobile_mode: bool,
}

impl WindowManager {
    pub const fn new() -> Self {
        const NONE: Option<Window> = None;
        Self {
            windows: [NONE; 8],
            window_count: 0,
            dragged_window: None,
            active_window: None,
            start_menu: Menu::new(),
            mobile_mode: true,
        }
    }

    pub fn create_window(&mut self, title: &'static str) {
        if self.window_count < 8 {
            let taskbar_h = 35;
            let x = 0;
            let y = taskbar_h;
            let w = 800;
            let h = 600 - taskbar_h;
            self.windows[self.window_count] = Some(Window::new(x, y, w, h, title));
            self.active_window = Some(self.window_count);
            self.window_count += 1;
        }
    }

    pub fn draw_all(&self, gfx: &mut Graphics, draw_content: &dyn Fn(&mut Graphics, &Window, usize)) {
        for i in 0..self.window_count {
            if let Some(ref win) = self.windows[i] {
                let active = self.active_window == Some(i);
                win.draw_mobile(gfx, active);
                draw_content(gfx, win, i);
            }
        }
    }

    pub fn handle_mouse_press(&mut self, gfx: &mut Graphics, cx: usize, cy: usize) -> bool {
        for i in (0..self.window_count).rev() {
            if let Some(ref win) = self.windows[i] {
                if win.is_close_button(cx, cy) {
                    for j in i..self.window_count - 1 {
                        self.windows[j] = self.windows[j + 1].take();
                    }
                    self.windows[self.window_count - 1] = None;
                    self.window_count -= 1;
                    self.dragged_window = None;
                    self.active_window = if self.window_count > 0 { Some(self.window_count - 1) } else { None };
                    return true;
                }
            }
        }
        
        for i in (0..self.window_count).rev() {
            if let Some(ref win) = self.windows[i] {
                if win.is_titlebar(cx, cy) {
                    self.active_window = Some(i);
                    return true;
                }
            }
        }
        false
    }

    pub fn handle_mouse_release(&mut self) -> bool {
        if self.dragged_window.is_some() {
            self.dragged_window = None;
            return true;
        }
        false
    }

    pub fn get_window_title(&self, idx: usize) -> Option<&'static str> {
        self.windows[idx].as_ref().map(|w| w.title)
    }
}