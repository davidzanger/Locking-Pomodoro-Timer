# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## 0.3.0 - 2024-04-17

### Added

- Add functionality to pause, resume and stop a Pomodoro session as well as a break. The timer observes key events namely `p` to pause, `r` to resume and `q` to quit. These key events are also displayed in the console.

## 0.2.0 - 2024-04-11

### Added

- Add `autoStartPomodoro` as JSON key to automatically start a new Pomodoro session after a break ends.
- Add `intervalLongBreak` as JSON key to specify the interval in number of Pomodoro sessions after which a long break should be taken.
  
### Changed

- Use default values for options that are not provided in the JSON configuration file.