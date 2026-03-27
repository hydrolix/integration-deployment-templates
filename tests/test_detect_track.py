"""Tests for detect_track module — classifying bundle state and routing pipeline tracks."""

import json
import os
import tempfile

import pytest

from scripts.detect_track import classify_bundle_state, detect_track


class TestClassifyBundleState:
    """Tests for classify_bundle_state() — determines if a bundle is raw, configured, or ambiguous."""

    def test_configured_bundle_detected(self, tmp_path):
        """A bundle with configured markers (wrapped dashboard, __DATASOURCE__, bundle.json,
        transformations/ dir, no __inputs) is classified as 'configured'."""
        bundle_dir = tmp_path / "aws" / "test_bundle"
        bundle_dir.mkdir(parents=True)

        # Configured dashboard: wrapped in {"dashboard": ...}, has __DATASOURCE__, no __inputs
        dashboards = bundle_dir / "dashboards"
        dashboards.mkdir()
        (dashboards / "default.json").write_text(json.dumps({
            "dashboard": {
                "__elements": {
                    "model": {
                        "datasource": {
                            "type": "hydrolix-hydrolix-datasource",
                            "uid": "__DATASOURCE__"
                        }
                    }
                },
                "panels": [
                    {
                        "datasource": {"uid": "__DATASOURCE__"},
                        "targets": [{"rawSql": "SELECT * FROM __PROJECT_NAME__.__TABLE_NAME__"}]
                    }
                ],
                "templating": {"list": []}
            }
        }))

        # Configured transforms: in transformations/ dir
        transforms = bundle_dir / "transformations" / "my_transform"
        transforms.mkdir(parents=True)
        (transforms / "transform.json").write_text(json.dumps({
            "name": "my_transform",
            "settings": {"output_columns": []}
        }))

        # bundle.json exists (auto-generated)
        (bundle_dir / "bundle.json").write_text(json.dumps({
            "name": "test_bundle",
            "tables": []
        }))

        # Summary SQL with template variables
        summaries = bundle_dir / "summaries"
        summaries.mkdir()
        (summaries / "summary.sql").write_text(
            "SELECT * FROM __PROJECT_NAME__.__TABLE_NAME__ GROUP BY timestamp"
        )

        assert classify_bundle_state(str(bundle_dir)) == "configured"

    def test_raw_bundle_detected(self, tmp_path):
        """A bundle with raw markers (__inputs in dashboard, no __DATASOURCE__, no bundle.json,
        bare dashboard JSON, hardcoded SQL) is classified as 'raw'."""
        bundle_dir = tmp_path / "aws" / "raw_bundle"
        bundle_dir.mkdir(parents=True)

        # Raw dashboard: NOT wrapped, has __inputs, no __DATASOURCE__
        dashboards = bundle_dir / "dashboards"
        dashboards.mkdir()
        (dashboards / "default.json").write_text(json.dumps({
            "__inputs": [
                {"name": "DS_HYDROLIX", "type": "datasource", "pluginId": "hydrolix-datasource"}
            ],
            "panels": [
                {
                    "datasource": {"uid": "${DS_HYDROLIX}"},
                    "targets": [{"rawSql": "SELECT * FROM mydb.mytable"}]
                }
            ],
            "templating": {"list": []}
        }))

        # Raw transforms: in transforms/ dir (not transformations/)
        transforms = bundle_dir / "transforms" / "my_transform"
        transforms.mkdir(parents=True)
        (transforms / "transform.json").write_text(json.dumps({
            "name": "my_transform",
            "uuid": "abc-123",
            "table": "mytable",
            "settings": {"output_columns": []}
        }))

        # No bundle.json (not yet generated)

        # Summary SQL with hardcoded table names
        summaries = bundle_dir / "summaries"
        summaries.mkdir()
        (summaries / "summary.sql").write_text(
            "SELECT * FROM mydb.mytable GROUP BY timestamp"
        )

        assert classify_bundle_state(str(bundle_dir)) == "raw"

    def test_ambiguous_bundle_detected(self, tmp_path):
        """A bundle with mixed signals (some raw, some configured markers) is classified as 'ambiguous'."""
        bundle_dir = tmp_path / "aws" / "mixed_bundle"
        bundle_dir.mkdir(parents=True)

        # Mixed dashboard: wrapped (configured) BUT has __inputs (raw), no __DATASOURCE__ (raw)
        dashboards = bundle_dir / "dashboards"
        dashboards.mkdir()
        (dashboards / "default.json").write_text(json.dumps({
            "dashboard": {
                "__inputs": [
                    {"name": "DS_HYDROLIX", "type": "datasource"}
                ],
                "panels": [
                    {
                        "datasource": {"uid": "${DS_HYDROLIX}"},
                        "targets": [{"rawSql": "SELECT * FROM mydb.mytable"}]
                    }
                ]
            }
        }))

        # bundle.json exists (configured signal) but no summary SQL
        (bundle_dir / "bundle.json").write_text(json.dumps({"name": "mixed"}))

        assert classify_bundle_state(str(bundle_dir)) == "ambiguous"


