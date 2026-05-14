"""Tests for dashboard_fixer module — self-referencing VAR_* template variable fixes."""

import json
import sys
import types

import pytest

# Stub out utils.file_utils before importing dashboard_fixer, since the module
# lives under scripts/ and isn't on sys.path at the repo root level.
_utils_pkg = types.ModuleType("utils")
_utils_pkg.file_utils = types.ModuleType("utils.file_utils")


def _stub_read_json(path, *a, **kw):
    """Stub that actually reads JSON files so transform data is available in tests."""
    with open(path) as f:
        return json.load(f)


import re as _re


def _real_slugify(title):
    slug = title.lower()
    slug = _re.sub(r"[^a-z0-9]+", "-", slug)
    return slug.strip("-")


_utils_pkg.file_utils.read_json = _stub_read_json
_utils_pkg.file_utils.write_json = lambda *a, **kw: None
_utils_pkg.file_utils.slugify_grafana_title = _real_slugify
sys.modules.setdefault("utils", _utils_pkg)
sys.modules.setdefault("utils.file_utils", _utils_pkg.file_utils)

from scripts.configurator.config import BundleConfig, BundleState, DashboardInfo, SummaryInfo, TransformInfo
from scripts.configurator.dashboard_fixer import (
    _build_inputs_map,
    _build_sibling_slug_map,
    _find_summary_var,
    _fix_hardcoded_uids,
    _fix_template_variables,
    _get_primary_timestamp_column,
    _slug_to_macro,
)
from scripts.utils.file_utils import slugify_grafana_title


# ---------------------------------------------------------------------------
# Shared helpers
# ---------------------------------------------------------------------------

def _make_var(name, query, var_type="constant"):
    """Create a Grafana template variable dict with self-referencing structure."""
    return {
        "hide": 2,
        "name": name,
        "query": query,
        "skipUrlSync": True,
        "type": var_type,
        "current": {
            "selected": False,
            "text": query,
            "value": query,
        },
        "options": [
            {
                "selected": False,
                "text": query,
                "value": query,
            }
        ],
    }


def _make_config(tmp_path, dry_run=False):
    """Create a minimal BundleConfig for testing."""
    bundle_dir = tmp_path / "trafficpeak" / "test_bundle"
    bundle_dir.mkdir(parents=True, exist_ok=True)
    return BundleConfig(
        bundle_dir=str(bundle_dir),
        table_name="test_table",
        data_category="cdn",
        source_name="test",
        bundle_name="test_bundle",
        dry_run=dry_run,
    )


def _make_transform_json(tmp_path, primary_col_name="reqTimeSec"):
    """Create a minimal transform.json with a primary timestamp column."""
    transform_data = {
        "name": "test_transform",
        "settings": {
            "output_columns": [
                {
                    "name": "version",
                    "datatype": {"type": "uint8", "primary": False},
                },
                {
                    "name": primary_col_name,
                    "datatype": {
                        "type": "epoch",
                        "primary": True,
                        "format": "s",
                        "resolution": "ms",
                    },
                },
                {
                    "name": "statusCode",
                    "datatype": {"type": "uint16", "primary": False},
                },
            ],
        },
    }
    transform_path = tmp_path / "transformations" / "transform.json"
    transform_path.parent.mkdir(parents=True, exist_ok=True)
    transform_path.write_text(json.dumps(transform_data))
    return str(transform_path)


def _make_state(transform_path=None):
    """Create a BundleState, optionally with one transform."""
    state = BundleState()
    if transform_path:
        tinfo = TransformInfo(original_path=transform_path)
        tinfo.final_path = transform_path
        state.transforms = [tinfo]
    return state


# ---------------------------------------------------------------------------
# LOTC-1302 / LOTC-1449: VAR_X constants classified by __inputs[VAR_X].value.
# (LOTC-1449 removed the old name-based self-reference fallback; these tests
# now exercise the value-based classifier with a raw-table input value.)
# ---------------------------------------------------------------------------

