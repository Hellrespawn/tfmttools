use std::time::Duration;

use once_cell::sync::Lazy;

static DELAY: Lazy<Duration> = Lazy::new(|| {
    Duration::from_millis(
        std::env::var("DEBUG_DELAY_MS")
            .unwrap_or_default()
            .parse::<u64>()
            .unwrap_or(0),
    )
});

pub fn delay() {
    if !DELAY.is_zero() {
        std::thread::sleep(*DELAY);
    }
}
