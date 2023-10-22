use once_cell::unsync::Lazy;
use std::time::Duration;

pub fn delay() {
    let delay: Lazy<Duration> = Lazy::new(|| {
        Duration::from_millis(
            std::env::var("DEBUG_DELAY_MS")
                .unwrap_or_default()
                .parse::<u64>()
                .unwrap_or(0),
        )
    });

    if !delay.is_zero() {
        std::thread::sleep(*delay);
    }
}
