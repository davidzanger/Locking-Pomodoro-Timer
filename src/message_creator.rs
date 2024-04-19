use crate::pomodoro_options::PomodoroOptions;

use crate::pomo_info::PomoInfo;

pub(crate) fn generate_print_message_before_pomodoro(
    pomo_info: &PomoInfo,
    options: &PomodoroOptions,
) -> String {
    let minutes_till_long_break = pomo_info.pomodoros_till_long_break
        * (options.duration_pomodoro + options.additional_duration)
        + (pomo_info.pomodoros_till_long_break - 1) * options.duration_short_break;
    let mut print_message = format!("Current: Pomodoro ({:.0} min)", options.duration_pomodoro);
    if options.additional_duration != 0 {
        print_message += &format!(
            " | Upcoming: Additional {} min",
            options.additional_duration
        );
    } else if !pomo_info.break_duration.is_zero() && pomo_info.is_long_break_coming {
        print_message += &format!(
            " | Upcoming: Long break ({:.0} min)",
            options.duration_long_break
        );
    } else if !pomo_info.break_duration.is_zero() && !pomo_info.is_long_break_coming {
        print_message += &format!(
            " | Upcoming: Short break ({:.0} min)",
            options.duration_short_break
        );
    } else if pomo_info.break_duration.is_zero() {
        print_message += &format!(
            " | Upcoming: Pomodoro ({:.0} min)",
            options.duration_pomodoro
        );
    } else {
        panic!("Invalid state. This line should not be reached.");
    }
    print_message += &format!(
        " | Pomodoros till long break: {} ({} min)",
        pomo_info.pomodoros_till_long_break, minutes_till_long_break
    );
    print_message
}

pub(crate) fn generate_print_message_before_additional_break(
    pomo_info: &PomoInfo,
    options: &PomodoroOptions,
) -> String {
    let mut print_message = format!(
        "Current: Additional ({:.0} min).",
        options.additional_duration
    );
    if !pomo_info.break_duration.is_zero() && pomo_info.is_long_break_coming {
        print_message += &format!(
            " | Upcoming: Long break ({:.0} min)",
            options.duration_long_break
        );
    } else if !pomo_info.break_duration.is_zero() && !pomo_info.is_long_break_coming {
        print_message += &format!(
            " | Upcoming: Short break ({:.0} min)",
            options.duration_short_break
        );
    } else if pomo_info.break_duration.is_zero() {
        print_message += &format!(
            " | Upcoming: Pomodoro ({:.0} min)",
            options.duration_pomodoro
        );
    } else {
        panic!("Invalid state. This line should not be reached.");
    }
    let minutes_till_long_break = pomo_info.pomodoros_till_long_break * options.additional_duration
        + (pomo_info.pomodoros_till_long_break - 1)
            * (options.duration_short_break + options.duration_pomodoro);
    print_message += &format!(
        " | Pomodoros till long break: {} ({} min)",
        pomo_info.pomodoros_till_long_break - 1,
        minutes_till_long_break
    );
    print_message
}

pub(crate) fn generate_print_message_before_break(
    pomo_info: &PomoInfo,
    options: &PomodoroOptions,
) -> String {
    let mut print_message = format!(
        "Current: {} break ({:.0} min).",
        if pomo_info.is_long_break_coming {
            "Long"
        } else {
            "Short"
        },
        pomo_info.break_duration.as_secs() / 60
    );
    print_message += &format!(
        " | Upcoming: Pomodoro ({:.0} min)",
        options.duration_pomodoro
    );
    let minutes_till_long_break = (pomo_info.pomodoros_till_long_break - 1)
        * (options.duration_pomodoro + options.duration_short_break + options.additional_duration);
    print_message += &format!(
        " | Pomodoros till long break: {} ({} min)",
        pomo_info.pomodoros_till_long_break - 1,
        minutes_till_long_break
    );
    print_message
}
