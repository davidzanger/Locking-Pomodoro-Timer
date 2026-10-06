/// This module contains functions and structs related to creating terminal print messages for the Pomodoro app.
use crate::pomodoro_options::PomodoroOptions;

use crate::pomo_info::PomoInfo;


/// Represents the data needed to create a terminal print message for the Pomodoro app.
struct MessageData<'a> {
    current: &'a str,
    current_duration: i32,
    upcoming: &'a str,
    upcoming_duration: i32,
    pomodoros_till_long_break: i32,
    minutes_till_long_break: i32,
}

impl MessageData<'_> {
    /// Generates a formatted print message with the current, upcoming, and pomodoro till long break information.
    ///
    /// # Returns
    ///
    /// A string containing the formatted print message.
    fn generate_print_message(&self) -> String {
        format!(
            "Current: {} ({:.0} min) | Upcoming: {} ({:.0} min) | Pomodoros till long break: {} ({:.0} min)",
            self.current,
            self.current_duration,
            self.upcoming,
            self.upcoming_duration,
            self.pomodoros_till_long_break,
            self.minutes_till_long_break
        )
    }
}

/// Generates a print message to be displayed before starting a pomodoro.
///
/// # Arguments
///
/// * `pomo_info` - The Pomodoro information.
/// * `options` - The Pomodoro options.
///
/// # Returns
///
/// A string containing the formatted print message.
///
/// # Panics
///
/// Panics if an invalid state is encountered.
pub(crate) fn generate_print_message_before_pomodoro(
    pomo_info: &PomoInfo,
    options: &PomodoroOptions,
) -> String {
    let minutes_till_long_break = pomo_info.pomodoros_till_long_break
        * (options.duration_pomodoro + options.additional_duration)
        + (pomo_info.pomodoros_till_long_break - 1) * options.duration_short_break;
        let (current, current_duration, upcoming, upcoming_duration); 
        current = "Pomodoro";
        current_duration = options.duration_pomodoro;
        if options.additional_duration != 0 {
            upcoming = "Additional Pomodoro";
            upcoming_duration = options.additional_duration;
    } else if !pomo_info.break_duration.is_zero() && pomo_info.is_long_break_coming {
        upcoming = "Long break";
        upcoming_duration = options.duration_long_break;
    } else if !pomo_info.break_duration.is_zero() && !pomo_info.is_long_break_coming {
        upcoming = "Short break";
        upcoming_duration = options.duration_short_break;
    } else if pomo_info.break_duration.is_zero() {
        upcoming = "Pomodoro";
        upcoming_duration = options.duration_pomodoro;
    } else {
        panic!("Invalid state. This line should not be reached.");
    }
    let message_data = MessageData {
        current,
        current_duration,
        upcoming,
        upcoming_duration,
        pomodoros_till_long_break: pomo_info.pomodoros_till_long_break,
        minutes_till_long_break,
    };
    let print_message = message_data.generate_print_message();
    print_message
}

/// Generates a print message to be displayed before starting an additional break.
///
/// # Arguments
///
/// * `pomo_info` - The Pomodoro information.
/// * `options` - The Pomodoro options.
///
/// # Returns
///
/// A string containing the formatted print message.
///
/// # Panics
///
/// Panics if an invalid state is encountered.
pub(crate) fn generate_print_message_before_additional_break(
    pomo_info: &PomoInfo,
    options: &PomodoroOptions,
) -> String {
    let minutes_till_long_break = pomo_info.pomodoros_till_long_break * options.additional_duration
        + (pomo_info.pomodoros_till_long_break - 1)
            * (options.duration_short_break + options.duration_pomodoro);
    let (current, current_duration, upcoming, upcoming_duration); 
    current = "Additional Pomodoro";
    current_duration = options.additional_duration;
    if !pomo_info.break_duration.is_zero() && pomo_info.is_long_break_coming {
        upcoming = "Long break";
        upcoming_duration = options.duration_long_break;
    } else if !pomo_info.break_duration.is_zero() && !pomo_info.is_long_break_coming {
        upcoming = "Short break";
        upcoming_duration = options.duration_short_break;
    } else if pomo_info.break_duration.is_zero() {
        upcoming = "Pomodoro";
        upcoming_duration = options.duration_pomodoro;
    } else {
        panic!("Invalid state. This line should not be reached.");
    }
    let message_data = MessageData {
        current,
        current_duration,
        upcoming,
        upcoming_duration,
        pomodoros_till_long_break: pomo_info.pomodoros_till_long_break,
        minutes_till_long_break,
    };
    let print_message = message_data.generate_print_message();
    print_message
}

