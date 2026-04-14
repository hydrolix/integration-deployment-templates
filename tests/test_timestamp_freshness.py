"""Tests for timestamp freshness shifting in Phase 2d (transform_organizer)."""

import json
import os
import sys
import time
import types
from datetime import datetime, timezone
from unittest.mock import patch
import calendar

import pytest

# Stub out utils.file_utils before importing configurator modules
_utils_pkg = types.ModuleType("utils")
_utils_pkg.file_utils = types.ModuleType("utils.file_utils")


def _stub_read_json(path, *a, **kw):
    with open(path) as f:
        return json.load(f)


_utils_pkg.file_utils.read_json = _stub_read_json
_utils_pkg.file_utils.write_json = lambda *a, **kw: None
sys.modules.setdefault("utils", _utils_pkg)
sys.modules.setdefault("utils.file_utils", _utils_pkg.file_utils)

from scripts.configurator.transform_organizer import (
    _shift_stale_timestamps,
    _extract_sample_data,
    _resolve_sample_key,
    _STALENESS_THRESHOLD_SECS,
)
from scripts.configurator.config import BundleConfig, BundleState, TransformInfo


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _now_epoch():
    return int(time.time())


def _first_of_month_epoch():
    now_utc = datetime.now(timezone.utc)
    first = now_utc.replace(day=1, hour=0, minute=0, second=0, microsecond=0)
    return int(calendar.timegm(first.timetuple()))


def _make_transform_data(primary_ts, extra_epoch_ts=None, extra_non_epoch=None):
    """Build a transform dict with output_columns and sample_data."""
    columns = [
        {
            "name": "timestamp",
            "datatype": {
                "type": "epoch",
                "primary": True,
                "format": "s",
                "resolution": "ms",
            },
        },
        {
            "name": "bytes",
            "datatype": {"type": "uint64"},
        },
    ]
    sample = {"timestamp": primary_ts, "bytes": 4096}

    if extra_epoch_ts is not None:
        columns.append(
            {
                "name": "ts_sec_idx",
                "datatype": {"type": "epoch", "format": "s"},
            }
        )
        sample["ts_sec_idx"] = extra_epoch_ts

    if extra_non_epoch is not None:
        sample["status_code"] = extra_non_epoch

    return {
        "name": "test_transform",
        "settings": {
            "output_columns": columns,
            "sample_data": sample,
        },
    }


def _make_config(tmp_path, verbose=False, dry_run=False):
    bundle_dir = tmp_path / "aws" / "test_bundle"
    bundle_dir.mkdir(parents=True, exist_ok=True)
    return BundleConfig(
        bundle_dir=str(bundle_dir),
        table_name="logs",
        data_category="cdn",
        verbose=verbose,
        dry_run=dry_run,
    )


def _make_tinfo(tmp_path):
    bundle_dir = tmp_path / "aws" / "test_bundle"
    transformations_dir = bundle_dir / "transformations"
    transformations_dir.mkdir(parents=True, exist_ok=True)
    return TransformInfo(
        original_path=str(transformations_dir / "transform.json"),
        final_path=str(transformations_dir / "transform.json"),
        final_dir=str(transformations_dir),
        sample_data_path=str(transformations_dir / "sample_data.json"),
    )


# ---------------------------------------------------------------------------
# Unit tests for _shift_stale_timestamps
# ---------------------------------------------------------------------------


