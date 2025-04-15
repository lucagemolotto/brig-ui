#!/bin/bash

STOP_NTP_SCRIPT="stop-NTP.sh"
IDRONAUT_SCRIPT="idronaut.py"
IDRONAUT2_SCRIPT="idronaut2.py"

if [ ! -x "$STOP_NTP_SCRIPT" ]; then
    echo "Error: $STOP_NTP_SCRIPT does not exist or is not executable."
    exit 1
fi

if [ ! -x "$IDRONAUT_SCRIPT" ]; then
    echo "Error: $IDRONAUT_SCRIPT does not exist or is not executable."
    exit 1
fi

if [ ! -x "$IDRONAUT2_SCRIPT" ]; then
    echo "Error: $IDRONAUT2_SCRIPT does not exist or is not executable."
    exit 1
fi

echo "Starting stop-NTP.sh..."
$STOP_NTP_SCRIPT || true

echo "Starting idronaut.py in a new terminal..."
if [ -z "$DISPLAY" ]; then
    python3 $IDRONAUT_SCRIPT &
elif command -v gnome-terminal &> /dev/null
then
    gnome-terminal -- bash -c "python3 $IDRONAUT_SCRIPT; exec bash"
elif command -v xterm &> /dev/null
then
    xterm -hold -e "python3 $IDRONAUT_SCRIPT" &
elif command -v konsole &> /dev/null
then
    konsole --noclose -e "python3 $IDRONAUT_SCRIPT" &
elif command -v xfce4-terminal &> /dev/null
then
    xfce4-terminal --hold -e "python3 $IDRONAUT_SCRIPT" &
elif command -v lxterminal &> /dev/null
then
    lxterminal -e "python3 $IDRONAUT_SCRIPT" &
else
    echo "Error: No suitable terminal emulator found."
    exit 1
fi

echo "Starting idronaut2.py in a new terminal..."
if [ -z "$DISPLAY" ]; then
    python3 $IDRONAUT2_SCRIPT &
elif command -v gnome-terminal &> /dev/null
then
    gnome-terminal -- bash -c "python3 $IDRONAUT2_SCRIPT; exec bash"
elif command -v xterm &> /dev/null
then
    xterm -hold -e "python3 $IDRONAUT2_SCRIPT" &
elif command -v konsole &> /dev/null
then
    konsole --noclose -e "python3 $IDRONAUT2_SCRIPT" &
elif command -v xfce4-terminal &> /dev/null
then
    xfce4-terminal --hold -e "python3 $IDRONAUT2_SCRIPT" &
elif command -v lxterminal &> /dev/null
then
    lxterminal -e "python3 $IDRONAUT2_SCRIPT" &
else
    echo "Error: No suitable terminal emulator found."
    exit 1
fi

echo "All scripts have been started."

function wait_ex {
    # https://stackoverflow.com/a/59723887
    # this waits for all jobs and returns the exit code of the last failing job
    ecode=0
    while true; do
        [ -z "$(jobs)" ] && break
        wait -n
        err="$?"
        [ "$err" != "0" ] && ecode="$err"
    done
    return $ecode
}

wait_ex



