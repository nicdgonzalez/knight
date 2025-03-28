# Knight

**Knight** allows you to switch system themes automatically based on the time
of day for the GNOME desktop environment on Linux.

## ✨ Features

- Automatically toggle between light and dark theme.
- Determines sunrise and sunset times based on your location.
- Configurable through a dedicated configuration file.
- Supports temporarily pausing automatic switching.

## 📦 Installation

Install this project using Rust's package manager, cargo:

```bash
cargo install --git https://github.com/nicdgonzalez/knight
```

### ⏳ Scheduling

In order to have the program run itself throughout the day, you need to use
some sort of scheduler.

Here are some example scripts to help quickstart the process:

<details>
    <summary>systemd</summary>

To check if your system uses systemd:

```bash
command -v systemctl > /dev/null && echo "true" || echo "false"
```

Copy and paste the following into a file named `knight.sh`:

```bash
#!/usr/bin/bash

set -eo pipefail
[ ! -z "${TRACE+x}" ] && set -x

SYSTEMD_CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

SERVICE_FILE="\
[Unit]
Description=Changes the system between light and dark theme automatically

[Service]
ExecStart=$HOME/.cargo/bin/knight
Type=exec"

TIMER_FILE="\
[Unit]
Description=A timer for Knight

[Timer]
# Runs once every thirty minutes.
OnCalendar=*-*-* *:*/30:00
# Run the task if it was missed (e.g., because the system was asleep).
Persistent=true
WakeSystem=false

[Install]
WantedBy=timers.target"

main() {
    # Write the configurations into their respective files.
    mkdir --parents "$SYSTEMD_CONFIG_HOME" \
        && echo "$SERVICE_FILE" > "$SYSTEMD_CONFIG_HOME/knight.service" \
        && echo "$TIMER_FILE" > "$SYSTEMD_CONFIG_HOME/knight.timer"

    # Make systemd aware of our changes.
    systemctl --user daemon-reload

    # Start the service and timer.
    systemctl --user start knight.service
    systemctl --user start knight.timer

    # Allow the service and timer to persist after reboots.
    systemctl --user enable knight.service
    systemctl --user enable knight.timer

    # Check that the units are running properly:
    systemctl --user status knight.service
    systemctl --user status knight.timer
}

# The main entry point to the script.
main "$@"
```

Now run the script using bash:

```bash
bash "$PWD/knight.sh"
```

</details>

<details>
    <summary>cronie</summary>

Make sure to install `crontab` via your favorite package manager.

```bash
# Fedora
sudo dnf install cronie
```

Copy and paste the following into a file named `knight.sh`:

```bash
#!/usr/bin/bash

set -eo pipefail
[ ! -z "${TRACE+x}" ] && set -x

SYSTEMD_CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"

main() {
    # This job will execute the program every thirty minutes.
    local job="*/30 * * * * $HOME/.cargo/bin/knight"

    # If the job already exists, return.
    if grep --fixed-strings "$job" -- <(crontab -l) > /dev/null; then
        echo >&2 "Job already exists!"
        return 0
    fi

    # Add the new job to the end of the existing cron file.
    crontab <(echo -e "$(crontab -l)\n$job")
}

# The main entry point to the script.
main "$@"
```

Now run the script using bash:

```bash
bash "$PWD/knight.sh"
```

</details>

## 🛠️ Configuration

Knight allows you to tailor its behaviors to better fit your needs;
an optional configuration file can be created at
`$XDG_CONFIG_HOME/knight/Knight.toml`. For additional information
about the different options, see [Knight.toml](./Knight.toml).

> [!NOTE]
> The `$XDG_CONFIG_HOME` environment variable typically points to a directory
> where user-specific configuration files are stored. If it is not set,
> the default location is usually `$HOME/.config`.
