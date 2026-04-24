#!/usr/bin/env python3
"""Initialize the tiny_server database schema and seed data."""

import argparse
import sqlite3
import time
from pathlib import Path

DEFAULT_CONFIG = "config/database.yaml"
DEFAULT_ROOMS = ("lobby", "rust", "music", "gaming")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", default=DEFAULT_CONFIG)
    parser.add_argument("--no-seed", action="store_true")
    return parser.parse_args()


def load_database_config(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}

    for raw_line in path.read_text().splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue

        name, separator, value = line.partition(":")
        if not separator:
            raise SystemExit(f"Invalid database config line: {raw_line}")

        values[name.strip()] = value.strip().strip("\"'")

    return values


def sqlite_path(config_path: Path, config: dict[str, str]) -> Path:
    driver = config.get("driver", "")
    if driver != "sqlite":
        raise SystemExit(f"Unsupported database driver: {driver}")

    path = config.get("path", "")
    if not path:
        raise SystemExit("Database config must include path.")
    if path == ":memory:":
        raise SystemExit("Cannot initialize persistent data for an in-memory database.")

    db_path = Path(path)
    if not db_path.is_absolute():
        db_path = config_path.parent.parent / db_path

    return db_path


def initialize_schema(connection: sqlite3.Connection) -> None:
    connection.execute("PRAGMA foreign_keys = ON")
    connection.executescript(
        """
        CREATE TABLE IF NOT EXISTS app_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS chat_rooms (
            name TEXT PRIMARY KEY,
            created_at_secs INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS chat_messages (
            id INTEGER PRIMARY KEY,
            room TEXT NOT NULL,
            user TEXT NOT NULL,
            text TEXT NOT NULL,
            created_at_secs INTEGER NOT NULL,
            FOREIGN KEY (room) REFERENCES chat_rooms(name)
                ON UPDATE CASCADE
                ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_chat_messages_room_id
            ON chat_messages (room, id);

        CREATE INDEX IF NOT EXISTS idx_chat_messages_created_at
            ON chat_messages (created_at_secs);
        """
    )


def seed_data(connection: sqlite3.Connection) -> None:
    created_at_secs = int(time.time())
    connection.executemany(
        "INSERT OR IGNORE INTO chat_rooms (name, created_at_secs) VALUES (?, ?)",
        [(room, created_at_secs) for room in DEFAULT_ROOMS],
    )
    connection.execute(
        """
        INSERT OR IGNORE INTO chat_messages
            (id, room, user, text, created_at_secs)
        VALUES
            (0, 'lobby', 'system', 'Welcome to tiny_server chat', ?)
        """,
        (created_at_secs,),
    )
    connection.execute(
        "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('schema_version', '1')"
    )


def main() -> None:
    args = parse_args()
    config_path = Path(args.config)
    config = load_database_config(config_path)
    db_path = sqlite_path(config_path, config)

    db_path.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(db_path) as connection:
        initialize_schema(connection)
        if not args.no_seed:
            seed_data(connection)

    print(f"Initialized database: {db_path}")


if __name__ == "__main__":
    main()
