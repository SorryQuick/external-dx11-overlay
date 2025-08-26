use std::{
    collections::HashMap,
    sync::{
        OnceLock,
        atomic::Ordering,
        mpsc::{Sender, channel},
    },
    time::{Duration, Instant},
};

use super::{
    DEBUG_FEATURES,
    debug_overlay::{OVERLAY_MODE, overlay_mode, refresh_overlay_buffer},
};

//Sender
static STATISTIC_SENDER: OnceLock<Sender<(u32, u32)>> = OnceLock::new();

//Stores the stats that will be rendered on the overlay
pub mod debug_stat {
    pub const FRAME_TIME_CUSTOM: u32 = 0;
    pub const FRAME_TIME_TOTAL: u32 = 1;
    pub const FRAME_TIME_DIFF: u32 = 2;
}

//Small thread that listens to and counts certain statistics for debugging purposes.
//They are displayed on statistic mode of the debug overlay.
pub fn start_statistics_server() {
    let (tx, rx) = channel::<(u32, u32)>();
    STATISTIC_SENDER.set(tx).ok();

    std::thread::spawn(move || {
        let mut stats = get_stats_map();

        let mut last_refresh = Instant::now();
        let refresh_interval = Duration::from_millis(500);

        while let Ok(msg) = rx.recv() {
            match msg.0 {
                _ => {
                    stats.insert(msg.0, msg.1);
                    /*log::debug!(
                        "Custom time: {}",
                        stats.get(&debug_stat::FRAME_TIME_CUSTOM).unwrap()
                    );
                    log::debug!(
                        "Total time: {}",
                        stats.get(&debug_stat::FRAME_TIME_TOTAL).unwrap()
                    );
                    log::debug!(
                        "Diff time: {}",
                        stats.get(&debug_stat::FRAME_TIME_DIFF).unwrap()
                    );*/
                }
            }
            if last_refresh.elapsed() >= refresh_interval
                && DEBUG_FEATURES.debug_overlay_enabled.load(Ordering::Relaxed)
                && OVERLAY_MODE.load(Ordering::Relaxed) == overlay_mode::STAT_MODE
            {
                refresh_overlay_buffer(Some(&stats));
                last_refresh = Instant::now();
            }
        }
    });
}

//Just to make sure the stats are indeed valid.
fn get_stats_map() -> HashMap<u32, u32> {
    let mut stats = HashMap::with_capacity(100);
    for i in 0..100 {
        stats.insert(i, 0);
    }
    return stats;
}

//Sends a simple statistic to the listener.
pub fn send_statistic(key: u32, value: u32) {
    STATISTIC_SENDER
        .get()
        .expect("Could not get the statistic sender")
        .send((key, value))
        .ok();
}