/// Generates a print message to be displayed before starting a break.
///
/// # Arguments
///
/// * `pomo_info` - The Pomodoro information.
/// * `options` - The Pomodoro options.
///
/// # Returns
///
/// A string containing the formatted print message.
///
/// # Panics
///
/// Panics if an invalid state is encountered.
pub(crate) fn generate_print_message_before_break(
    pomo_info: &PomoInfo,
    options: &PomodoroOptions,
) -> String {
    let (current, current_duration, upcoming, upcoming_duration); 
    current = if pomo_info.is_long_break_coming {
        "Long break"
    } else {
        "Short break"
    };
    current_duration = (pomo_info.break_duration.as_secs() / 60) as i32;
    upcoming = "Pomodoro";
    upcoming_duration = options.duration_pomodoro;
    let minutes_till_long_break = (pomo_info.pomodoros_till_long_break - 1)
        * (options.duration_pomodoro + options.duration_short_break + options.additional_duration);
    let message_data = MessageData {
        current,
        current_duration,
        upcoming,
        upcoming_duration,
        pomodoros_till_long_break: pomo_info.pomodoros_till_long_break - 1,
        minutes_till_long_break,
    };
    let print_message = message_data.generate_print_message();
    print_message
}

#[cfg(test)]
mod tests {
    use super::{
        generate_print_message_before_additional_break,
        generate_print_message_before_break, generate_print_message_before_pomodoro,
    };
    use crate::pomo_info::PomoInfo;
    use crate::pomodoro_options::PomodoroOptions;
    use std::time::Duration;

    fn info(is_long_break_coming: bool, break_minutes: u64, until_long_break: i32) -> PomoInfo {
        PomoInfo {
            pomodoros_till_long_break: until_long_break,
            is_long_break_coming,
            break_duration: Duration::from_secs(break_minutes * 60),
        }
    }

    #[test]
    fn pomodoro_message_announces_additional_session_when_configured() {
        let options = PomodoroOptions::default();
        let message = generate_print_message_before_pomodoro(&info(false, 5, 4), &options);

        assert!(message.contains("Current: Pomodoro (25 min)"));
        assert!(message.contains("Upcoming: Additional Pomodoro (5 min)"));
        assert!(message.contains("Pomodoros till long break: 4 (135 min)"));
    }

    #[test]
    fn pomodoro_message_announces_short_long_or_next_pomodoro() {
        let mut options = PomodoroOptions::default();
        options.additional_duration = 0;

        assert!(generate_print_message_before_pomodoro(&info(false, 5, 2), &options)
            .contains("Upcoming: Short break (5 min)"));
        assert!(generate_print_message_before_pomodoro(&info(true, 15, 1), &options)
            .contains("Upcoming: Long break (15 min)"));
        assert!(generate_print_message_before_pomodoro(&info(false, 0, 1), &options)
            .contains("Upcoming: Pomodoro (25 min)"));
    }

    #[test]
    fn additional_session_message_announces_upcoming_break_or_pomodoro() {
        let options = PomodoroOptions::default();

        assert!(generate_print_message_before_additional_break(&info(true, 15, 1), &options)
            .contains("Upcoming: Long break (15 min)"));
        assert!(generate_print_message_before_additional_break(&info(false, 5, 2), &options)
            .contains("Upcoming: Short break (5 min)"));
        assert!(generate_print_message_before_additional_break(&info(false, 0, 1), &options)
            .contains("Upcoming: Pomodoro (25 min)"));
    }

    #[test]
    fn break_message_announces_next_pomodoro_and_remaining_interval() {
        let options = PomodoroOptions::default();

        let short = generate_print_message_before_break(&info(false, 5, 3), &options);
        assert!(short.contains("Current: Short break (5 min)"));
        assert!(short.contains("Upcoming: Pomodoro (25 min)"));
        assert!(short.contains("Pomodoros till long break: 2 (70 min)"));

        let long = generate_print_message_before_break(&info(true, 15, 1), &options);
        assert!(long.contains("Current: Long break (15 min)"));
        assert!(long.contains("Pomodoros till long break: 0 (0 min)"));
    }
}
