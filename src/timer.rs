use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use log::trace;
pub(crate) struct Timer {
    duration: Duration,
    elapsed_time: Arc<AtomicU32>,
    paused: Arc<AtomicBool>,
    should_terminate: Arc<AtomicBool>,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        Timer {
            duration,
            elapsed_time: Arc::new(AtomicU32::new(0)),
            paused: Arc::new(AtomicBool::new(false)),
            should_terminate: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self) {
        let elapsed_time_storage = self.elapsed_time.clone();
        let duration = self.duration;
        let mut time_buffer = 0.0;
        let paused = self.paused.clone();
        let should_terminate = self.should_terminate.clone();
        thread::spawn(move || {
            let delta = 0.5;
            while elapsed_time_storage.load(Ordering::Relaxed) < duration.as_secs() as u32{
                if should_terminate.load(Ordering::Relaxed) {
                    break
                }
                thread::sleep(Duration::from_secs_f64(delta));
                if paused.load(Ordering::Relaxed) {
                    continue
                }
                time_buffer += delta;
                if time_buffer >= 1.0 {
                    let time_to_add = time_buffer as u32;
                    time_buffer -= time_to_add as f64;
                    elapsed_time_storage.fetch_add(time_to_add,Ordering::Relaxed);
                }
            }
            trace!("Timer thread terminated.");
        });
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.should_terminate.store(true, Ordering::Relaxed);
    }

    pub fn get_elapsed_time(&self) -> Duration {
        let elapsed_time = self.elapsed_time.load(Ordering::Relaxed);
        Duration::from_secs(elapsed_time as u64)
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.stop();
        trace!("Timer dropped.");
    }
}