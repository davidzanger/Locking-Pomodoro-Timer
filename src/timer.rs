use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use log::trace;

/// Represents a timer that counts the elapsed time.
/// 
/// The timer runs in a separate thread and counts the elapsed time in seconds.
pub(crate) struct Timer {
    duration: Duration,
    elapsed_time: Arc<AtomicU32>,
    paused: Arc<AtomicBool>,
    should_terminate: Arc<AtomicBool>,
}

impl Timer {
    /// Creates a new Timer instance with the specified duration.
    ///
    /// # Arguments
    ///
    /// * `duration` - The duration after which the timer should stop.
    pub fn new(duration: Duration) -> Self {
        Timer {
            duration,
            elapsed_time: Arc::new(AtomicU32::new(0)),
            paused: Arc::new(AtomicBool::new(false)),
            should_terminate: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Starts the timer in a separate thread.
    /// 
    /// The thread sleeps for 0.5 seconds and increments the elapsed time when more than 1 second has passed.
    /// The timer stops when the elapsed time reaches the specified duration.
    /// The timer can be paused and resumed using the `pause` and `resume` methods.
    /// The timer can be stopped using the `stop` method.
    /// The thread terminates when the timer stops.
    pub fn start(&self) {
        let elapsed_time_storage = self.elapsed_time.clone();
        let duration = self.duration;
        let mut time_buffer = 0.0;
        let paused = self.paused.clone();
        let should_terminate = self.should_terminate.clone();
        thread::spawn(move || {
            let delta = 0.5;
            while elapsed_time_storage.load(Ordering::Relaxed) < duration.as_secs() as u32 {
                if should_terminate.load(Ordering::Relaxed) {
                    break;
                }
                thread::sleep(Duration::from_secs_f64(delta));
                if paused.load(Ordering::Relaxed) {
                    continue;
                }
                time_buffer += delta;
                if time_buffer >= 1.0 {
                    let time_to_add = time_buffer as u32;
                    time_buffer -= time_to_add as f64;
                    elapsed_time_storage.fetch_add(time_to_add, Ordering::Relaxed);
                }
            }
            trace!("Timer thread terminated.");
        });
    }

    /// Pauses the timer.
    pub fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    /// Resumes the timer.
    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    /// Checks if the timer is currently paused.
    ///
    /// Returns `true` if the timer is paused, `false` otherwise.
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    /// Stops the timer. The timer thread will terminate.
    pub fn stop(&self) {
        self.should_terminate.store(true, Ordering::Relaxed);
    }

    /// Gets the elapsed time of the timer.
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