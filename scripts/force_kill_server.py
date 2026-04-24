#!/usr/bin/env python3
"""Force kill tiny_server by pid, port or process pattern."""

import argparse
import os
from pathlib import Path
import signal
import subprocess
import time

DEFAULT_PATTERN = "tiny_server"
DEFAULT_PORT = 7878


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pid", type=int, action="append", default=[])
    parser.add_argument("--port", type=int, action="append")
    parser.add_argument("--pattern", default=DEFAULT_PATTERN)
    parser.add_argument("--no-default-port", action="store_true")
    return parser.parse_args()


def collect_pids(args: argparse.Namespace) -> list[int]:
    pids = set(args.pid)
    pids.update(find_pids_by_pattern(args.pattern))

    ports = list(args.port or [])
    if not args.no_default_port and DEFAULT_PORT not in ports:
        ports.append(DEFAULT_PORT)

    for port in ports:
        pids.update(find_pids_by_port(port))

    return sorted(pid for pid in pids if pid > 0 and pid != os.getpid())


def find_pids_by_pattern(pattern: str) -> set[int]:
    try:
        output = subprocess.check_output(["pgrep", "-f", pattern], text=True)
    except subprocess.CalledProcessError:
        return set()

    pids = {int(line) for line in output.splitlines() if line.strip().isdigit()}
    return {pid for pid in pids if matches_target_process(pid, pattern)}


def find_pids_by_port(port: int) -> set[int]:
    try:
        output = subprocess.check_output(
            ["lsof", "-t", f"-iTCP:{port}", "-sTCP:LISTEN"], text=True
        )
    except (FileNotFoundError, subprocess.CalledProcessError):
        return find_pids_by_port_from_proc(port)

    return {int(line) for line in output.splitlines() if line.strip().isdigit()}


def find_pids_by_port_from_proc(port: int) -> set[int]:
    listening_inodes = socket_inodes_for_port(port)
    if not listening_inodes:
        return set()

    pids = set()
    for proc_entry in Path("/proc").iterdir():
        if not proc_entry.name.isdigit():
            continue

        fd_dir = proc_entry / "fd"
        try:
            fd_entries = list(fd_dir.iterdir())
        except OSError:
            continue

        for fd_entry in fd_entries:
            try:
                target = os.readlink(fd_entry)
            except OSError:
                continue

            if target.startswith("socket:[") and target[8:-1] in listening_inodes:
                pids.add(int(proc_entry.name))
                break

    return pids


def socket_inodes_for_port(port: int) -> set[str]:
    inodes = set()
    port_hex = f"{port:04X}"

    for path in (Path("/proc/net/tcp"), Path("/proc/net/tcp6")):
        try:
            lines = path.read_text().splitlines()[1:]
        except OSError:
            continue

        for line in lines:
            parts = line.split()
            if len(parts) < 10:
                continue

            local_address = parts[1]
            state = parts[3]
            inode = parts[9]
            _, local_port = local_address.rsplit(":", 1)

            if local_port.upper() == port_hex and state == "0A":
                inodes.add(inode)

    return inodes


def is_running(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    return True


def matches_target_process(pid: int, pattern: str) -> bool:
    cmdline_path = Path(f"/proc/{pid}/cmdline")
    exe_path = Path(f"/proc/{pid}/exe")

    try:
        raw_cmdline = cmdline_path.read_bytes()
        cmdline_parts = [
            part.decode(errors="ignore") for part in raw_cmdline.split(b"\x00") if part
        ]
    except OSError:
        cmdline_parts = []

    try:
        exe_name = exe_path.resolve().name
    except OSError:
        exe_name = ""

    if exe_name == pattern:
        return True

    for part in cmdline_parts:
        normalized = part.replace("\\", "/")
        if normalized.endswith(f"/target/debug/{pattern}"):
            return True
        if normalized.endswith(f"/target/release/{pattern}"):
            return True

    return False


def wait_for_exit(pid: int, timeout_secs: float) -> bool:
    deadline = time.time() + timeout_secs
    while time.time() < deadline:
        if not is_running(pid):
            return True
        time.sleep(0.05)
    return not is_running(pid)


def kill_pid(pid: int) -> None:
    if not is_running(pid):
        return

    for sig in (signal.SIGINT, signal.SIGTERM):
        try:
            os.kill(pid, sig)
        except ProcessLookupError:
            return
        if wait_for_exit(pid, timeout_secs=1.0):
            return

    try:
        os.kill(pid, signal.SIGKILL)
    except ProcessLookupError:
        return

    wait_for_exit(pid, timeout_secs=1.0)


def main() -> None:
    args = parse_args()
    pids = collect_pids(args)

    if not pids:
        print("No matching tiny_server process found.")
        return

    for pid in pids:
        kill_pid(pid)

    survivors = [pid for pid in pids if is_running(pid)]
    if survivors:
        raise SystemExit(f"Failed to kill pids: {survivors}")

    print(f"Killed tiny_server pids: {pids}")


if __name__ == "__main__":
    main()
