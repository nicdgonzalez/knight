# Knight

**Knight** automatically switches between light and dark theme based on local
sunrise and sunset times (for the GNOME desktop environment on Linux).

## Installation

Install directly from Git using cargo:

```bash
cargo install --git https://github.com/nicdgonzalez/knight
```

## Requirements

**Tested with**:

- gsettings 2.84.4
- cargo 1.87.0

Other recent versions may also work.

## Usage

A minimal example to show how it works.

First, change your system theme to the opposite of the current time. You can
use the Graphical User Interface (GUI), or the following command:

```bash
gsettings set org.gnome.desktop.interface color-scheme <default | prefer-dark>
```

Then, run the program:

```bash
knight start
```

The program is designed to run forever; you aren't meant to run it directly. To
quit, press <kbd>Ctrl</kbd>+<kbd>C</kbd>.

### Intended usage

To automatically run `knight start` when the system boots, run:

> [!TIP]\
> Piping directly to bash can be risky because it prevents you from reading the
> code that will run on your system. Always inspect scripts before executing
> them to ensure they are safe.
>
> You can inspect the script used below [here](./scripts/systemd.sh).

```bash
curl -SsL https://raw.githubusercontent.com/nicdgonzalez/knight/refs/heads/main/scripts/systemd.sh | bash
```

Or create a new file at `$HOME/.config/systemd/user/knight.service` with the
following contents:

```ini
[Unit]
Description=Automatically switch between light and dark theme

[Service]
ExecStart=/usr/bin/env knight start

[Install]
WantedBy=default.target
```

Then run the following commands:

```bash
# Make systemd aware of our changes
systemctl --user daemon-reload

# Start the service
systemctl --user start knight.service

# Persist after reboots
systemctl --user enable knight.service

# Check if the service is running
systemctl --user status knight.service
```

### Stopping the program

To stop the program until the next reboot:

```bash
systemctl --user stop knight.service
```

To stop the program indefinitely:

```bash
systemctl --user disable knight.service
systemctl --user stop knight.service
```

To uninstall:

```bash
cargo uninstall knight
rm -rf "${XDG_CACHE_HOME:-$HOME/.cache}/knight" "${XDG_CONFIG_HOME:-$HOME/.config}/knight" "${XDG_STATE_HOME:-$HOME/.local/share}/knight"
```

## Roadmap

- [ ] Disable until next sunrise/sunset when the user manually changes the
  theme.

## Attributions

This project is only possible thanks to the following services:

- [Free IP API]: Used to get your location.
- [SunriseSunset.io]: Used to get your location's sunrise/sunset times.

[free ip api]: https://freeipapi.com/
[sunrisesunset.io]: https://sunrisesunset.io/