class TestShiftStaleTimestamps:
    """Unit tests for the _shift_stale_timestamps helper."""

    def test_fresh_data_not_modified(self, tmp_path):
        """Timestamps within 6 months should not be shifted."""
        fresh_ts = _now_epoch() - (30 * 86400)  # 30 days ago
        data = _make_transform_data(fresh_ts)
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path)
        tinfo = _make_tinfo(tmp_path)

        _shift_stale_timestamps(sample, data, tinfo, config)

        assert sample["timestamp"] == fresh_ts
        assert sample["bytes"] == 4096

    def test_stale_data_shifted_to_first_of_month(self, tmp_path):
        """Timestamps older than 6 months should shift to 1st of current month."""
        stale_ts = _now_epoch() - (365 * 86400)  # 1 year ago
        data = _make_transform_data(stale_ts)
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path)
        tinfo = _make_tinfo(tmp_path)

        _shift_stale_timestamps(sample, data, tinfo, config)

        expected_target = _first_of_month_epoch()
        assert sample["timestamp"] == expected_target

    def test_delta_applied_to_all_epoch_columns(self, tmp_path):
        """All epoch-typed columns should be shifted by the same delta."""
        stale_ts = _now_epoch() - (365 * 86400)
        extra_ts = stale_ts - 3600  # 1 hour before primary
        data = _make_transform_data(stale_ts, extra_epoch_ts=extra_ts)
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path)
        tinfo = _make_tinfo(tmp_path)

        _shift_stale_timestamps(sample, data, tinfo, config)

        target = _first_of_month_epoch()
        delta = target - stale_ts
        assert sample["timestamp"] == target
        assert sample["ts_sec_idx"] == extra_ts + delta

    def test_mixed_format_secondary_columns(self, tmp_path):
        """Secondary epoch columns with different formats get their own per-format delta."""
        stale_secs = _now_epoch() - (365 * 86400)
        secondary_ms = (stale_secs - 3600) * 1000  # 1 hour before primary, in ms
        data = {
            "settings": {
                "output_columns": [
                    {
                        "name": "timestamp",
                        "datatype": {"type": "epoch", "primary": True, "format": "s"},
                    },
                    {
                        "name": "event_time_ms",
                        "datatype": {"type": "epoch", "format": "ms"},
                    },
                    {"name": "bytes", "datatype": {"type": "uint64"}},
                ],
                "sample_data": {
                    "timestamp": stale_secs,
                    "event_time_ms": secondary_ms,
                    "bytes": 4096,
                },
            }
        }
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path)
        tinfo = _make_tinfo(tmp_path)

        _shift_stale_timestamps(sample, data, tinfo, config)

        target = _first_of_month_epoch()
        delta_secs = target - stale_secs
        # Primary (seconds format) shifted by delta in seconds
        assert sample["timestamp"] == target
        # Secondary (ms format) shifted by delta in milliseconds
        assert sample["event_time_ms"] == secondary_ms + (delta_secs * 1000)
        # Non-epoch untouched
        assert sample["bytes"] == 4096

    def test_non_epoch_columns_untouched(self, tmp_path):
        """Non-epoch columns should not be modified."""
        stale_ts = _now_epoch() - (365 * 86400)
        data = _make_transform_data(stale_ts, extra_non_epoch=200)
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path)
        tinfo = _make_tinfo(tmp_path)

        _shift_stale_timestamps(sample, data, tinfo, config)

        assert sample["bytes"] == 4096
        assert sample["status_code"] == 200

    def test_no_primary_column_skips(self, tmp_path):
        """If no primary epoch column exists, skip without error."""
        data = {
            "settings": {
                "output_columns": [
                    {"name": "bytes", "datatype": {"type": "uint64"}},
                ],
                "sample_data": {"bytes": 4096},
            }
        }
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path, verbose=True)
        tinfo = _make_tinfo(tmp_path)

        _shift_stale_timestamps(sample, data, tinfo, config)

        assert sample["bytes"] == 4096

    def test_no_output_columns_skips(self, tmp_path):
        """If output_columns is missing entirely, skip without error."""
        data = {"settings": {"sample_data": {"timestamp": 1000000000}}}
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path)
        tinfo = _make_tinfo(tmp_path)

        _shift_stale_timestamps(sample, data, tinfo, config)

        assert sample["timestamp"] == 1000000000

    def test_primary_value_not_numeric_skips(self, tmp_path):
        """If primary timestamp value is a string, skip without error."""
        data = {
            "settings": {
                "output_columns": [
                    {
                        "name": "timestamp",
                        "datatype": {"type": "epoch", "primary": True},
                    }
                ],
                "sample_data": {"timestamp": "not-a-number"},
            }
        }
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path, verbose=True)
        tinfo = _make_tinfo(tmp_path)

        _shift_stale_timestamps(sample, data, tinfo, config)

        assert sample["timestamp"] == "not-a-number"

    def test_threshold_boundary_not_shifted(self, tmp_path):
        """Data exactly at the 183-day threshold should not be shifted."""
        fixed_now = 1800000000  # fixed reference time
        boundary_ts = fixed_now - _STALENESS_THRESHOLD_SECS
        data = _make_transform_data(boundary_ts)
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path)
        tinfo = _make_tinfo(tmp_path)

        with patch("scripts.configurator.transform_organizer.time") as mock_time:
            mock_time.time.return_value = fixed_now
            _shift_stale_timestamps(sample, data, tinfo, config)

        assert sample["timestamp"] == boundary_ts

    def test_threshold_boundary_plus_one_shifted(self, tmp_path):
        """Data 1 second past the threshold should be shifted."""
        fixed_now = 1800000000
        # 2027-01-15 approx — target should be 2027-01-01 00:00 UTC = 1798761600
        fixed_target = 1798761600
        stale_ts = fixed_now - _STALENESS_THRESHOLD_SECS - 1
        data = _make_transform_data(stale_ts)
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path)
        tinfo = _make_tinfo(tmp_path)

        fixed_dt = datetime(2027, 1, 15, 12, 0, 0, tzinfo=timezone.utc)
        with patch("scripts.configurator.transform_organizer.time") as mock_time, \
             patch("scripts.configurator.transform_organizer.datetime") as mock_dt:
            mock_time.time.return_value = fixed_now
            mock_dt.now.return_value = fixed_dt
            mock_dt.side_effect = lambda *a, **kw: datetime(*a, **kw)
            _shift_stale_timestamps(sample, data, tinfo, config)

        assert sample["timestamp"] == fixed_target

    def test_millisecond_format_shifted_correctly(self, tmp_path):
        """Epoch-ms timestamps should be normalized to seconds for comparison and shifted in ms."""
        stale_secs = _now_epoch() - (365 * 86400)  # 1 year ago in seconds
        stale_ms = stale_secs * 1000  # convert to milliseconds
        data = {
            "settings": {
                "output_columns": [
                    {
                        "name": "timestamp",
                        "datatype": {
                            "type": "epoch",
                            "primary": True,
                            "format": "ms",
                        },
                    },
                ],
                "sample_data": {"timestamp": stale_ms},
            }
        }
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path)
        tinfo = _make_tinfo(tmp_path)

        _shift_stale_timestamps(sample, data, tinfo, config)

        # Should be shifted to first-of-month in milliseconds
        expected_ms = _first_of_month_epoch() * 1000
        assert sample["timestamp"] == expected_ms

    def test_millisecond_format_fresh_not_shifted(self, tmp_path):
        """Fresh ms timestamps should not be shifted."""
        fresh_secs = _now_epoch() - (30 * 86400)  # 30 days ago
        fresh_ms = fresh_secs * 1000
        data = {
            "settings": {
                "output_columns": [
                    {
                        "name": "timestamp",
                        "datatype": {
                            "type": "epoch",
                            "primary": True,
                            "format": "ms",
                        },
                    },
                ],
                "sample_data": {"timestamp": fresh_ms},
            }
        }
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path)
        tinfo = _make_tinfo(tmp_path)

        _shift_stale_timestamps(sample, data, tinfo, config)

        assert sample["timestamp"] == fresh_ms


