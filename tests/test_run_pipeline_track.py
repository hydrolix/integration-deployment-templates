"""Tests for --track flag integration in run_pipeline.py."""

import json
import os
import sys

import pytest

from scripts.run_pipeline import parse_args, resolve_stages, _apply_track, REPO_ROOT


class TestTrackFlag:
    """Tests for --track flag behavior in the pipeline orchestrator."""

    def test_track_validate_only_forces_only_validate(self, monkeypatch):
        """--track validate-only should force --only-validate, running only Stage 3."""
        monkeypatch.setattr(
            "sys.argv",
            ["run_pipeline.py", "--bundle-dir", "aws/test-bundle", "--track", "validate-only"],
        )
        args = parse_args()
        assert args.track == "validate-only"

        _apply_track(args)
        assert args.only_validate is True

        stages = resolve_stages(args)
        stage_names = [name for name, _ in stages]
        assert stage_names == ["validate"]

    def test_track_full_creates_originals_on_first_run(self, tmp_path, monkeypatch):
        """--track full on a bundle with no .originals/ should backup raw assets to .originals/."""
        # Set up a raw bundle in a temp repo root
        repo_root = tmp_path
        bundle_rel = "aws/new-bundle"
        bundle_dir = repo_root / "aws" / "new-bundle"
        bundle_dir.mkdir(parents=True)

        # Raw assets
        dashboards = bundle_dir / "dashboards"
        dashboards.mkdir()
        (dashboards / "default.json").write_text(json.dumps({"panels": [], "__inputs": []}))
        (bundle_dir / "bundle-config.json").write_text(json.dumps({"table_name": "test"}))

        # Patch REPO_ROOT so _apply_track uses our temp dir
        monkeypatch.setattr("scripts.run_pipeline.REPO_ROOT", str(repo_root))
        monkeypatch.setattr(
            "sys.argv",
            ["run_pipeline.py", "--bundle-dir", str(bundle_dir), "--track", "full"],
        )
        args = parse_args()
        _apply_track(args)

        # .originals/ should now exist with the backed-up dashboard
        originals_dir = repo_root / ".originals" / "aws" / "new-bundle"
        assert originals_dir.is_dir()
        assert (originals_dir / "dashboards" / "default.json").is_file()
        # bundle-config.json should NOT be in .originals/
        assert not (originals_dir / "bundle-config.json").exists()

    def test_track_full_restores_from_originals_on_rerun(self, tmp_path, monkeypatch):
        """--track full on a bundle WITH .originals/ should restore raw assets before pipeline."""
        repo_root = tmp_path
        bundle_dir = repo_root / "aws" / "rerun-bundle"
        bundle_dir.mkdir(parents=True)

        # Configured assets currently in bundle dir
        dashboards = bundle_dir / "dashboards"
        dashboards.mkdir()
        (dashboards / "default.json").write_text("configured content")
        (bundle_dir / "bundle-config.json").write_text(json.dumps({"table_name": "test"}))

        # .originals/ has the raw version
        originals = repo_root / ".originals" / "aws" / "rerun-bundle" / "dashboards"
        originals.mkdir(parents=True)
        (originals / "default.json").write_text("raw original content")

        monkeypatch.setattr("scripts.run_pipeline.REPO_ROOT", str(repo_root))
        monkeypatch.setattr(
            "sys.argv",
            ["run_pipeline.py", "--bundle-dir", str(bundle_dir), "--track", "full"],
        )
        args = parse_args()
        _apply_track(args)

        # Bundle dir should now have the restored raw content
        restored = (dashboards / "default.json").read_text()
        assert restored == "raw original content"
        # bundle-config.json should be preserved
        assert (bundle_dir / "bundle-config.json").is_file()

    def test_track_auto_configured_bundle_routes_validate_only(self, tmp_path, monkeypatch):
        """--track auto on a configured bundle (no .originals/, no bundle-config.json) → validate-only."""
        repo_root = tmp_path
        bundle_dir = repo_root / "aws" / "legacy-bundle"
        bundle_dir.mkdir(parents=True)

        # Configured dashboard
        dashboards = bundle_dir / "dashboards"
        dashboards.mkdir()
        (dashboards / "default.json").write_text(json.dumps({
            "dashboard": {
                "__elements": {"model": {"datasource": {"uid": "__DATASOURCE__"}}},
                "panels": [{"datasource": {"uid": "__DATASOURCE__"},
                            "targets": [{"rawSql": "SELECT * FROM __PROJECT_NAME__.__TABLE_NAME__"}]}]
            }
        }))
        (bundle_dir / "bundle.json").write_text(json.dumps({"name": "legacy"}))
        summaries = bundle_dir / "summaries"
        summaries.mkdir()
        (summaries / "s.sql").write_text("SELECT * FROM __PROJECT_NAME__.__TABLE_NAME__")

        monkeypatch.setattr("scripts.run_pipeline.REPO_ROOT", str(repo_root))
        monkeypatch.setattr(
            "sys.argv",
            ["run_pipeline.py", "--bundle-dir", str(bundle_dir), "--track", "auto"],
        )
        args = parse_args()
        _apply_track(args)

        assert args.track == "validate-only"
        assert args.only_validate is True

    def test_track_auto_raw_bundle_with_config_routes_full(self, tmp_path, monkeypatch):
        """--track auto on a raw bundle with bundle-config.json (first run) → full pipeline."""
        repo_root = tmp_path
        bundle_dir = repo_root / "aws" / "fresh-bundle"
        bundle_dir.mkdir(parents=True)

        # Raw dashboard
        dashboards = bundle_dir / "dashboards"
        dashboards.mkdir()
        (dashboards / "default.json").write_text(json.dumps({
            "__inputs": [{"name": "DS_HYDROLIX"}],
            "panels": [{"datasource": {"uid": "${DS_HYDROLIX}"},
                        "targets": [{"rawSql": "SELECT * FROM mydb.mytable"}]}]
        }))
        (bundle_dir / "bundle-config.json").write_text(json.dumps({"table_name": "test"}))
        summaries = bundle_dir / "summaries"
        summaries.mkdir()
        (summaries / "s.sql").write_text("SELECT * FROM mydb.mytable")

        monkeypatch.setattr("scripts.run_pipeline.REPO_ROOT", str(repo_root))
        monkeypatch.setattr(
            "sys.argv",
            ["run_pipeline.py", "--bundle-dir", str(bundle_dir), "--track", "auto"],
        )
        args = parse_args()
        _apply_track(args)

        assert args.track == "full"
        # Should have created .originals/ backup
        originals = repo_root / ".originals" / "aws" / "fresh-bundle"
        assert originals.is_dir()
