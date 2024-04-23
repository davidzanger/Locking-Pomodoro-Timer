use crate::pomodoro_options::PomodoroOptions;

use crate::pomo_info::PomoInfo;


struct MessageData<'a> {
    current: &'a str,
    current_duration: i32,
    upcoming: &'a str,
    upcoming_duration: i32,
    pomodoros_till_long_break: i32,
    minutes_till_long_break: i32,
}

impl MessageData<'_> {
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

pub(crate) fn generate_print_message_before_additional_break(
    pomo_info: &PomoInfo,
    options: &PomodoroOptions,
) -> String {
    let minutes_till_long_break = pomo_info.pomodoros_till_long_break * options.additional_duration
        + (pomo_info.pomodoros_till_long_break - 1)
            * (options.duration_short_break + options.duration_pomodoro);
    let (current, current_duration, upcoming, upcoming_duration); 
    current = "Additional";
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