# ---------------------------------------------------------------------------
# Integration test: _extract_sample_data with stale timestamps
# ---------------------------------------------------------------------------


class TestExtractSampleDataIntegration:
    """Integration test: Phase 2d extraction + timestamp shifting."""

    def test_extract_shifts_stale_and_writes(self, tmp_path):
        """Full Phase 2d: extract sample_data from transform with stale ts, verify in-memory data."""
        config = _make_config(tmp_path)
        state = BundleState()
        tinfo = _make_tinfo(tmp_path)

        stale_ts = _now_epoch() - (365 * 86400)
        transform_data = _make_transform_data(stale_ts)

        # Write transform to disk so _extract_sample_data can read paths
        transform_path = tinfo.final_path
        os.makedirs(os.path.dirname(transform_path), exist_ok=True)
        with open(transform_path, "w") as f:
            json.dump(transform_data, f)

        ok = _extract_sample_data(transform_data, tinfo, config, state)

        assert ok is True
        assert state.errors == []
        assert tinfo.has_sample_data is True

        # Check the in-memory sample_data (modified in-place before writing)
        sample = transform_data["settings"]["sample_data"]
        expected_target = _first_of_month_epoch()
        assert sample["timestamp"] == expected_target
        assert sample["bytes"] == 4096

    def test_extract_fresh_data_unchanged(self, tmp_path):
        """Fresh data should pass through extraction without modification."""
        config = _make_config(tmp_path)
        state = BundleState()
        tinfo = _make_tinfo(tmp_path)

        fresh_ts = _now_epoch() - (30 * 86400)
        transform_data = _make_transform_data(fresh_ts)

        transform_path = tinfo.final_path
        os.makedirs(os.path.dirname(transform_path), exist_ok=True)
        with open(transform_path, "w") as f:
            json.dump(transform_data, f)

        ok = _extract_sample_data(transform_data, tinfo, config, state)

        assert ok is True
        sample = transform_data["settings"]["sample_data"]
        assert sample["timestamp"] == fresh_ts


# ---------------------------------------------------------------------------
# Tests for _resolve_sample_key (LOTC-1412)
# ---------------------------------------------------------------------------


