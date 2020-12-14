#[cfg(unix)]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    pub fn abort_handle() -> impl Fn() -> bool {
        let abort = Arc::new(AtomicBool::new(false));
        let abort_signal = abort.clone();
        simple_signal::set_handler(
            &[simple_signal::Signal::Term, simple_signal::Signal::Int],
            move |_| {
                abort_signal.store(true, Ordering::Relaxed);
            },
        );
        move || abort.load(Ordering::Relaxed)
    }
}

#[cfg(not(unix))]
mod imp {
    pub fn abort_handle() -> impl Fn() -> bool {
        || false
    }
}

pub use imp::abort_handle;
