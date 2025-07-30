use crate::ui::{FRAME_BUFFER, OVERLAY_STATE};

//Prints a bunch of debug info.
pub fn dump_debug_data() {
    log::info!("------PRINTING DEBUG DATA------");

    {
        log::info!("Overlay State:");
        let state = OVERLAY_STATE.get().unwrap();
        let mut state_lock_opt = state.lock().unwrap();
        let state_lock = state_lock_opt.as_mut().unwrap();
        log::info!("  Width: {}", state_lock.width);
        log::info!("  Height: {}", state_lock.height);
        log::info!("Attempting to reset OVERLAY_STATE");
        *state_lock_opt = None;
    }

    {
        log::info!("Printing FRAME_BUFFER:");
        let frame = FRAME_BUFFER.get().unwrap();
        let mut frame_lock = frame.lock().unwrap();
        log::info!("  Width: {}", frame_lock.width);
        log::info!("  Height: {}", frame_lock.height);
        let mut r_count = 0;
        let mut g_count = 0;
        let mut b_count = 0;
        let mut a_count = 0;

        for chunk in frame_lock.pixels.chunks_exact(4) {
            let [r, g, b, a] = [chunk[0], chunk[1], chunk[2], chunk[3]];

            if r > 0 {
                r_count += 1;
            }
            if g > 0 {
                g_count += 1;
            }
            if b > 0 {
                b_count += 1;
            }
            if a > 0 {
                a_count += 1;
            }
        }
        log::info!(
            "  Total Rs: {} Gs: {} Bs: {} As: {}",
            r_count,
            g_count,
            b_count,
            a_count
        );

        let size = (frame_lock.width * frame_lock.height * 4) as usize;
        log::info!("Attempting to reset FRAME_BUFFER.");
        frame_lock.pixels = Vec::with_capacity(size);
        unsafe { frame_lock.pixels.set_len(size) };
    }

    log::info!("-------------------------------");
}
