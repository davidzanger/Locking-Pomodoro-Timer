# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

### Changed

### Deprecated

### Removed

### Fixed

- Set the thread sleep to 10 ms to fix lagging issues.

### Security

## 1.2.1 - 2024-01-22

### Fixed

- When lagging issues after pressing `s` to skip a minute of a Pomodoro session.

## 1.2.0 - 2024-07-24

### Added

- Add option `interval_reminder_after_break` to specify the interval in minutes after which a reminder should be triggered after a break ends. This shall remind the user to either go back to work or to start a new Pomodoro session if already working. This option is only relevant if `auto_start_pomodoro` is `false`. The default value is `5`.
- Add option `event_reminder_after_break` to specify the event to be executed after the reminder interval after a break ends. The default value is `EndEvent::Sound`, using the default sound.

## 1.1.0 - 2024-05-22

### Added

- Add option to skip one minute of a Pomodoro session or a break by pressing `s`.

## 1.0.0 - 2024-05-22

### Added

- Create default option JSON on first start.
- Upload package to crates.io.

### Changed

- Update README.md with installation instructions.

## 0.5.0 - 2024-04-24

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

## 0.4.1 - 2024-04-22

### Changed

- Further refined print statement during additional duration and break.

### Fixed

- Could not press enter anymore to start a new Pomodoro session after a break. This was fixed by only using the receiver for key events and not `std::io::stdin`. This is only important when auto start pomodoro or break is disabled.

## 0.4.0 - 2024-04-18

### Changed

- Change the print statements to be more concise and informative like `Current: Pomodoro (1 min) | Upcoming: Additional 1 min | Pomodoros till long break: 4 (11 min)`.

## 0.3.1 - 2024-04-17

### Fixed

- Fixed send error when quitting a Pomodoro session or a break. The problem was that the receiver was dropped after the first timer had finished. This was fixed by invoking the receiver earlier in the code so that it is dropped later. 

## 0.3.0 - 2024-04-17

### Added

- Add functionality to pause, resume and stop a Pomodoro session as well as a break. The timer observes key events namely `p` to pause, `r` to resume and `q` to quit. These key events are also displayed in the console.

## 0.2.0 - 2024-04-11

### Added

- Add `autoStartPomodoro` as JSON key to automatically start a new Pomodoro session after a break ends.
- Add `intervalLongBreak` as JSON key to specify the interval in number of Pomodoro sessions after which a long break should be taken.
  
### Changed

- Use default values for options that are not provided in the JSON configuration file.