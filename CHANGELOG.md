# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.0] - 2026-10-24

### Added

- Repeat Additional Pomodoros until you choose to take the due break. In `pomodoro_options.json` next to the executable, `repeatAdditionalPomodoro` and `askBeforeBreak` default to `true`, including for older settings files that omit them. After an Additional Pomodoro finishes, another interval starts while the break question remains open: press `1` to dismiss the question and keep the current interval running, or `2` (or `Q`) to start the scheduled short or long break. Set `repeatAdditionalPomodoro` to `false` to restore a single Additional Pomodoro before the break. Set `askBeforeBreak` to `false` to repeat without the question; `Q` still ends the current interval. Repetition is bypassed when the due break has zero duration, and `additionalDuration: 0` disables Additional Pomodoros.

- Run the executable with `--service` to integrate a desktop GUI over stdin/stdout JSON lines. Send one protocol-version-1 request per line, for example `{"protocolVersion":1,"command":{"type":"start"}}`. Commands include `pause`, `resume`, `skip` (with `seconds`), `cancel`, and `quit`. The service emits `ready`, `state`, phase-completion, waiting, error, and stopped events; diagnostic output goes to stderr. It uses the same settings file as the terminal timer. Rust integrations can instead call `locking_pomodoro_timer::run_embedded_service(commands, events)` with string channels using the same protocol, without launching a child process.

- GUI clients can display the upcoming phase from `nextStep` and track break questions through `breakDecisionPending` and `breakDecisionId` in state events. While a question is pending, send `{"protocolVersion":1,"command":{"type":"continueAdditional"}}` to dismiss it while work continues, or use `startBreak` to begin the due break. Send `nextPhase` to end the active phase without its sound or lock action; include the state's `phaseId` as `expectedPhaseId` to reject delayed actions against an expired phase, for example `{"protocolVersion":1,"command":{"type":"nextPhase","expectedPhaseId":3}}`. Protocol version 1 clients may omit this optional guard.

- Rust GUI bridges can read and save timer settings with `locking_pomodoro_timer::read_timer_settings_json()` and `save_timer_settings_json(settings_json)`. Read the settings, edit the returned JSON, and save it to persist durations, automatic starts, Additional Pomodoro preferences, end actions, and reminders in the shared settings file. Invalid settings are rejected before writing; omitted optional end-action and reminder fields preserve existing values. Reading settings preserves unavailable sound paths so a GUI can display them. Send the service's `updateSettings` command with a `settings` object containing the duration and automatic-start fields to apply timer changes to the running session; an active phase keeps its current duration. This command updates session settings and does not itself save them to disk.

### Fixed

- Terminal controls now handle key-press events and ignore key-release events, allowing pause, resume, skip, Enter, and quit controls to respond when keys are pressed.
- Setting `intervalLongBreak` to `0` disables long breaks without a division-by-zero failure. Negative intervals are rejected when reading or saving settings and by service settings updates.

## [1.2.1] - 2024-01-22

### Fixed

- When lagging issues after pressing `s` to skip a minute of a Pomodoro session.

## [1.2.0] - 2024-07-24

### Added

- Add option `interval_reminder_after_break` to specify the interval in minutes after which a reminder should be triggered after a break ends. This shall remind the user to either go back to work or to start a new Pomodoro session if already working. This option is only relevant if `auto_start_pomodoro` is `false`. The default value is `5`.
- Add option `event_reminder_after_break` to specify the event to be executed after the reminder interval after a break ends. The default value is `EndEvent::Sound`, using the default sound.

## [1.1.0] - 2024-05-22

### Added

- Add option to skip one minute of a Pomodoro session or a break by pressing `s`.

## [1.0.0] - 2024-05-22

### Added

- Create default option JSON on first start.
- Upload package to crates.io.

### Changed

- Update README.md with installation instructions.

## [0.5.0] - 2024-04-24

### Added

- Add keys for custom end events in the JSON options file. The new keys are `endEventPomodoro` and `endEventAdditionalPomodoro`. The current options are `sound` where a sound file must be specified (empty string results in default sound) and `lockScreen`. You can use them for example like:

    ```json
    {
        "endEventPomodoro": {
            "sound": {
                "filepathSound": ""
            }
        },
        "endEventAdditionalPomodoro": "lockScreen"
    }
    ```

- Add input validation for JSON configuration file.
- Icon for the application.

### Changed

- Change printing of options to JSON format.

### Fixed

- Verification of options. 0 minutes for breaks are now allowed.

## [0.4.1] - 2024-04-22

### Changed

- Further refined print statement during additional duration and break.

### Fixed

- Could not press enter anymore to start a new Pomodoro session after a break. This was fixed by only using the receiver for key events and not `std::io::stdin`. This is only important when auto start pomodoro or break is disabled.

## [0.4.0] - 2024-04-18

### Changed

- Change the print statements to be more concise and informative like `Current: Pomodoro (1 min) | Upcoming: Additional 1 min | Pomodoros till long break: 4 (11 min)`.

## [0.3.1] - 2024-04-17

### Fixed

- Fixed send error when quitting a Pomodoro session or a break. The problem was that the receiver was dropped after the first timer had finished. This was fixed by invoking the receiver earlier in the code so that it is dropped later. 

## [0.3.0] - 2024-04-17

### Added

- Add functionality to pause, resume and stop a Pomodoro session as well as a break. The timer observes key events namely `p` to pause, `r` to resume and `q` to quit. These key events are also displayed in the console.

## [0.2.0] - 2024-04-11

### Added

- Add `autoStartPomodoro` as JSON key to automatically start a new Pomodoro session after a break ends.
- Add `intervalLongBreak` as JSON key to specify the interval in number of Pomodoro sessions after which a long break should be taken.
  
### Changed

- Use default values for options that are not provided in the JSON configuration file.

[1.3.0]: https://github.com/davidzanger/Locking-Pomodoro-Timer/compare/v1.2.1...v1.3.0
[1.2.1]: https://github.com/davidzanger/Locking-Pomodoro-Timer/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/davidzanger/Locking-Pomodoro-Timer/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/davidzanger/Locking-Pomodoro-Timer/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/davidzanger/Locking-Pomodoro-Timer/compare/v0.5.0...v1.0.0
[0.5.0]: https://github.com/davidzanger/Locking-Pomodoro-Timer/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/davidzanger/Locking-Pomodoro-Timer/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/davidzanger/Locking-Pomodoro-Timer/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/davidzanger/Locking-Pomodoro-Timer/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/davidzanger/Locking-Pomodoro-Timer/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/davidzanger/Locking-Pomodoro-Timer/releases/tag/v0.2.0