class TestDetectTrack:
    """Tests for detect_track() — routes bundles to 'full' or 'validate-only' pipeline track."""

    def test_originals_touched_returns_full(self, tmp_path):
        """When changed files include paths under .originals/, always returns 'full'."""
        repo_root = tmp_path
        bundle_dir = repo_root / "aws" / "cdn-insights"
        bundle_dir.mkdir(parents=True)
        (bundle_dir / "bundle-config.json").write_text(json.dumps({"table_name": "test"}))

        # .originals/ exists with content
        originals = repo_root / ".originals" / "aws" / "cdn-insights"
        originals.mkdir(parents=True)
        (originals / "something.json").write_text("{}")

        changed_files = [".originals/aws/cdn-insights/something.json"]

        result = detect_track(str(bundle_dir), changed_files, str(repo_root))
        assert result == "full"

    def test_first_run_raw_with_config_returns_full(self, tmp_path):
        """First run: no .originals/, raw assets, has bundle-config.json → returns 'full'."""
        repo_root = tmp_path
        bundle_dir = repo_root / "aws" / "first-run-bundle"
        bundle_dir.mkdir(parents=True)

        # bundle-config.json present (required for full pipeline)
        (bundle_dir / "bundle-config.json").write_text(json.dumps({"table_name": "test"}))

        # Raw dashboard: bare JSON (not wrapped), has __inputs, no __DATASOURCE__
        dashboards = bundle_dir / "dashboards"
        dashboards.mkdir()
        (dashboards / "default.json").write_text(json.dumps({
            "__inputs": [
                {"name": "DS_HYDROLIX", "type": "datasource"}
            ],
            "panels": [
                {
                    "datasource": {"uid": "${DS_HYDROLIX}"},
                    "targets": [{"rawSql": "SELECT * FROM mydb.mytable"}]
                }
            ],
            "templating": {"list": []}
        }))

        # No .originals/ directory
        # No bundle.json (not yet generated)

        changed_files = ["aws/first-run-bundle/dashboards/default.json"]

        result = detect_track(str(bundle_dir), changed_files, str(repo_root))
        assert result == "full"

    def test_configured_edits_with_originals_returns_validate_only(self, tmp_path):
        """Configured bundle with .originals/ existing → returns 'validate-only'."""
        repo_root = tmp_path
        bundle_dir = repo_root / "aws" / "configured-bundle"
        bundle_dir.mkdir(parents=True)

        # Configured dashboard: wrapped, __DATASOURCE__, no __inputs
        dashboards = bundle_dir / "dashboards"
        dashboards.mkdir()
        (dashboards / "default.json").write_text(json.dumps({
            "dashboard": {
                "__elements": {
                    "model": {
                        "datasource": {
                            "type": "hydrolix-hydrolix-datasource",
                            "uid": "__DATASOURCE__"
                        }
                    }
                },
                "panels": [
                    {
                        "datasource": {"uid": "__DATASOURCE__"},
                        "targets": [{"rawSql": "SELECT * FROM __PROJECT_NAME__.__TABLE_NAME__"}]
                    }
                ],
                "templating": {"list": []}
            }
        }))

        # bundle.json (configured marker)
        (bundle_dir / "bundle.json").write_text(json.dumps({"name": "configured-bundle"}))

        # Summary SQL with template variables
        summaries = bundle_dir / "summaries"
        summaries.mkdir()
        (summaries / "summary.sql").write_text(
            "SELECT * FROM __PROJECT_NAME__.__TABLE_NAME__ GROUP BY timestamp"
        )

        # .originals/ exists for this bundle
        originals = repo_root / ".originals" / "aws" / "configured-bundle"
        originals.mkdir(parents=True)
        (originals / "something.json").write_text("{}")

        # Changed files are in the main bundle dir, NOT in .originals/
        changed_files = ["aws/configured-bundle/dashboards/default.json"]

        result = detect_track(str(bundle_dir), changed_files, str(repo_root))
        assert result == "validate-only"

    def test_legacy_bundle_no_originals_no_config_returns_validate_only(self, tmp_path):
        """Legacy bundle: no .originals/, no bundle-config.json → returns 'validate-only'."""
        repo_root = tmp_path
        bundle_dir = repo_root / "aws" / "legacy-bundle"
        bundle_dir.mkdir(parents=True)

        # Configured dashboard (already processed, legacy)
        dashboards = bundle_dir / "dashboards"
        dashboards.mkdir()
        (dashboards / "default.json").write_text(json.dumps({
            "dashboard": {
                "panels": [
                    {
                        "datasource": {"uid": "__DATASOURCE__"},
                        "targets": [{"rawSql": "SELECT * FROM __PROJECT_NAME__.__TABLE_NAME__"}]
                    }
                ],
                "templating": {"list": []}
            }
        }))

        # bundle.json exists
        (bundle_dir / "bundle.json").write_text(json.dumps({"name": "legacy-bundle"}))

        # No .originals/ directory
        # No bundle-config.json

        changed_files = ["aws/legacy-bundle/dashboards/default.json"]

        result = detect_track(str(bundle_dir), changed_files, str(repo_root))
        assert result == "validate-only"

    def test_track1_missing_config_raises_valueerror(self, tmp_path):
        """Track 1 detected (.originals/ in changed files) but missing bundle-config.json → ValueError."""
        repo_root = tmp_path
        bundle_dir = repo_root / "aws" / "no-config-bundle"
        bundle_dir.mkdir(parents=True)

        # .originals/ exists
        originals = repo_root / ".originals" / "aws" / "no-config-bundle"
        originals.mkdir(parents=True)
        (originals / "dashboard.json").write_text("{}")

        # No bundle-config.json!

        changed_files = [".originals/aws/no-config-bundle/dashboard.json"]

        with pytest.raises(ValueError, match="bundle-config.json"):
            detect_track(str(bundle_dir), changed_files, str(repo_root))

    def test_real_cdn_insights_bundle_returns_validate_only(self):
        """Real bundle: aws/cdn-insights is configured, no .originals/, no bundle-config.json → 'validate-only'."""
        repo_root = "/Users/kevinborkman/Desktop/hydrolix/console/integration-deployment-templates"
        bundle_dir = os.path.join(repo_root, "aws", "cdn-insights")

        # No changed files in .originals/
        changed_files = ["aws/cdn-insights/dashboards/default.json"]

        result = detect_track(bundle_dir, changed_files, repo_root)
        assert result == "validate-only"
