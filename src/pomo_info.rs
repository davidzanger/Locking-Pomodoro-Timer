use std::time::Duration;

use crate::pomodoro_options::PomodoroOptions;

/// Represents the information related to a Pomodoro session.
pub(crate) struct PomoInfo {
    /// The number of Pomodoros remaining until a long break is triggered.
    pub(crate) pomodoros_till_long_break: i32,
    /// Indicates whether a long break is approaching.
    pub(crate) is_long_break_coming: bool,
    /// The duration of the next break.
    pub(crate) break_duration: Duration,
}

impl PomoInfo {
    /// Creates a new `PomoInfo` instance from the given `PomodoroOptions` and counter.
    ///
    /// # Arguments
    ///
    /// * `options` - The `PomodoroOptions` struct containing the Pomodoro settings. Necessary to calculate the break duration and the number of Pomodoros until the next long break.
    /// * `counter` - The current counter value indicating the number of completed Pomodoros.
    ///
    /// # Returns
    ///
    /// A new `PomoInfo` instance with the calculated values.
    pub(crate) fn from_options(options: &PomodoroOptions, counter: i32) -> Self {
        let pomodoros_till_long_break =
            options.interval_long_break - counter % options.interval_long_break;
        let is_long_break_coming =
            counter % options.interval_long_break == options.interval_long_break - 1;
        let break_duration: Duration;
        if is_long_break_coming {
            break_duration = Duration::from_secs((options.duration_long_break * 60) as u64)
        } else {
            break_duration = Duration::from_secs((options.duration_short_break * 60) as u64)
        };
        PomoInfo {
            pomodoros_till_long_break,
            is_long_break_coming,
            break_duration,
        }
    }
}
