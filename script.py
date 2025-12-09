#!/usr/bin/env python3
"""
Simple ETL-style script used by evaluation challenges.

It intentionally fails when the required input file is missing so that
the evaluator can verify whether summaries surface the error clearly.
"""

import json
import sys
from pathlib import Path


INPUT_PATH = Path("evals/quality/fixtures/data/input.json")


def load_input() -> dict:
    if not INPUT_PATH.exists():
        raise FileNotFoundError(
            f"Required input file is missing: {INPUT_PATH}. "
            "Add a small JSON file with a top-level 'records' list."
        )

    with INPUT_PATH.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def validate_payload(payload: dict) -> None:
    if "records" not in payload:
        raise ValueError("Input JSON missing 'records' key.")
    if not isinstance(payload["records"], list):
        raise ValueError("The 'records' field must be a list of items to process.")
    if not payload["records"]:
        raise ValueError("No records provided in the input file.")


def main() -> None:
    try:
        payload = load_input()
        validate_payload(payload)
    except Exception as exc:  # noqa: BLE001 - bubble up for a clear traceback
        print("Processing failed before any records were handled.", file=sys.stderr)
        raise


if __name__ == "__main__":
    main()

