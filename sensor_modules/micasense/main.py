#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""General purpose script wrapper and crash recover."""

import subprocess
import sys
import time
from pathlib import Path

"""The script to launch. Must be a valid path."""
SCRIPT_PATH = Path("camera_capture/camera_capture.py")

"""
The path to Python executable.

Must be a valid path or command registered in PATH.
Must point to a Python 3.6+ executable.
"""
PYTHON_PATH = "python"


"""
The number of attempts to run the script before abort.

Between a run try and the following, there is a sleep time calculated with this formula:
'1.0 * attempt * attempt', where `attempt` is the current attempt number.
With the default value (25), this script will run for ~1:30 h.
"""
MAX_ATTEMPTS = 25


def main():
    if not PYTHON_PATH:
        print("Invalid Python path")
        exit(1)
        return

    if not SCRIPT_PATH.exists():
        print("Invalid script path '%s'" % SCRIPT_PATH, file=sys.stderr)
        exit(1)
        return

    running = True
    attempt = 0
    result = None

    while running and attempt < MAX_ATTEMPTS:
        try:
            result = subprocess.run(
                [PYTHON_PATH, str(SCRIPT_PATH.absolute())], cwd=str(SCRIPT_PATH.parent)
            )
            if result.returncode == 0:
                # Nothing else to do. Abort
                running = False
            else:
                print("Non-zero return code, try again (attempt %d)" % attempt)
                attempt += 1
                time.sleep(1.0 * attempt**2.0)

        except Exception as ex:
            print("Exception raised, try again (attempt %d)" % attempt, ex)
            attempt += 1
            time.sleep(1.0 * attempt**2.0)

        except BaseException:
            print("Execution halted, break")
            running = False

    if attempt >= MAX_ATTEMPTS:
        print("Failed to run the script '%s', abort" % SCRIPT_PATH)
        exit(1)
    else:
        print("The script runs successfully, quit.")
        exit(0)


if __name__ == "__main__":
    main()
