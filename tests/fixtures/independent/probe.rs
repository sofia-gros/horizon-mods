fn fixture_probe(right: bool) {
    static SEEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !SEEN.swap(true, std::sync::atomic::Ordering::Relaxed) {
        eprintln!("independent fixture station-click hook executed");
    }
    std::hint::black_box(right);
}