class TestResolveSampleKey:
    """Tests for JSON pointer resolution when output name != raw key."""

    def test_output_name_matches_sample_key(self):
        """When output name exists in sample_data, return it directly."""
        col = {"name": "timestamp", "datatype": {"type": "epoch", "source": None}}
        sample = {"timestamp": 1700000000}
        assert _resolve_sample_key(col, sample) == "timestamp"

    def test_json_pointer_fallback(self):
        """When output name is missing, resolve from from_json_pointers."""
        col = {
            "name": "timestamp",
            "datatype": {
                "type": "epoch",
                "source": {"from_json_pointers": ["/reqTimeSec"]},
            },
        }
        sample = {"reqTimeSec": 1700000000}
        assert _resolve_sample_key(col, sample) == "reqTimeSec"

    def test_no_match_returns_none(self):
        """When neither output name nor pointer matches, return None."""
        col = {
            "name": "timestamp",
            "datatype": {
                "type": "epoch",
                "source": {"from_json_pointers": ["/missing_field"]},
            },
        }
        sample = {"reqTimeSec": 1700000000}
        assert _resolve_sample_key(col, sample) is None

    def test_source_without_json_pointers_returns_none(self):
        """When source has from_input_field but no from_json_pointers, return None."""
        col = {
            "name": "computed_ts",
            "datatype": {"type": "epoch", "source": {"from_input_field": "sql_transform"}},
        }
        sample = {"reqTimeSec": 1700000000}
        assert _resolve_sample_key(col, sample) is None

    def test_nested_pointer_skipped(self):
        """Multi-segment JSON pointers (e.g. /avail/fillRate) cannot be resolved to flat keys."""
        col = {
            "name": "fill_rate",
            "datatype": {
                "type": "epoch",
                "source": {"from_json_pointers": ["/avail/fillRate"]},
            },
        }
        sample = {"avail/fillRate": 1700000000}  # even if flat key exists, nested pointer skipped
        assert _resolve_sample_key(col, sample) is None

    def test_missing_name_returns_none(self):
        """Column without a name field returns None gracefully."""
        col = {"datatype": {"type": "epoch"}}
        sample = {"timestamp": 1700000000}
        assert _resolve_sample_key(col, sample) is None

    def test_output_name_preferred_over_pointer(self):
        """If output name exists in sample, use it even if pointer also matches."""
        col = {
            "name": "timestamp",
            "datatype": {
                "type": "epoch",
                "source": {"from_json_pointers": ["/reqTimeSec"]},
            },
        }
        sample = {"timestamp": 1700000000, "reqTimeSec": 1699999999}
        assert _resolve_sample_key(col, sample) == "timestamp"


# ---------------------------------------------------------------------------
# Tests for shifted timestamps with JSON pointer resolution (LOTC-1412)
# ---------------------------------------------------------------------------


class TestShiftWithJsonPointer:
    """End-to-end shift tests where output column name != raw JSON key."""

    def test_akamai_style_reqtimesec_shifted(self, tmp_path):
        """Akamai-style transform: output 'timestamp' from raw 'reqTimeSec'."""
        stale_ts = _now_epoch() - (365 * 86400)
        data = {
            "settings": {
                "output_columns": [
                    {
                        "name": "timestamp",
                        "datatype": {
                            "type": "epoch",
                            "primary": True,
                            "format": "s",
                            "source": {"from_json_pointers": ["/reqTimeSec"]},
                        },
                    },
                    {"name": "bytes", "datatype": {"type": "uint64"}},
                ],
                "sample_data": {"reqTimeSec": stale_ts, "bytes": 4096},
            }
        }
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path)
        tinfo = _make_tinfo(tmp_path)

        _shift_stale_timestamps(sample, data, tinfo, config)

        expected_target = _first_of_month_epoch()
        assert sample["reqTimeSec"] == expected_target
        assert sample["bytes"] == 4096

    def test_fresh_akamai_style_not_shifted(self, tmp_path):
        """Fresh Akamai-style timestamps should not be shifted."""
        fresh_ts = _now_epoch() - (30 * 86400)
        data = {
            "settings": {
                "output_columns": [
                    {
                        "name": "timestamp",
                        "datatype": {
                            "type": "epoch",
                            "primary": True,
                            "format": "s",
                            "source": {"from_json_pointers": ["/reqTimeSec"]},
                        },
                    },
                ],
                "sample_data": {"reqTimeSec": fresh_ts},
            }
        }
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path)
        tinfo = _make_tinfo(tmp_path)

        _shift_stale_timestamps(sample, data, tinfo, config)

        assert sample["reqTimeSec"] == fresh_ts

    def test_unresolvable_primary_skips(self, tmp_path):
        """If primary epoch column can't be resolved to a sample key, skip."""
        stale_ts = _now_epoch() - (365 * 86400)
        data = {
            "settings": {
                "output_columns": [
                    {
                        "name": "timestamp",
                        "datatype": {
                            "type": "epoch",
                            "primary": True,
                            "format": "s",
                            "source": {"from_input_field": "sql_transform"},
                        },
                    },
                ],
                "sample_data": {"some_other_field": stale_ts},
            }
        }
        sample = data["settings"]["sample_data"]
        config = _make_config(tmp_path, verbose=True)
        tinfo = _make_tinfo(tmp_path)

        _shift_stale_timestamps(sample, data, tinfo, config)

        assert sample["some_other_field"] == stale_ts