class TestSelfReferencingVarFix:
    """VAR_* constants pointing at the raw-logs table resolve to
    __PROJECT_NAME__.__TABLE_NAME__ via value-based classification."""

    def test_table_var_resolved(self, tmp_path):
        """Variable 'table' with VAR_TABLE bound to the raw-logs table resolves."""
        dashboard = {
            "templating": {
                "list": [_make_var("table", "${VAR_TABLE}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)  # table_name="test_table"
        state = _make_state()

        _fix_template_variables(
            dashboard, dinfo, {"VAR_TABLE": "akamai.test_table"}, config, state,
        )

        var = dashboard["templating"]["list"][0]
        expected = "__PROJECT_NAME__.__TABLE_NAME__"
        assert var["query"] == expected
        assert var["current"]["value"] == expected
        assert var["current"]["text"] == expected
        assert var["options"][0]["value"] == expected
        assert var["options"][0]["text"] == expected

    def test_logs_var_resolved(self, tmp_path):
        """VAR_LOGS bound to the raw-logs table resolves to table placeholder."""
        dashboard = {
            "templating": {
                "list": [_make_var("logs", "${VAR_LOGS}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()

        _fix_template_variables(
            dashboard, dinfo, {"VAR_LOGS": "akamai.test_table"}, config, state,
        )

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"

    def test_ds2_var_resolved(self, tmp_path):
        """VAR_DS2 bound to the raw-logs table resolves to table placeholder."""
        dashboard = {
            "templating": {
                "list": [_make_var("ds2", "${VAR_DS2}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()

        _fix_template_variables(
            dashboard, dinfo, {"VAR_DS2": "akamai.test_table"}, config, state,
        )

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"

    def test_non_self_referencing_var_not_touched(self, tmp_path):
        """A ${VAR_X} constant with no __inputs entry is left unchanged."""
        dashboard = {
            "templating": {
                "list": [_make_var("my_summary", "${VAR_SUMMARY_HOUR}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        # No inputs entry, no matching summary — stays untouched.
        assert var["query"] == "${VAR_SUMMARY_HOUR}"

    def test_case_insensitive_value_match(self, tmp_path):
        """Value matching against config.table_name is case-insensitive."""
        dashboard = {
            "templating": {
                "list": [_make_var("treemapCells", "${VAR_TREEMAPCELLS}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)  # table_name="test_table"
        state = _make_state()

        _fix_template_variables(
            dashboard, dinfo, {"VAR_TREEMAPCELLS": "AKAMAI.TEST_TABLE"}, config, state,
        )

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"

    def test_non_constant_type_not_affected(self, tmp_path):
        """A ${VAR_X} pattern in a non-constant variable type is not touched."""
        var = _make_var("table", "${VAR_TABLE}", var_type="query")
        dashboard = {"templating": {"list": [var]}}
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()

        _fix_template_variables(
            dashboard, dinfo, {"VAR_TABLE": "akamai.test_table"}, config, state,
        )

        assert dashboard["templating"]["list"][0]["query"] == "${VAR_TABLE}"

    def test_raw_table_still_injected(self, tmp_path):
        """raw_table injection still works alongside value-based resolution."""
        dashboard = {
            "templating": {
                "list": [
                    _make_var("table", "${VAR_TABLE}"),
                    _make_var("raw_table", "some_old_value"),
                ]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()

        _fix_template_variables(
            dashboard, dinfo, {"VAR_TABLE": "akamai.test_table"}, config, state,
        )

        var_list = dashboard["templating"]["list"]
        table_var = next(v for v in var_list if v["name"] == "table")
        raw_table_var = next(v for v in var_list if v["name"] == "raw_table")

        assert table_var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"
        assert raw_table_var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"

    def test_multiple_vars_resolved(self, tmp_path):
        """Multiple VAR_X constants bound to the raw-logs table are all resolved.
        VAR_TIMESTAMP has its own special-case path (LOTC-1303) independent of
        __inputs."""
        transform_path = _make_transform_json(tmp_path, "reqTimeSec")
        dashboard = {
            "templating": {
                "list": [
                    _make_var("table", "${VAR_TABLE}"),
                    _make_var("ds2", "${VAR_DS2}"),
                    _make_var("timestamp", "${VAR_TIMESTAMP}"),
                ]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state(transform_path)
        inputs_map = {
            "VAR_TABLE": "akamai.test_table",
            "VAR_DS2": "akamai.test_table",
        }

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        var_list = dashboard["templating"]["list"]
        table_var = next(v for v in var_list if v["name"] == "table")
        ds2_var = next(v for v in var_list if v["name"] == "ds2")
        ts_var = next(v for v in var_list if v["name"] == "timestamp")

        assert table_var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"
        assert ds2_var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"
        assert ts_var["query"] == "reqTimeSec"  # Resolved by LOTC-1303 timestamp fix


# ---------------------------------------------------------------------------
# LOTC-1303: VAR_TIMESTAMP → primary timestamp column from transform
# ---------------------------------------------------------------------------

class TestGetPrimaryTimestampColumn:
    """Tests for _get_primary_timestamp_column helper."""

    def test_returns_primary_column_name(self, tmp_path):
        """Should return the name of the column with datatype.primary == True."""
        transform_path = _make_transform_json(tmp_path, "reqTimeSec")
        config = _make_config(tmp_path)
        state = _make_state(transform_path)

        result = _get_primary_timestamp_column(config, state)
        assert result == "reqTimeSec"

    def test_returns_timestamp_column(self, tmp_path):
        """Should work when the primary column is named 'timestamp'."""
        transform_path = _make_transform_json(tmp_path, "timestamp")
        config = _make_config(tmp_path)
        state = _make_state(transform_path)

        result = _get_primary_timestamp_column(config, state)
        assert result == "timestamp"

    def test_returns_none_when_no_transforms(self, tmp_path):
        """Should return None when state has no transforms."""
        config = _make_config(tmp_path)
        state = BundleState()

        result = _get_primary_timestamp_column(config, state)
        assert result is None

    def test_returns_none_when_no_primary(self, tmp_path):
        """Should return None when no column has primary == True."""
        transform_data = {
            "name": "test",
            "settings": {
                "output_columns": [
                    {"name": "col1", "datatype": {"type": "string", "primary": False}},
                ],
            },
        }
        transform_path = tmp_path / "transform.json"
        transform_path.write_text(json.dumps(transform_data))

        config = _make_config(tmp_path)
        state = _make_state(str(transform_path))

        result = _get_primary_timestamp_column(config, state)
        assert result is None

    def test_dry_run_reads_original_path(self, tmp_path):
        """In dry_run mode, should read from original_path instead of final_path."""
        transform_path = _make_transform_json(tmp_path, "reqTimeSec")
        config = _make_config(tmp_path, dry_run=True)

        tinfo = TransformInfo(original_path=transform_path)
        tinfo.final_path = "/nonexistent/path/transform.json"
        state = BundleState()
        state.transforms = [tinfo]

        result = _get_primary_timestamp_column(config, state)
        assert result == "reqTimeSec"


class TestFixVarTimestamp:
    """Tests for VAR_TIMESTAMP self-referencing constant detection and fix."""

    def _make_dashboard_with_timestamp_var(self, query="${VAR_TIMESTAMP}", var_name="timestamp"):
        """Create a minimal dashboard dict with a timestamp template variable."""
        return {
            "templating": {
                "list": [
                    {
                        "name": var_name,
                        "type": "constant",
                        "query": query,
                        "current": {
                            "selected": False,
                            "text": query,
                            "value": query,
                        },
                        "options": [{
                            "selected": False,
                            "text": query,
                            "value": query,
                        }],
                    }
                ]
            }
        }

    def test_self_referencing_timestamp_resolved(self, tmp_path):
        """A constant named 'timestamp' with query '${VAR_TIMESTAMP}' should be
        resolved to the primary timestamp column from the transform."""
        transform_path = _make_transform_json(tmp_path, "reqTimeSec")
        config = _make_config(tmp_path)
        state = _make_state(transform_path)

        dashboard = self._make_dashboard_with_timestamp_var()
        dinfo = DashboardInfo(path="/fake/dashboard.json", is_primary=True)

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "reqTimeSec"
        assert var["current"]["value"] == "reqTimeSec"
        assert var["current"]["text"] == "reqTimeSec"
        assert var["options"][0]["value"] == "reqTimeSec"
        assert var["hide"] == 2
        assert var["skipUrlSync"] is True

    def test_resolves_to_different_primary_column(self, tmp_path):
        """When the transform's primary column is 'timestamp' (not 'reqTimeSec'),
        the variable should resolve to 'timestamp'."""
        transform_path = _make_transform_json(tmp_path, "timestamp")
        config = _make_config(tmp_path)
        state = _make_state(transform_path)

        dashboard = self._make_dashboard_with_timestamp_var()
        dinfo = DashboardInfo(path="/fake/dashboard.json", is_primary=True)

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "timestamp"

    def test_non_self_referencing_var_untouched(self, tmp_path):
        """A constant variable with a ${VAR_*} query that doesn't match its own
        name should NOT be resolved by the self-referencing fix."""
        transform_path = _make_transform_json(tmp_path, "reqTimeSec")
        config = _make_config(tmp_path)
        state = _make_state(transform_path)

        # Variable named "my_panel" with query "${VAR_SUMMARY_HOUR}" — name doesn't match VAR_ suffix
        dashboard = self._make_dashboard_with_timestamp_var(
            query="${VAR_SUMMARY_HOUR}", var_name="my_panel"
        )
        dinfo = DashboardInfo(path="/fake/dashboard.json", is_primary=True)

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        # Should remain unchanged (no summary match in state, not self-referencing)
        assert var["query"] == "${VAR_SUMMARY_HOUR}"

    def test_non_constant_timestamp_untouched(self, tmp_path):
        """A non-constant variable named 'timestamp' should not be modified."""
        transform_path = _make_transform_json(tmp_path, "reqTimeSec")
        config = _make_config(tmp_path)
        state = _make_state(transform_path)

        dashboard = {
            "templating": {
                "list": [
                    {
                        "name": "timestamp",
                        "type": "query",
                        "query": "SELECT DISTINCT timestamp FROM table",
                    }
                ]
            }
        }
        dinfo = DashboardInfo(path="/fake/dashboard.json", is_primary=True)

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "SELECT DISTINCT timestamp FROM table"

    def test_no_transform_leaves_var_unchanged(self, tmp_path):
        """When no transform is available, VAR_TIMESTAMP should be left as-is."""
        config = _make_config(tmp_path)
        state = BundleState()  # No transforms

        dashboard = self._make_dashboard_with_timestamp_var()
        dinfo = DashboardInfo(path="/fake/dashboard.json", is_primary=True)

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        # Still appended to list but query unchanged (no primary col found)
        assert var["query"] == "${VAR_TIMESTAMP}"


# ---------------------------------------------------------------------------
# LOTC-1435 / LOTC-1449: ${VAR_SUMMARY_*} resolves to summary placeholders by
# matching the __inputs value (e.g. `akamai.bot_summary_hour`) against the
# state.summaries name list, not by fuzzy name matching.
# ---------------------------------------------------------------------------

class TestSummaryVarPrecedence:
    """Raw Grafana exports use constants named summary_hour/day/month with
    queries `${VAR_SUMMARY_HOUR/DAY/MONTH}`. These must route to the matching
    summary table placeholder, via the __inputs value the author bound.
    """

    def _make_summary(self, name, dashboard_var):
        return SummaryInfo(path="/fake/sql", filename=f"{name}.sql", name=name, dashboard_var=dashboard_var)

    def test_summary_hour_primary_dashboard(self, tmp_path):
        """Primary dashboard: summary var omits __PROJECT_NAME__ prefix."""
        dashboard = {
            "templating": {
                "list": [_make_var("summary_hour", "${VAR_SUMMARY_HOUR}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()
        state.summaries = [self._make_summary("bot_summary_hour", "__SUMMARY_TABLE_NAME_1__")]
        inputs_map = {"VAR_SUMMARY_HOUR": "akamai.bot_summary_hour"}

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "__SUMMARY_TABLE_NAME_1__"
        assert var["current"]["value"] == "__SUMMARY_TABLE_NAME_1__"
        assert var["options"][0]["value"] == "__SUMMARY_TABLE_NAME_1__"

    def test_summary_hour_non_primary_dashboard(self, tmp_path):
        """Non-primary dashboard: summary var includes __PROJECT_NAME__ prefix."""
        dashboard = {
            "templating": {
                "list": [_make_var("summary_hour", "${VAR_SUMMARY_HOUR}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=False)
        config = _make_config(tmp_path)
        state = _make_state()
        state.summaries = [self._make_summary("bot_summary_hour", "__SUMMARY_TABLE_NAME_1__")]
        inputs_map = {"VAR_SUMMARY_HOUR": "akamai.bot_summary_hour"}

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "__PROJECT_NAME__.__SUMMARY_TABLE_NAME_1__"

    def test_all_three_summary_buckets_resolve(self, tmp_path):
        """summary_hour, summary_day, and summary_month each resolve to their
        own __SUMMARY_TABLE_NAME_N__ placeholder based on their __inputs values."""
        dashboard = {
            "templating": {
                "list": [
                    _make_var("summary_hour", "${VAR_SUMMARY_HOUR}"),
                    _make_var("summary_day", "${VAR_SUMMARY_DAY}"),
                    _make_var("summary_month", "${VAR_SUMMARY_MONTH}"),
                ]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()
        state.summaries = [
            self._make_summary("bot_summary_hour", "__SUMMARY_TABLE_NAME_1__"),
            self._make_summary("bot_summary_day", "__SUMMARY_TABLE_NAME_2__"),
            self._make_summary("bot_summary_month", "__SUMMARY_TABLE_NAME_3__"),
        ]
        inputs_map = {
            "VAR_SUMMARY_HOUR": "akamai.bot_summary_hour",
            "VAR_SUMMARY_DAY": "akamai.bot_summary_day",
            "VAR_SUMMARY_MONTH": "akamai.bot_summary_month",
        }

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        by_name = {v["name"]: v for v in dashboard["templating"]["list"] if v["name"].startswith("summary_")}
        assert by_name["summary_hour"]["query"] == "__SUMMARY_TABLE_NAME_1__"
        assert by_name["summary_day"]["query"] == "__SUMMARY_TABLE_NAME_2__"
        assert by_name["summary_month"]["query"] == "__SUMMARY_TABLE_NAME_3__"

    def test_summary_var_without_match_stays_unchanged(self, tmp_path):
        """If state has no summary matching the __inputs value, the variable
        is left untouched (LOTC-1449: no silent rewrite — validator surfaces)."""
        dashboard = {
            "templating": {
                "list": [_make_var("summary_hour", "${VAR_SUMMARY_HOUR}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()  # No summaries
        inputs_map = {"VAR_SUMMARY_HOUR": "akamai.bot_summary_hour"}

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "${VAR_SUMMARY_HOUR}"

    def test_table_var_still_resolves_when_summaries_present(self, tmp_path):
        """Regression: ${VAR_TABLE} bound to the raw-logs table must resolve
        to the raw-logs placeholder even when summaries exist in state."""
        dashboard = {
            "templating": {
                "list": [_make_var("table", "${VAR_TABLE}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)  # table_name="test_table"
        state = _make_state()
        state.summaries = [self._make_summary("bot_summary_hour", "__SUMMARY_TABLE_NAME_1__")]
        inputs_map = {"VAR_TABLE": "akamai.test_table"}

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"


# ---------------------------------------------------------------------------
# LOTC-1449: _find_summary_var is exact-match only. Fuzzy substring and
# endswith fallbacks are removed to prevent raw-table names that happen to be
# substrings of summary names (e.g. `edns` in `edns_summary_hour`) from
# silently hijacking the summary placeholder.
# ---------------------------------------------------------------------------


class TestFindSummaryVarExactMatch:
    def _make_summary(self, name, dashboard_var):
        return SummaryInfo(path="/fake/sql", filename=f"{name}.sql", name=name, dashboard_var=dashboard_var)

    def test_exact_name_match_returns_dashboard_var(self):
        state = BundleState()
        state.summaries = [self._make_summary("bot_summary_hour", "__SUMMARY_TABLE_NAME_1__")]

        assert _find_summary_var("bot_summary_hour", state) == "__SUMMARY_TABLE_NAME_1__"

    def test_substring_prefix_no_longer_matches(self):
        """`edns` must not match summary `edns_summary_hour` — the filed bug."""
        state = BundleState()
        state.summaries = [self._make_summary("edns_summary_hour", "__SUMMARY_TABLE_NAME_1__")]

        assert _find_summary_var("edns", state) is None

    def test_substring_suffix_no_longer_matches(self):
        """Legacy `summary_hour` → `bot_summary_hour` fuzzy match is gone."""
        state = BundleState()
        state.summaries = [self._make_summary("bot_summary_hour", "__SUMMARY_TABLE_NAME_1__")]

        assert _find_summary_var("summary_hour", state) is None


class TestBuildInputsMap:
    """LOTC-1449: inputs map carries __inputs[].value (deterministic signal)
    rather than __inputs[].label (author-chosen, unreliable)."""

    def test_maps_var_name_to_value(self):
        dashboard = {
            "__inputs": [
                {"name": "VAR_EDNS", "label": "edns", "value": "akamai.edns"},
                {"name": "VAR_EDNS_SUMMARY_HOUR", "label": "edns_summary_hour",
                 "value": "akamai.edns_summary_hour"},
            ]
        }

        result = _build_inputs_map(dashboard)

        assert result == {
            "VAR_EDNS": "akamai.edns",
            "VAR_EDNS_SUMMARY_HOUR": "akamai.edns_summary_hour",
        }

    def test_skips_entries_without_value(self):
        """Datasource inputs (no `value`) should be skipped — they're handled
        elsewhere via the datasource UID rewrite."""
        dashboard = {
            "__inputs": [
                {"name": "DS_MY_DATASOURCE", "type": "datasource", "label": "My DS"},
                {"name": "VAR_TABLE", "label": "table", "value": "akamai.logs"},
            ]
        }

        result = _build_inputs_map(dashboard)

        assert result == {"VAR_TABLE": "akamai.logs"}

    def test_empty_when_no_inputs(self):
        assert _build_inputs_map({}) == {}


# ---------------------------------------------------------------------------
# LOTC-1449: _fix_template_variables classifies ${VAR_X} constants by looking
# up __inputs[VAR_X].value. Values match summary tables (with or without a
# `<prefix>.` qualifier) or the raw-logs table; unknown values stay untouched.
# ---------------------------------------------------------------------------


class TestValueBasedClassification:
    def _make_summary(self, name, dashboard_var):
        return SummaryInfo(path="/fake/sql", filename=f"{name}.sql", name=name, dashboard_var=dashboard_var)

    def test_value_with_prefix_matches_summary(self, tmp_path):
        """`akamai.bot_summary_hour` should resolve to the bot_summary_hour summary."""
        dashboard = {
            "templating": {
                "list": [_make_var("my_hourly", "${VAR_HOURLY}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()
        state.summaries = [self._make_summary("bot_summary_hour", "__SUMMARY_TABLE_NAME_1__")]
        inputs_map = {"VAR_HOURLY": "akamai.bot_summary_hour"}

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        assert dashboard["templating"]["list"][0]["query"] == "__SUMMARY_TABLE_NAME_1__"

    def test_value_without_prefix_matches_summary(self, tmp_path):
        """Bare summary name (no `<prefix>.`) still matches."""
        dashboard = {
            "templating": {
                "list": [_make_var("my_hourly", "${VAR_HOURLY}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()
        state.summaries = [self._make_summary("bot_summary_hour", "__SUMMARY_TABLE_NAME_1__")]
        inputs_map = {"VAR_HOURLY": "bot_summary_hour"}

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        assert dashboard["templating"]["list"][0]["query"] == "__SUMMARY_TABLE_NAME_1__"

    def test_non_primary_dashboard_prefixes_summary(self, tmp_path):
        """Non-primary dashboards get __PROJECT_NAME__.__SUMMARY_TABLE_NAME_N__."""
        dashboard = {
            "templating": {
                "list": [_make_var("my_hourly", "${VAR_HOURLY}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=False)
        config = _make_config(tmp_path)
        state = _make_state()
        state.summaries = [self._make_summary("bot_summary_hour", "__SUMMARY_TABLE_NAME_1__")]
        inputs_map = {"VAR_HOURLY": "akamai.bot_summary_hour"}

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        assert dashboard["templating"]["list"][0]["query"] == "__PROJECT_NAME__.__SUMMARY_TABLE_NAME_1__"

    def test_value_with_prefix_matches_raw_table(self, tmp_path):
        """`akamai.test_table` should resolve to __PROJECT_NAME__.__TABLE_NAME__."""
        dashboard = {
            "templating": {
                "list": [_make_var("whatever", "${VAR_WHATEVER}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)  # table_name="test_table"
        state = _make_state()
        inputs_map = {"VAR_WHATEVER": "akamai.test_table"}

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        assert dashboard["templating"]["list"][0]["query"] == "__PROJECT_NAME__.__TABLE_NAME__"

    def test_value_without_prefix_matches_raw_table(self, tmp_path):
        """Bare `test_table` also matches."""
        dashboard = {
            "templating": {
                "list": [_make_var("whatever", "${VAR_WHATEVER}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()
        inputs_map = {"VAR_WHATEVER": "test_table"}

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        assert dashboard["templating"]["list"][0]["query"] == "__PROJECT_NAME__.__TABLE_NAME__"

    def test_edns_substring_collision_is_fixed(self, tmp_path):
        """Regression for the filed trafficpeak/edns bug: a raw-table var name
        (`edns`) that is a substring of a summary name (`edns_summary_hour`)
        must NOT hijack the summary placeholder. Prior to LOTC-1449 both
        `${edns}` and `${edns_summary_hour}` were rewritten to the same
        __SUMMARY_TABLE_NAME_1__, causing ClickHouse `code: 47` on raw-column
        queries against the summary."""
        dashboard = {
            "templating": {
                "list": [
                    _make_var("edns", "${VAR_EDNS}"),
                    _make_var("edns_summary_hour", "${VAR_EDNS_SUMMARY_HOUR}"),
                ]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        # config.table_name from _make_config is "test_table"; use bundle_dir
        # override to simulate edns table name.
        bundle_dir = tmp_path / "trafficpeak" / "edns"
        bundle_dir.mkdir(parents=True, exist_ok=True)
        config = BundleConfig(
            bundle_dir=str(bundle_dir),
            table_name="edns",
            data_category="dns",
            source_name="trafficpeak",
            bundle_name="edns",
        )
        state = _make_state()
        state.summaries = [self._make_summary("edns_summary_hour", "__SUMMARY_TABLE_NAME_1__")]
        inputs_map = {
            "VAR_EDNS": "akamai.edns",
            "VAR_EDNS_SUMMARY_HOUR": "akamai.edns_summary_hour",
        }

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        by_name = {v["name"]: v for v in dashboard["templating"]["list"]}
        assert by_name["edns"]["query"] == "__PROJECT_NAME__.__TABLE_NAME__"
        assert by_name["edns_summary_hour"]["query"] == "__SUMMARY_TABLE_NAME_1__"

    def test_unknown_value_leaves_var_unchanged(self, tmp_path):
        """If __inputs[VAR_X].value matches neither a summary nor the raw
        table, don't guess — leave it alone so the validator can flag it."""
        dashboard = {
            "templating": {
                "list": [_make_var("mystery", "${VAR_MYSTERY}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)  # table_name="test_table"
        state = _make_state()
        state.summaries = [self._make_summary("bot_summary_hour", "__SUMMARY_TABLE_NAME_1__")]
        inputs_map = {"VAR_MYSTERY": "akamai.some_other_thing"}

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        assert dashboard["templating"]["list"][0]["query"] == "${VAR_MYSTERY}"

    def test_no_inputs_entry_leaves_var_unchanged(self, tmp_path):
        """If a ${VAR_X} constant has no matching __inputs entry, leave it.
        The name-based self-reference fallback was removed in LOTC-1449."""
        dashboard = {
            "templating": {
                "list": [_make_var("orphan", "${VAR_ORPHAN}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        assert dashboard["templating"]["list"][0]["query"] == "${VAR_ORPHAN}"

    def test_summary_and_raw_table_name_collision_warns(self, tmp_path):
        """Defensive: if a summary name collides with the raw-table name, the
        summary check wins (first in resolution order). Surface a warning so
        the bundle author can disambiguate rather than hitting a downstream
        validator error with no trail back."""
        dashboard = {
            "templating": {
                "list": [_make_var("anything", "${VAR_ANYTHING}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        bundle_dir = tmp_path / "trafficpeak" / "collision"
        bundle_dir.mkdir(parents=True, exist_ok=True)
        config = BundleConfig(
            bundle_dir=str(bundle_dir),
            table_name="logs",  # same name as the summary below
            data_category="cdn",
            source_name="trafficpeak",
            bundle_name="collision",
        )
        state = _make_state()
        state.summaries = [self._make_summary("logs", "__SUMMARY_TABLE_NAME_1__")]
        inputs_map = {"VAR_ANYTHING": "akamai.logs"}

        _fix_template_variables(dashboard, dinfo, inputs_map, config, state)

        # Summary wins resolution order.
        assert dashboard["templating"]["list"][0]["query"] == "__SUMMARY_TABLE_NAME_1__"
        # Warning surfaces the ambiguity.
        assert any("collide" in w.lower() and "logs" in w for w in state.warnings)


# ---------------------------------------------------------------------------
# LOTC-1605 / LOTC-1615-1617: slugify_grafana_title + _fix_hardcoded_uids
# ---------------------------------------------------------------------------

class TestSlugifyGrafanaTitle:
    """slugify_grafana_title mirrors Grafana's own title-to-slug conversion."""

    def test_ascii_words(self):
        assert slugify_grafana_title("Raw Logs") == "raw-logs"

    def test_multiple_words(self):
        assert slugify_grafana_title("Cache Analysis Treemap") == "cache-analysis-treemap"

    def test_acronym_words(self):
        assert slugify_grafana_title("CDN Dashboard Default") == "cdn-dashboard-default"

    def test_single_word(self):
        assert slugify_grafana_title("Home") == "home"

    def test_punctuation_collapses(self):
        # Multiple non-alphanumeric chars in a row collapse to a single hyphen
        assert slugify_grafana_title("Foo  --  Bar") == "foo-bar"

    def test_accented_chars_treated_as_separators(self):
        # Non-ASCII chars are non-alphanumeric → replaced with hyphen
        assert slugify_grafana_title("Café Bar") == "caf-bar"

    def test_leading_trailing_stripped(self):
        assert slugify_grafana_title("  Hello World  ") == "hello-world"

    def test_slug_macro_roundtrip(self):
        assert _slug_to_macro("raw-logs") == "__DASHBOARD_UID_RAW_LOGS__"
        assert _slug_to_macro("cdn-dashboard-default") == "__DASHBOARD_UID_CDN_DASHBOARD_DEFAULT__"
        assert _slug_to_macro("cache-analysis-treemap") == "__DASHBOARD_UID_CACHE_ANALYSIS_TREEMAP__"


def _uid_dashboard(title, extra_panels=None):
    """Build a minimal dashboard dict with a title and optional panels list."""
    d = {"title": title, "templating": {"list": []}, "panels": extra_panels or []}
    return d


def _constant_var(name, value):
    return {
        "name": name,
        "type": "constant",
        "query": value,
        "hide": 2,
        "skipUrlSync": True,
        "current": {"selected": False, "text": value, "value": value},
        "options": [{"selected": False, "text": value, "value": value}],
    }


def _make_uid_config(tmp_path):
    bundle_dir = tmp_path / "aws" / "test_bundle"
    bundle_dir.mkdir(parents=True, exist_ok=True)
    return BundleConfig(
        bundle_dir=str(bundle_dir),
        table_name="logs",
        data_category="cdn",
        source_name="aws",
        bundle_name="test_bundle",
    )


class TestFixHardcodedUids:
    """_fix_hardcoded_uids rewrites <uuid>/<slug> patterns in dashboard JSON."""

    SELF_UUID = "fed820d2-e36b-4cfd-9570-8bd44ca92eea"
    SIBLING_UUID = "c44c5f0c-badf-4794-94a7-2ca3c6f37ade"
    EXTERNAL_UUID = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"

    # ------------------------------------------------------------------ Shape A

    def test_shape_a_self_reference(self, tmp_path):
        """Constant value <uuid>/<own-slug> → __DASHBOARD_UUID__/<own-slug>."""
        dashboard = _uid_dashboard("Cache Analysis Treemap")
        var = _constant_var("reset_link", f"{self.SELF_UUID}/cache-analysis-treemap")
        dashboard["templating"]["list"].append(var)

        dinfo = DashboardInfo(path="/fake/cache.json", filename="cache.json")
        config = _make_uid_config(tmp_path)
        state = _make_state()
        sibling_map = {}  # no siblings — self only

        _fix_hardcoded_uids(dashboard, dinfo, sibling_map, config, state)

        assert var["query"] == "__DASHBOARD_UUID__/cache-analysis-treemap"
        assert var["current"]["value"] == "__DASHBOARD_UUID__/cache-analysis-treemap"
        assert var["options"][0]["value"] == "__DASHBOARD_UUID__/cache-analysis-treemap"

    def test_shape_a_sibling_reference(self, tmp_path):
        """Constant value <uuid>/<sibling-slug> → __DASHBOARD_UID_*__/<sibling-slug>."""
        dashboard = _uid_dashboard("CDN Global View")
        var = _constant_var("raw_logs", f"{self.SIBLING_UUID}/raw-logs")
        dashboard["templating"]["list"].append(var)

        dinfo = DashboardInfo(path="/fake/global.json", filename="global.json")
        config = _make_uid_config(tmp_path)
        state = _make_state()
        sibling_map = {"raw-logs": "raw.json", "cdn-global-view": "global.json"}

        _fix_hardcoded_uids(dashboard, dinfo, sibling_map, config, state)

        assert var["query"] == "__DASHBOARD_UID_RAW_LOGS__/raw-logs"
        assert var["current"]["value"] == "__DASHBOARD_UID_RAW_LOGS__/raw-logs"
        assert var["options"][0]["text"] == "__DASHBOARD_UID_RAW_LOGS__/raw-logs"

    # ------------------------------------------------------------------ Shape B

    def test_shape_b_inline_macro_no_constant(self, tmp_path):
        """Panel URL /d/<uuid>/<sibling-slug>?q → /d/__MACRO__/<slug>?q (no constant)."""
        url = f"/d/{self.SIBLING_UUID}/raw-logs?var-x=1&var-y=2"
        dashboard = _uid_dashboard("CDN Global View")
        dashboard["panels"] = [{"links": [{"url": url}]}]

        dinfo = DashboardInfo(path="/fake/global.json", filename="global.json")
        config = _make_uid_config(tmp_path)
        state = _make_state()
        sibling_map = {"raw-logs": "raw.json", "cdn-global-view": "global.json"}

        _fix_hardcoded_uids(dashboard, dinfo, sibling_map, config, state)

        result_url = dashboard["panels"][0]["links"][0]["url"]
        assert result_url == "/d/__DASHBOARD_UID_RAW_LOGS__/raw-logs?var-x=1&var-y=2"

    def test_shape_b_constant_indirection(self, tmp_path):
        """Panel URL /d/<uuid>/<sibling-slug>?q → /d/${constant}?q when constant exists."""
        # Pass 1 will rewrite the constant; Pass 2 should then prefer ${raw_logs}
        url = f"/d/{self.SIBLING_UUID}/raw-logs?var-x=1"
        dashboard = _uid_dashboard("CDN Global View")
        # Add the constant that Pass 1 will rewrite
        var = _constant_var("raw_logs", f"{self.SIBLING_UUID}/raw-logs")
        dashboard["templating"]["list"].append(var)
        dashboard["panels"] = [{"links": [{"url": url}]}]

        dinfo = DashboardInfo(path="/fake/global.json", filename="global.json")
        config = _make_uid_config(tmp_path)
        state = _make_state()
        sibling_map = {"raw-logs": "raw.json", "cdn-global-view": "global.json"}

        _fix_hardcoded_uids(dashboard, dinfo, sibling_map, config, state)

        # Constant was rewritten by Pass 1
        assert var["query"] == "__DASHBOARD_UID_RAW_LOGS__/raw-logs"
        # URL uses ${raw_logs} indirection (not inline macro)
        result_url = dashboard["panels"][0]["links"][0]["url"]
        assert result_url == "/d/${raw_logs}?var-x=1"

    def test_shape_b_query_string_preserved(self, tmp_path):
        """Query string survives the URL rewrite intact."""
        qs = "?${__all_variables}&var-filter=client_country_iso_code|=|${__value.text}"
        url = f"/d/{self.SIBLING_UUID}/raw-logs{qs}"
        dashboard = _uid_dashboard("CDN Global View")
        dashboard["panels"] = [{"links": [{"url": url}]}]

        dinfo = DashboardInfo(path="/fake/global.json", filename="global.json")
        config = _make_uid_config(tmp_path)
        state = _make_state()
        sibling_map = {"raw-logs": "raw.json", "cdn-global-view": "global.json"}

        _fix_hardcoded_uids(dashboard, dinfo, sibling_map, config, state)

        result_url = dashboard["panels"][0]["links"][0]["url"]
        assert result_url == f"/d/__DASHBOARD_UID_RAW_LOGS__/raw-logs{qs}"

    # ------------------------------------------------------------------ Non-match / warn

    def test_nonmatching_uid_unchanged_and_warns(self, tmp_path):
        """UID whose slug doesn't match any sibling is left alone and a warning is emitted."""
        url = f"/d/{self.EXTERNAL_UUID}/some-external-dashboard?var-x=1"
        dashboard = _uid_dashboard("CDN Global View")
        dashboard["panels"] = [{"links": [{"url": url}]}]

        dinfo = DashboardInfo(path="/fake/global.json", filename="global.json")
        config = _make_uid_config(tmp_path)
        state = _make_state()
        sibling_map = {"raw-logs": "raw.json"}

        _fix_hardcoded_uids(dashboard, dinfo, sibling_map, config, state)

        # URL unchanged
        assert dashboard["panels"][0]["links"][0]["url"] == url
        # Warning emitted
        assert any("some-external-dashboard" in w for w in state.warnings)

    # ------------------------------------------------------------------ Slug collision

    def test_slug_collision_raises(self, tmp_path):
        """_build_sibling_slug_map raises ValueError when two dashboards share a slug."""
        import tempfile, json, os

        d1 = {"title": "My Dashboard"}
        d2 = {"title": "My  Dashboard"}  # same slug: "my-dashboard"

        with tempfile.TemporaryDirectory() as td:
            p1 = os.path.join(td, "d1.json")
            p2 = os.path.join(td, "d2.json")
            with open(p1, "w") as f:
                json.dump(d1, f)
            with open(p2, "w") as f:
                json.dump(d2, f)

            state = _make_state()
            state.dashboards = [
                DashboardInfo(path=p1, filename="d1.json"),
                DashboardInfo(path=p2, filename="d2.json"),
            ]

            with pytest.raises(ValueError, match="collision"):
                _build_sibling_slug_map(state)
