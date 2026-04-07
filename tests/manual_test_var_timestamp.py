#!/usr/bin/env python3
"""Manual test: inject ${VAR_TIMESTAMP} into a temp copy of mcdn and run the dashboard fixer.

Usage:
    python3 tests/manual_test_var_timestamp.py
"""

import json
import os
import shutil
import sys
import tempfile

# Add repo root and scripts/ to path for imports
repo_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, repo_dir)
sys.path.insert(0, os.path.join(repo_dir, "scripts"))

from scripts.configurator.config import BundleConfig, BundleState, DashboardInfo, TransformInfo
from scripts.configurator.dashboard_fixer import _fix_template_variables, _get_primary_timestamp_column

REPO_ROOT = os.path.join(os.path.dirname(__file__), "..")
MCDN_BUNDLE = os.path.join(REPO_ROOT, "trafficpeak", "mcdn", "1.0.0")


def main():
    with tempfile.TemporaryDirectory() as tmp:
        # Copy the mcdn transform so we can read the primary column
        src_transform = os.path.join(MCDN_BUNDLE, "transformations", "transform.json")
        if not os.path.isfile(src_transform):
            # Try default_shared fallback
            src_transform = os.path.join(
                REPO_ROOT, "trafficpeak", "default_shared", "transformations", "transform.json"
            )

        dst_transform = os.path.join(tmp, "transformations", "transform.json")
        os.makedirs(os.path.dirname(dst_transform), exist_ok=True)
        shutil.copy2(src_transform, dst_transform)

        # Read the transform to show what primary column we expect
        with open(dst_transform) as f:
            transform_data = json.load(f)
        for col in transform_data["settings"]["output_columns"]:
            if col.get("datatype", {}).get("primary") is True:
                expected_col = col["name"]
                break
        else:
            print("ERROR: No primary column found in transform!")
            return 1

        print(f"Transform primary column: {expected_col}")
        print()

        # Build a fake dashboard with ${VAR_TIMESTAMP}
        dashboard = {
            "templating": {
                "list": [
                    {
                        "name": "raw_table",
                        "type": "constant",
                        "query": "${VAR_SIEM}",
                        "current": {"selected": False, "text": "${VAR_SIEM}", "value": "${VAR_SIEM}"},
                        "options": [{"selected": False, "text": "${VAR_SIEM}", "value": "${VAR_SIEM}"}],
                    },
                    {
                        "name": "timestamp",
                        "type": "constant",
                        "query": "${VAR_TIMESTAMP}",
                        "current": {"selected": False, "text": "${VAR_TIMESTAMP}", "value": "${VAR_TIMESTAMP}"},
                        "options": [{"selected": False, "text": "${VAR_TIMESTAMP}", "value": "${VAR_TIMESTAMP}"}],
                    },
                    {
                        "name": "interval",
                        "type": "interval",
                        "query": "1m,5m,15m,30m,1h,6h,12h,1d,7d",
                    },
                ]
            }
        }

        print("BEFORE:")
        for var in dashboard["templating"]["list"]:
            print(f"  {var['name']:15s} type={var['type']:10s} query={var.get('query', '')}")
        print()

        # Set up config and state
        config = BundleConfig(
            bundle_dir=tmp,
            table_name="test_table",
            data_category="web",
            source_name="trafficpeak",
            bundle_name="mcdn",
            verbose=True,
        )

        tinfo = TransformInfo(original_path=dst_transform)
        tinfo.final_path = dst_transform

        state = BundleState()
        state.transforms = [tinfo]

        dinfo = DashboardInfo(path="/fake/dashboard.json", is_primary=True)

        # Run the fixer
        _fix_template_variables(dashboard, dinfo, {}, config, state)

        print("AFTER:")
        for var in dashboard["templating"]["list"]:
            print(f"  {var['name']:15s} type={var['type']:10s} query={var.get('query', '')}")
        print()

        # Check timestamp was resolved
        ts_var = next(v for v in dashboard["templating"]["list"] if v["name"] == "timestamp")
        if ts_var["query"] == expected_col:
            print(f"SUCCESS: timestamp resolved to '{expected_col}'")
            return 0
        else:
            print(f"FAIL: timestamp query is '{ts_var['query']}', expected '{expected_col}'")
            return 1


if __name__ == "__main__":
    sys.exit(main())
