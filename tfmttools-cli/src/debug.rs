use std::sync::LazyLock;
use std::time::Duration;

const DEBUG_DELAY_MS_ENV: &str = "DEBUG_DELAY_MS";

static DELAY: LazyLock<Duration> = LazyLock::new(|| {
    Duration::from_millis(
        std::env::var(DEBUG_DELAY_MS_ENV)
            .unwrap_or_default()
            .parse::<u64>()
            .unwrap_or_default(),
    )
});

pub fn delay() {
    if !DELAY.is_zero() {
        std::thread::sleep(*DELAY);
    }
}
