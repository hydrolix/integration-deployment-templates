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


_utils_pkg.file_utils.read_json = _stub_read_json
_utils_pkg.file_utils.write_json = lambda *a, **kw: None
sys.modules.setdefault("utils", _utils_pkg)
sys.modules.setdefault("utils.file_utils", _utils_pkg.file_utils)

from scripts.configurator.config import BundleConfig, BundleState, DashboardInfo, TransformInfo
from scripts.configurator.dashboard_fixer import _fix_template_variables, _get_primary_timestamp_column


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
# LOTC-1302: Self-referencing VAR_TABLE / VAR_LOGS / VAR_DS2 constants
# ---------------------------------------------------------------------------

class TestSelfReferencingVarFix:
    """Tests for self-referencing VAR_* constant resolution in _fix_template_variables."""

    def test_table_var_resolved(self, tmp_path):
        """Variable 'table' with query '${VAR_TABLE}' is resolved to __PROJECT_NAME__.__TABLE_NAME__."""
        dashboard = {
            "templating": {
                "list": [_make_var("table", "${VAR_TABLE}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        expected = "__PROJECT_NAME__.__TABLE_NAME__"
        assert var["query"] == expected
        assert var["current"]["value"] == expected
        assert var["current"]["text"] == expected
        assert var["options"][0]["value"] == expected
        assert var["options"][0]["text"] == expected

    def test_logs_var_resolved(self, tmp_path):
        """Variable 'logs' with query '${VAR_LOGS}' is resolved to table placeholder."""
        dashboard = {
            "templating": {
                "list": [_make_var("logs", "${VAR_LOGS}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"

    def test_ds2_var_resolved(self, tmp_path):
        """Variable 'ds2' with query '${VAR_DS2}' is resolved to table placeholder."""
        dashboard = {
            "templating": {
                "list": [_make_var("ds2", "${VAR_DS2}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"

    def test_non_self_referencing_var_not_touched(self, tmp_path):
        """A VAR_* reference that does NOT match the variable's own name is not treated
        as self-referencing (falls through to summary matching)."""
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
        # No matching summary in state, so query should remain unchanged
        assert var["query"] == "${VAR_SUMMARY_HOUR}"

    def test_case_insensitive_match(self, tmp_path):
        """Self-referencing detection is case-insensitive: 'treemapCells' matches VAR_TREEMAPCELLS."""
        dashboard = {
            "templating": {
                "list": [_make_var("treemapCells", "${VAR_TREEMAPCELLS}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"

    def test_non_constant_type_not_affected(self, tmp_path):
        """A self-referencing pattern in a non-constant variable type is not touched."""
        var = _make_var("table", "${VAR_TABLE}", var_type="query")
        dashboard = {"templating": {"list": [var]}}
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = _make_state()

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        assert dashboard["templating"]["list"][0]["query"] == "${VAR_TABLE}"

    def test_raw_table_still_injected(self, tmp_path):
        """raw_table injection still works alongside self-referencing fix."""
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

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var_list = dashboard["templating"]["list"]
        table_var = next(v for v in var_list if v["name"] == "table")
        raw_table_var = next(v for v in var_list if v["name"] == "raw_table")

        assert table_var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"
        assert raw_table_var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"

    def test_multiple_self_referencing_vars(self, tmp_path):
        """Multiple self-referencing vars in one dashboard are all resolved."""
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

        _fix_template_variables(dashboard, dinfo, {}, config, state)

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
