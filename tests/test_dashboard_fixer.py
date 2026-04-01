"""Tests for dashboard_fixer module — fixing self-referencing VAR_* template variables."""

import sys
import types

import pytest

# Stub out utils.file_utils before importing dashboard_fixer, since the module
# lives under scripts/ and isn't on sys.path at the repo root level.
_utils_pkg = types.ModuleType("utils")
_utils_pkg.file_utils = types.ModuleType("utils.file_utils")
_utils_pkg.file_utils.read_json = lambda *a, **kw: {}
_utils_pkg.file_utils.write_json = lambda *a, **kw: None
sys.modules.setdefault("utils", _utils_pkg)
sys.modules.setdefault("utils.file_utils", _utils_pkg.file_utils)

from scripts.configurator.config import BundleConfig, BundleState, DashboardInfo
from scripts.configurator.dashboard_fixer import _fix_template_variables


def _make_config(tmp_path):
    """Create a minimal BundleConfig for testing."""
    bundle_dir = tmp_path / "trafficpeak" / "test_bundle"
    bundle_dir.mkdir(parents=True)
    return BundleConfig(
        bundle_dir=str(bundle_dir),
        table_name="test_table",
        data_category="cdn",
    )


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
        state = BundleState()

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
        state = BundleState()

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
        state = BundleState()

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"

    def test_timestamp_var_not_resolved(self, tmp_path):
        """Variable 'timestamp' with query '${VAR_TIMESTAMP}' is left untouched (LOTC-1303)."""
        dashboard = {
            "templating": {
                "list": [_make_var("timestamp", "${VAR_TIMESTAMP}")]
            }
        }
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = BundleState()

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "${VAR_TIMESTAMP}"
        assert var["current"]["value"] == "${VAR_TIMESTAMP}"

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
        state = BundleState()

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
        state = BundleState()

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        assert var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"

    def test_non_constant_type_not_affected(self, tmp_path):
        """A self-referencing pattern in a non-constant variable type is not touched."""
        var = _make_var("table", "${VAR_TABLE}", var_type="query")
        dashboard = {"templating": {"list": [var]}}
        dinfo = DashboardInfo(path="/fake/path.json", is_primary=True)
        config = _make_config(tmp_path)
        state = BundleState()

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
        state = BundleState()

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var_list = dashboard["templating"]["list"]
        table_var = next(v for v in var_list if v["name"] == "table")
        raw_table_var = next(v for v in var_list if v["name"] == "raw_table")

        assert table_var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"
        assert raw_table_var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"

    def test_multiple_self_referencing_vars(self, tmp_path):
        """Multiple self-referencing vars in one dashboard are all resolved."""
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
        state = BundleState()

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var_list = dashboard["templating"]["list"]
        table_var = next(v for v in var_list if v["name"] == "table")
        ds2_var = next(v for v in var_list if v["name"] == "ds2")
        ts_var = next(v for v in var_list if v["name"] == "timestamp")

        assert table_var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"
        assert ds2_var["query"] == "__PROJECT_NAME__.__TABLE_NAME__"
        assert ts_var["query"] == "${VAR_TIMESTAMP}"  # Untouched
