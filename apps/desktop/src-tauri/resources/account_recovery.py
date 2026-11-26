"""Local Docker-owner recovery for the pinned HERMES Open WebUI SQLite backend.
No password is accepted on argv or printed. Existing users, chats and KBs remain.
"""
import asyncio
import contextlib
import io
import json
import os
from pathlib import Path
import sqlite3
import sys
import uuid


def recover(db_path, payload, hasher=None):
    db_path = Path(db_path)
    if not db_path.is_file():
        raise ValueError("Start the HERMES backend before account recovery.")
    action = payload.get("action")
    if action not in ("list", "reset"):
        raise ValueError("Unsupported recovery action.")
    connection = sqlite3.connect(str(db_path), timeout=20)
    connection.row_factory = sqlite3.Row
    try:
        if action == "list":
            return {"accounts": [dict(row) for row in connection.execute(
                'SELECT u.id, u.name, a.email FROM "user" u JOIN auth a ON a.id=u.id '
                "WHERE u.role='admin' AND a.active=1 ORDER BY a.email") ]}
        email = payload.get("email", "").strip().lower()
        password = payload.get("password", "")
        if "@" not in email or len(email) > 254 or not isinstance(password, str) or len(password) < 12 or len(password.encode("utf-8")) > 1024:
            raise ValueError("Choose a local administrator and a password of at least 12 characters.")
        account = connection.execute(
            'SELECT u.id FROM "user" u JOIN auth a ON a.id=u.id WHERE a.email=? AND u.role=\'admin\' AND a.active=1', (email,)
        ).fetchone()
        if account is None:
            raise ValueError("The selected local administrator was not found; no account changed.")
        if hasher is None:
            # Use the exact pinned backend's maintained password hasher (Argon2/bcrypt).
            from open_webui.utils.auth import get_password_hash, validate_password
            validate_password(password)
            hashed = asyncio.run(get_password_hash(password))
        else:
            hashed = hasher(password)
        backup_dir = db_path.parent / "hermes-recovery-backups"
        backup_dir.mkdir(mode=0o700, exist_ok=True)
        backup_path = backup_dir / ("webui-before-password-reset-" + uuid.uuid4().hex + ".db")
        descriptor = os.open(backup_path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
        os.close(descriptor)
        backup = sqlite3.connect(str(backup_path))
        try:
            connection.backup(backup)
        finally:
            backup.close()
        if hasher is None:
            from open_webui.models.auths import Auths
            async def apply_password():
                return await Auths.update_user_password_by_id(account["id"], hashed)
            if not asyncio.run(apply_password()):
                raise ValueError("Account changed during recovery.")
        else:
            # Local fixture tests exercise preservation without importing server packages.
            with connection:
                changed = connection.execute("UPDATE auth SET password=?, session_stamp=? WHERE id=? AND email=?", (hashed, uuid.uuid4().hex, account["id"], email)).rowcount
                if changed != 1:
                    raise ValueError("Account changed during recovery. Retry after checking the local account.")
        return {"reset": True, "backup": backup_path.name}
    finally:
        connection.close()


def main():
    try:
        raw = sys.stdin.buffer.read(16 * 1024 + 1)
        if len(raw) > 16 * 1024:
            raise ValueError("Recovery request is too large.")
        with contextlib.redirect_stdout(io.StringIO()):
            result = recover("/app/backend/data/webui.db", json.loads(raw))
        print(json.dumps(result))
    except Exception:
        # Upstream exception strings can contain inputs. Never echo them or credentials.
        print(json.dumps({"error": "Local account recovery failed. Check the selected administrator and backend availability. Existing books and chats were not removed."}))
        sys.exit(1)


if __name__ == "__main__":
    main()
