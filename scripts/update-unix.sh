#!/bin/sh
# Arguments: embedded installer, result file, release version, installation kind.
# This outer shell reports the child installer's exit status without interfering
# with its own cleanup traps. It stays visible so the user can read the outcome.
[ "$#" -eq 4 ] || exit 2
[ ! -f "$2.cancel" ] || exit 1
printf started > "$2.started" || exit 1
trap 'printf 130 > "$2"; exit 130' HUP INT TERM
NUMPAD_VERSION="$3" NUMPAD_INSTALL="$4" sh "$1"
result=$?
printf '%s' "$result" > "$2"
printf '\nPress Enter to close this window.'
read -r _
exit "$result"
