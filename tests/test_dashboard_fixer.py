"""Tests for dashboard_fixer module — VAR_TIMESTAMP self-referencing constant fix."""

import json
import os

import pytest

from scripts.configurator.config import BundleConfig, BundleState, DashboardInfo, TransformInfo
from scripts.configurator.dashboard_fixer import _fix_template_variables, _get_primary_timestamp_column


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


def _make_config(tmp_path, dry_run=False):
    """Create a minimal BundleConfig."""
    config = BundleConfig(
        bundle_dir=str(tmp_path),
        table_name="test_table",
        data_category="web",
        source_name="test",
        bundle_name="test_bundle",
    )
    config.dry_run = dry_run
    config.verbose = False
    return config


def _make_state(transform_path):
    """Create a BundleState with one transform."""
    tinfo = TransformInfo(original_path=transform_path)
    tinfo.final_path = transform_path
    state = BundleState()
    state.transforms = [tinfo]
    return state


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
        """A constant variable with a ${VAR_*} query that doesn't self-reference
        should NOT be modified by the timestamp fix."""
        transform_path = _make_transform_json(tmp_path, "reqTimeSec")
        config = _make_config(tmp_path)
        state = _make_state(transform_path)

        # Variable named "cdn_panel" with query "${VAR_CDN_PANEL}" — not self-referencing timestamp
        dashboard = self._make_dashboard_with_timestamp_var(
            query="${VAR_CDN_PANEL}", var_name="cdn_panel"
        )
        dinfo = DashboardInfo(path="/fake/dashboard.json", is_primary=True)

        _fix_template_variables(dashboard, dinfo, {}, config, state)

        var = dashboard["templating"]["list"][0]
        # Should remain unchanged (no summary match either, so stays as-is)
        assert var["query"] == "${VAR_CDN_PANEL}"

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
