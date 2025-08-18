use std::{
    collections::VecDeque,
    sync::{Mutex, MutexGuard, OnceLock, atomic::Ordering},
};

use fontdue::{Font, FontSettings};

use super::DEBUG_FEATURES;

//---------------------------------------- Debug Overlay ---------------------------------------
//Is always drawn at 0,0 for minimal confusion and complexity
//This overlay is very raw (compared to eg.imgui)
//Lets us draw some debug information. Ideally toggled with a keybind.
static OVERLAY: OnceLock<Box<[u8]>> = OnceLock::new();
const OVERLAY_WIDTH: usize = 600;
const OVERLAY_HEIGHT: usize = 180;
static FONT_DATA: &[u8] = include_bytes!("../ui/segoeui.ttf");
static FONT: OnceLock<Font> = OnceLock::new();
const FONT_SIZE: f32 = 12.0;
static LOG: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();
const MAX_LOG_LINES: usize = 12;

const DEBUG_OVERLAY_BG_R: u8 = 0;
const DEBUG_OVERLAY_BG_G: u8 = 0;
const DEBUG_OVERLAY_BG_B: u8 = 20;
const DEBUG_OVERLAY_BG_A: u8 = 200;

fn get_overlay() -> &'static [u8] {
    OVERLAY.get_or_init(|| {
        let mut buf = vec![0u8; (OVERLAY_WIDTH * OVERLAY_HEIGHT * 4) as usize].into_boxed_slice();

        //Border
        let border_r = 200;
        let border_g = 200;
        let border_b = 200;
        let border_alpha = 255;

        for j in 0..OVERLAY_HEIGHT {
            for i in 0..OVERLAY_WIDTH {
                let idx = ((j * OVERLAY_WIDTH + i) * 4) as usize;

                //Border
                if i == 0 || i == OVERLAY_WIDTH - 1 || j == 0 || j == OVERLAY_HEIGHT - 1 {
                    buf[idx + 0] = border_r;
                    buf[idx + 1] = border_g;
                    buf[idx + 2] = border_b;
                    buf[idx + 3] = border_alpha;
                //Inside
                } else {
                    buf[idx + 0] = DEBUG_OVERLAY_BG_R;
                    buf[idx + 1] = DEBUG_OVERLAY_BG_G;
                    buf[idx + 2] = DEBUG_OVERLAY_BG_B;
                    buf[idx + 3] = DEBUG_OVERLAY_BG_A;
                }
            }
        }
        buf
    })
}

fn get_log_lock() -> MutexGuard<'static, VecDeque<String>> {
    LOG.get_or_init(|| Mutex::new(VecDeque::with_capacity(MAX_LOG_LINES)))
        .lock()
        .expect("Could not get log mutex")
}

pub fn add_to_debug_log_overlay(str: String) {
    let mut log = get_log_lock();

    if log.len() == MAX_LOG_LINES {
        log.pop_back();
    }
    log.push_front(str);
    if DEBUG_FEATURES.debug_overlay_enabled.load(Ordering::Relaxed) {
        drop(log);
        refresh_overlay_buffer();
    }
}

pub fn refresh_overlay_buffer() {
    let overlay = get_overlay();
    let overlay_ptr = overlay.as_ptr() as *mut u8;
    let log = get_log_lock();

    let mut y = 12.0;

    let max_x = OVERLAY_WIDTH as f32 - 5.0;

    //Flush the previous buffer
    clear_log_area(overlay_ptr);

    unsafe {
        for line in log.iter().rev() {
            let mut x = 2.0;
            for c in line.chars() {
                if x + FONT_SIZE >= max_x {
                    //Overflow
                    break;
                }
                x = draw_char(overlay_ptr, x, y, c);
            }
            y += FONT_SIZE + 2.0;
        }
    }
}

fn clear_log_area(buf: *mut u8) {
    for j in 0..OVERLAY_HEIGHT {
        for i in 0..OVERLAY_WIDTH {
            let idx = ((j * OVERLAY_WIDTH + i) * 4) as usize;

            //Border doesn't need to be cleared
            if i == 0 || i == OVERLAY_WIDTH - 1 || j == 0 || j == OVERLAY_HEIGHT - 1 {
            } else {
                unsafe {
                    *buf.add(idx + 0) = DEBUG_OVERLAY_BG_R;
                    *buf.add(idx + 1) = DEBUG_OVERLAY_BG_G;
                    *buf.add(idx + 2) = DEBUG_OVERLAY_BG_B;
                    *buf.add(idx + 3) = DEBUG_OVERLAY_BG_A;
                }
            }
        }
    }
}

//Draws a character. Very inefficient, but since it's only for debugging anyway we don't care
unsafe fn draw_char(buf: *mut u8, x: f32, y: f32, ch: char) -> f32 {
    let font = FONT.get_or_init(|| {
        let font = Font::from_bytes(FONT_DATA, FontSettings::default()).unwrap();
        font
    });

    let (metrics, bitmap) = font.rasterize(ch, FONT_SIZE);

    let glyph_x = x as usize + metrics.xmin as usize;
    let glyph_y = y as usize - metrics.height as usize - metrics.ymin as usize;

    unsafe {
        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let src = bitmap[row * metrics.width + col];

                if src == 0 {
                    continue;
                }
                let dst_x = glyph_x + col;
                let dst_y = glyph_y + row;

                let dst_index = ((dst_y as usize) * OVERLAY_WIDTH + (dst_x as usize)) * 4;
                *buf.add(dst_index) = src; // R
                *buf.add(dst_index + 1) = src; // G
                *buf.add(dst_index + 2) = src; // B
                *buf.add(dst_index + 3) = 255; // A
            }
        }
    }
    x as f32 + metrics.advance_width
}

//Draws the debug overlay in the buffer into the pixels array (which will be copied to GPU)
pub fn draw_debug_overlay(pixels: &mut [u8], width: u32) {
    let overlay = get_overlay();

    let frame_row_bytes = (width * 4) as usize;
    let overlay_row_bytes = (OVERLAY_WIDTH * 4) as usize;

    unsafe {
        let dst_ptr = pixels.as_mut_ptr();
        let src_ptr = overlay.as_ptr();

        for j in 0..OVERLAY_HEIGHT {
            let dst_row_ptr = dst_ptr.add(j * frame_row_bytes + 4);
            let src_row_ptr = src_ptr.add(j * overlay_row_bytes);
            std::ptr::copy_nonoverlapping(src_row_ptr, dst_row_ptr, OVERLAY_WIDTH * 4);
        }
    }
}

//When overlay is toggled off, these pixels have to be cleared.
pub fn clear_debug_overlay(pixels: &mut [u8], width: u32) {
    let frame_row_bytes = (width * 4) as usize;

    unsafe {
        let dst_ptr = pixels.as_mut_ptr();

        for j in 0..OVERLAY_HEIGHT {
            let dst_row_ptr = dst_ptr.add(j * frame_row_bytes + 4);
            for i in 0..OVERLAY_WIDTH {
                *dst_row_ptr.add(i * 4 + 3) = 0;
            }
        }
    }
}
