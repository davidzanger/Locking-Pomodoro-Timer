use std::time::Duration;

use crate::pomodoro_options::PomodoroOptions;

pub(crate) struct PomoInfo {
    pub(crate) pomodoros_till_long_break: i32,
    pub(crate) is_long_break_coming: bool,
    pub(crate) break_duration: Duration,
}
impl PomoInfo {
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
