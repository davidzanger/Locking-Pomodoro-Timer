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

#[cfg(test)]
mod tests {
    use super::PomoInfo;
    use crate::pomodoro_options::PomodoroOptions;
    use std::time::Duration;

    #[test]
    fn calculates_short_break_before_long_break_interval() {
        let info = PomoInfo::from_options(&PomodoroOptions::default(), 0);

        assert_eq!(info.pomodoros_till_long_break, 4);
        assert!(!info.is_long_break_coming);
        assert_eq!(info.break_duration, Duration::from_secs(5 * 60));
    }

    #[test]
    fn selects_long_break_at_end_of_interval() {
        let info = PomoInfo::from_options(&PomodoroOptions::default(), 3);

        assert_eq!(info.pomodoros_till_long_break, 1);
        assert!(info.is_long_break_coming);
        assert_eq!(info.break_duration, Duration::from_secs(15 * 60));
    }

    #[test]
    fn wraps_counter_after_long_break_interval() {
        let info = PomoInfo::from_options(&PomodoroOptions::default(), 4);

        assert_eq!(info.pomodoros_till_long_break, 4);
        assert!(!info.is_long_break_coming);
    }
}
