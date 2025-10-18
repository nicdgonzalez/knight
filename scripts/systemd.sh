#!/usr/bin/env bash

[ ! -z "${TRACE+x}" ] && set -x

SERVICE_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/knight.service"

SERVICE_CONTENT="\
[Unit]
Description=Automatically switch between light and dark theme

[Service]
ExecStart=/usr/bin/env knight start

[Install]
WantedBy=default.target"

main() {
    mkdir --parents "$(dirname "$SERVICE_FILE")"
    echo "$SERVICE_CONTENT" > "$SERVICE_FILE"

    # Make systemd aware of our changes.
    systemctl --user daemon-reload

    # Start the service.
    systemctl --user start "$(basename "$SERVICE_FILE")"

    # Allow the service to persist after reboots.
    systemctl --user enable "$(basename "$SERVICE_FILE")"

    # Check if the service is running.
    systemctl --user status "$(basename "$SERVICE_FILE")"
}

main "$@"
