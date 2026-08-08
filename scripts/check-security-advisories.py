#!/usr/bin/env python3
"""Fail CI when RustSec ignores are undocumented or the review log is stale."""

from __future__ import annotations

import argparse
import re
import sys
from datetime import date, datetime, timezone
from pathlib import Path

RUSTSEC_PATTERN = re.compile(r"RUSTSEC-\d{4}-\d{4}")
REVIEW_PATTERN = re.compile(r"Last automated review:\s*\*\*(\d{4}-\d{2}-\d{2})\*\*")


def fail(message: str) -> None:
    print(f"security-advisory-check: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--max-age-days", type=int, default=45)
    args = parser.parse_args()

    if args.max_age_days < 1:
        fail("--max-age-days must be positive")

    audit_path = Path(".cargo/audit.toml")
    report_path = Path("SECURITY_ADVISORIES.md")

    audit = audit_path.read_text(encoding="utf-8")
    report = report_path.read_text(encoding="utf-8")

    ignored_ids = sorted(set(RUSTSEC_PATTERN.findall(audit)))
    undocumented = [advisory for advisory in ignored_ids if advisory not in report]
    if undocumented:
        fail("undocumented ignored RustSec IDs: " + ", ".join(undocumented))

    match = REVIEW_PATTERN.search(report)
    if not match:
        fail("SECURITY_ADVISORIES.md is missing 'Last automated review: **YYYY-MM-DD**'")

    try:
        reviewed = date.fromisoformat(match.group(1))
    except ValueError as exc:
        fail(f"invalid review date: {exc}")

    today = datetime.now(timezone.utc).date()
    age_days = (today - reviewed).days
    if age_days < 0:
        fail(f"review date {reviewed.isoformat()} is in the future")
    if age_days > args.max_age_days:
        fail(
            f"security advisory review is {age_days} days old; "
            f"maximum allowed age is {args.max_age_days} days"
        )

    print(
        "security-advisory-check: PASS "
        f"({len(ignored_ids)} ignored RustSec IDs documented; review age {age_days} days)"
    )


if __name__ == "__main__":
    main()
