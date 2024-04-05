use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

pub(crate) struct Timer {
    start_time: Instant,
    duration: Duration,
    elapsed_time: Arc<AtomicU32>,
    paused: Arc<AtomicBool>,
    should_terminate: Arc<AtomicBool>,
}

impl Timer {
    fn new(duration: Duration) -> Self {
        Timer {
            start_time: Instant::now(),
            duration,
            elapsed_time: Arc::new(AtomicU32::new(0)),
            paused: Arc::new(AtomicBool::new(false)),
            should_terminate: Arc::new(AtomicBool::new(false)),
        }
    }

    fn start(&self) {
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
        });
    }

    fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    fn stop(&self) {
        self.should_terminate.store(true, Ordering::Relaxed);
    }

    fn get_elapsed_time(&self) -> Duration {
        let elapsed_time = self.elapsed_time.load(Ordering::Relaxed);
        Duration::from_secs(elapsed_time as u64)
    }

    fn reset(&self) {
        self.elapsed_time.store(0, Ordering::Relaxed);
    }
    // TODO: What about clean up? Use the exit_timer to stop the thread.
}
