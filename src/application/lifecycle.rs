//! One shutdown signal shared by desktop actions, the server and its owner.
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;
static ACTIVE: AtomicBool = AtomicBool::new(false);
static STOPPING: AtomicBool = AtomicBool::new(false);
static RESTART: AtomicBool = AtomicBool::new(false);
static CHANGED: Notify = Notify::const_new();
pub fn activate() {
    ACTIVE.store(true, Ordering::SeqCst);
}
pub fn active() -> bool {
    ACTIVE.load(Ordering::SeqCst)
}
pub fn stopping() -> bool {
    STOPPING.load(Ordering::SeqCst)
}
pub fn request_shutdown() {
    STOPPING.store(true, Ordering::SeqCst);
    CHANGED.notify_waiters();
}
pub fn request_restart() {
    RESTART.store(true, Ordering::SeqCst);
    request_shutdown();
}
pub fn restarting() -> bool {
    RESTART.load(Ordering::SeqCst)
}
pub async fn shutdown_requested() {
    loop {
        let notified = CHANGED.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if stopping() {
            return;
        }
        notified.await;
    }
}
