use std::sync::LazyLock;
use std::time::Duration;

static DELAY: LazyLock<Duration> = Lazy::new(|| {
    Duration::from_millis(
        std::env::var("DEBUG_DELAY_MS")
            .unwrap_or_default()
            .parse::<u64>()
            .unwrap_or(200),
    )
});

pub fn delay() {
    if !DELAY.is_zero() {
        std::thread::sleep(*DELAY);
    }
}
