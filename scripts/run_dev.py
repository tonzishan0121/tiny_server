#!/usr/bin/env python3
"""Start tiny_server in development mode."""

import os
import signal
import subprocess
from pathlib import Path

SHUTDOWN_TIMEOUT_SECS = 3.0


def stop_process_tree(process: subprocess.Popen[bytes]) -> int:
    if process.poll() is not None:
        return process.returncode

    pgid = os.getpgid(process.pid)
    os.killpg(pgid, signal.SIGINT)

    try:
        return process.wait(timeout=SHUTDOWN_TIMEOUT_SECS)
    except subprocess.TimeoutExpired:
        os.killpg(pgid, signal.SIGTERM)

    try:
        return process.wait(timeout=1.0)
    except subprocess.TimeoutExpired:
        os.killpg(pgid, signal.SIGKILL)
        return process.wait(timeout=1.0)


def main() -> None:
    project_dir = Path(__file__).resolve().parent.parent
    subprocess.run(["cargo", "build"], cwd=project_dir, check=True)

    server = subprocess.Popen(
        [str(project_dir / "target/debug/tiny_server")],
        cwd=project_dir,
        start_new_session=True,
    )

    try:
        raise SystemExit(server.wait())
    except KeyboardInterrupt:
        raise SystemExit(stop_process_tree(server))


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as err:
        raise SystemExit(err.returncode) from err
