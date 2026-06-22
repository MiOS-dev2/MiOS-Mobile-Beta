// sound.rs - PC Speaker звуковые эффекты через PIT
#![allow(dead_code)]

use core::arch::asm;

const PIT_FREQUENCY: u32 = 1193182; // Базовая частота PIT

// Порт для PC Speaker
const SPEAKER_PORT: u16 = 0x61;
// Порт для PIT канала 2
const PIT_CHANNEL2: u16 = 0x42;
// Командный порт PIT
const PIT_CMD: u16 = 0x43;

// Команда для установки частоты на канале 2
const PIT_CMD_CHANNEL2: u8 = 0xB6;

/// Включить PC Speaker с заданной частотой (Гц)
pub unsafe fn speaker_on(freq_hz: u32) {
    if freq_hz == 0 { return; }
    
    let divisor = (PIT_FREQUENCY / freq_hz) as u16;
    
    // Отправляем команду PIT
    asm!("out dx, al", in("dx") PIT_CMD, in("al") PIT_CMD_CHANNEL2);
    
    // Отправляем делитель (младший байт)
    asm!("out dx, al", in("dx") PIT_CHANNEL2, in("al") (divisor & 0xFF) as u8);
    // Отправляем делитель (старший байт)
    asm!("out dx, al", in("dx") PIT_CHANNEL2, in("al") (divisor >> 8) as u8);
    
    // Включаем динамик: устанавливаем биты 0 и 1
    let mut speaker_val: u8;
    asm!("in al, dx", out("al") speaker_val, in("dx") SPEAKER_PORT);
    speaker_val |= 0x03;
    asm!("out dx, al", in("dx") SPEAKER_PORT, in("al") speaker_val);
}

/// Выключить PC Speaker
pub unsafe fn speaker_off() {
    let mut speaker_val: u8;
    asm!("in al, dx", out("al") speaker_val, in("dx") SPEAKER_PORT);
    speaker_val &= !0x03;
    asm!("out dx, al", in("dx") SPEAKER_PORT, in("al") speaker_val);
}

/// Простая задержка через busy loop (примерно 1 мс на ~1000 итераций)
fn delay_ms(ms: u32) {
    for _ in 0..(ms * 1000) {
        unsafe { asm!("nop") }
    }
}

/// Воспроизвести звук заданной частоты на заданное время (мс)
pub unsafe fn beep(freq_hz: u32, duration_ms: u32) {
    speaker_on(freq_hz);
    delay_ms(duration_ms);
    speaker_off();
}

// ============= ЗВУКОВЫЕ ЭФФЕКТЫ ДЛЯ КОМПОНЕНТОВ =============

/// Звук запуска ОС (загрузка)
pub unsafe fn sound_boot() {
    beep(880, 100);  // Ля
    delay_ms(50);
    beep(1047, 100); // До
    delay_ms(50);
    beep(1319, 150); // Ми
}

/// Звук клика по иконке/кнопке
pub unsafe fn sound_click() {
    beep(1200, 30);
}

/// Звук открытия окна
pub unsafe fn sound_window_open() {
    beep(800, 40);
    delay_ms(20);
    beep(1000, 50);
}

/// Звук закрытия окна
pub unsafe fn sound_window_close() {
    beep(1000, 30);
    delay_ms(15);
    beep(800, 40);
}

/// Звук сворачивания/разворачивания окна
pub unsafe fn sound_window_minimize() {
    beep(600, 25);
    delay_ms(10);
    beep(500, 35);
}

/// Звук ошибки
pub unsafe fn sound_error() {
    beep(400, 200);
    delay_ms(100);
    beep(400, 200);
}

/// Звук успешного действия
pub unsafe fn sound_success() {
    beep(1047, 60);
    delay_ms(30);
    beep(1319, 80);
}

/// Звук ввода с клавиатуры
pub unsafe fn sound_keypress() {
    beep(1500, 10);
}

/// Звук энтера
pub unsafe fn sound_enter() {
    beep(1000, 40);
    delay_ms(20);
    beep(1200, 50);
}

/// Звук бэкспейса
pub unsafe fn sound_backspace() {
    beep(800, 20);
}

/// Звук переключения темы
pub unsafe fn sound_theme_change() {
    beep(523, 60);  // До
    delay_ms(30);
    beep(587, 60);  // Ре
    delay_ms(30);
    beep(659, 60);  // Ми
}

/// Звук смены обоев
pub unsafe fn sound_wallpaper_change() {
    beep(440, 80);
    delay_ms(40);
    beep(523, 100);
}

/// Звук сохранения документа
pub unsafe fn sound_save_document() {
    beep(880, 50);
    delay_ms(50);
    beep(1047, 70);
}

/// Звук загрузки документа
pub unsafe fn sound_load_document() {
    beep(1047, 50);
    delay_ms(30);
    beep(880, 70);
}

/// Звук перезагрузки
pub unsafe fn sound_reboot() {
    beep(400, 150);
    delay_ms(100);
    beep(350, 150);
    delay_ms(100);
    beep(300, 200);
}

/// Звук выключения
pub unsafe fn sound_shutdown() {
    beep(523, 200);
    delay_ms(150);
    beep(392, 300);
}

/// Звук контекстного меню
pub unsafe fn sound_context_menu() {
    beep(700, 25);
}

/// Звук Paint - рисование точки
pub unsafe fn sound_paint_dot() {
    beep(2000, 5);
}

/// Звук Paint - смена инструмента
pub unsafe fn sound_paint_tool() {
    beep(900, 30);
    delay_ms(15);
    beep(1100, 30);
}

/// Звук Paint - смена цвета
pub unsafe fn sound_paint_color() {
    beep(1000, 25);
}

/// Звук меню Пуск
pub unsafe fn sound_start_menu() {
    beep(600, 30);
    delay_ms(20);
    beep(800, 40);
}

/// Звук логина
pub unsafe fn sound_login() {
    beep(659, 80);
    delay_ms(40);
    beep(784, 80);
    delay_ms(40);
    beep(880, 120);
}

/// Звук выхода из программы
pub unsafe fn sound_exit() {
    beep(880, 40);
    delay_ms(20);
    beep(784, 40);
    delay_ms(20);
    beep(659, 60);
}