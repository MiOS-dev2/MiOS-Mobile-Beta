//! Простой декодер PNG изображений
//! Поддерживает только PNG с фильтром 0 (None) и без сжатия для простоты в ядре

use core::slice;

pub struct PngImage {
    pub data: &'static [u8],  // RGBA данные
    pub width: usize,
    pub height: usize,
}

impl PngImage {
    pub fn from_bytes(bytes: &'static [u8]) -> Option<Self> {
        // Проверяем PNG сигнатуру
        if bytes.len() < 8 || &bytes[0..8] != [137, 80, 78, 71, 13, 10, 26, 10] {
            return None;
        }
        
        let mut width = 0;
        let mut height = 0;
        let mut idat_start = 0;
        let mut idat_len = 0;
        let mut bit_depth = 0;
        let mut color_type = 0;
        
        let mut offset = 8;
        
        while offset + 8 <= bytes.len() {
            let chunk_len = ((bytes[offset] as u32) << 24) |
                           ((bytes[offset + 1] as u32) << 16) |
                           ((bytes[offset + 2] as u32) << 8) |
                           (bytes[offset + 3] as u32);
            let chunk_type = &bytes[offset + 4..offset + 8];
            
            if chunk_type == b"IHDR" {
                if offset + 8 + 13 > bytes.len() { return None; }
                width = ((bytes[offset + 8] as u32) << 24) |
                       ((bytes[offset + 9] as u32) << 16) |
                       ((bytes[offset + 10] as u32) << 8) |
                       (bytes[offset + 11] as u32);
                height = ((bytes[offset + 12] as u32) << 24) |
                        ((bytes[offset + 13] as u32) << 16) |
                        ((bytes[offset + 14] as u32) << 8) |
                        (bytes[offset + 15] as u32);
                bit_depth = bytes[offset + 16];
                color_type = bytes[offset + 17];
                
                if width == 0 || height == 0 { return None; }
                if bit_depth != 8 { return None; }
                if color_type != 6 { return None; } // Только RGBA
            }
            
            if chunk_type == b"IDAT" {
                idat_start = offset + 8;
                idat_len = chunk_len as usize;
                break;
            }
            
            offset += 4 + 4 + chunk_len as usize + 4; // длина, тип, данные, CRC
        }
        
        if idat_len == 0 { return None; }
        
        let compressed_data = &bytes[idat_start..idat_start + idat_len];
        
        // Простая распаковка PNG (без сжатия)
        // Для ядра мы ожидаем PNG с фильтром 0 и без сжатия
        let image_size = (width as usize) * (height as usize) * 4;
        let mut rgba_data = vec![0u8; image_size];
        
        // Пропускаем заголовок zlib (2 байта)
        if compressed_data.len() < 2 { return None; }
        let mut src_idx = 2;
        let mut dst_idx = 0;
        let bpp = 4; // bytes per pixel (RGBA)
        let stride = width as usize * bpp;
        
        for y in 0..height as usize {
            if src_idx >= compressed_data.len() { break; }
            let filter = compressed_data[src_idx];
            src_idx += 1;
            
            match filter {
                0 => {
                    for x in 0..stride {
                        if src_idx < compressed_data.len() {
                            rgba_data[dst_idx] = compressed_data[src_idx];
                            src_idx += 1;
                            dst_idx += 1;
                        }
                    }
                }
                _ => {
                    // Другие фильтры не поддерживаются в упрощенной версии
                    return None;
                }
            }
        }
        
        // Создаем статическую ссылку
        let static_data = Box::leak(rgba_data.into_boxed_slice());
        
        Some(PngImage {
            data: static_data,
            width: width as usize,
            height: height as usize,
        })
    }
}