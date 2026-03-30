"""Tests for originals_manager module — managing .originals/ directory for clean pipeline re-runs."""

import json
import os
import shutil
import tempfile

import pytest

from scripts.originals_manager import backup_to_originals, restore_from_originals, update_originals


class TestBackupToOriginals:
    """Tests for backup_to_originals() — copying raw bundle assets into .originals/."""

    def test_backup_creates_correct_structure(self, tmp_path):
        """Given a bundle with dashboards/, transforms/, and sample data,
        backup_to_originals mirrors them into .originals/<relative_path>/."""
        repo_root = tmp_path / "repo"
        repo_root.mkdir()

        bundle_dir = repo_root / "trafficpeak" / "my_bundle"
        bundle_dir.mkdir(parents=True)

        # Create some raw assets
        dashboards = bundle_dir / "dashboards"
        dashboards.mkdir()
        (dashboards / "overview.json").write_text('{"dashboard": {}}')

        transforms = bundle_dir / "transforms"
        transforms.mkdir()
        (transforms / "main.sql").write_text("SELECT 1")

        (bundle_dir / "sample_data.csv").write_text("col1,col2\na,b")

        originals_path = backup_to_originals(str(bundle_dir), str(repo_root))

        expected_path = os.path.join(str(repo_root), ".originals", "trafficpeak", "my_bundle")
        assert originals_path == expected_path
        assert os.path.isdir(originals_path)
        assert os.path.isfile(os.path.join(originals_path, "dashboards", "overview.json"))
        assert os.path.isfile(os.path.join(originals_path, "transforms", "main.sql"))
        assert os.path.isfile(os.path.join(originals_path, "sample_data.csv"))

    def test_backup_excludes_bundle_json_and_bundle_config_json(self, tmp_path):
        """backup_to_originals should NOT copy bundle.json or bundle-config.json."""
        repo_root = tmp_path / "repo"
        repo_root.mkdir()

        bundle_dir = repo_root / "aws" / "test_bundle"
        bundle_dir.mkdir(parents=True)

        # Create assets plus the excluded files
        (bundle_dir / "dashboards").mkdir()
        (bundle_dir / "dashboards" / "dash.json").write_text("{}")
        (bundle_dir / "bundle.json").write_text('{"name": "test"}')
        (bundle_dir / "bundle-config.json").write_text('{"config": true}')

        originals_path = backup_to_originals(str(bundle_dir), str(repo_root))

        assert os.path.isfile(os.path.join(originals_path, "dashboards", "dash.json"))
        assert not os.path.exists(os.path.join(originals_path, "bundle.json"))
        assert not os.path.exists(os.path.join(originals_path, "bundle-config.json"))


class TestRestoreFromOriginals:
    """Tests for restore_from_originals() — replacing configured assets with raw originals."""

    def test_restore_replaces_configured_with_originals(self, tmp_path):
        """Given a bundle dir with configured assets and .originals/ with raw assets,
        restore should replace configured content with the raw originals."""
        repo_root = tmp_path / "repo"
        repo_root.mkdir()

        bundle_dir = repo_root / "aws" / "test_bundle"
        bundle_dir.mkdir(parents=True)

        # Current bundle has configured content
        dashboards = bundle_dir / "dashboards"
        dashboards.mkdir()
        (dashboards / "dash.json").write_text('{"configured": true}')
        (bundle_dir / "bundle.json").write_text('{"name": "test"}')
        (bundle_dir / "bundle-config.json").write_text('{"config": true}')

        # Set up .originals/ with raw content
        originals_dir = repo_root / ".originals" / "aws" / "test_bundle"
        originals_dir.mkdir(parents=True)
        orig_dashboards = originals_dir / "dashboards"
        orig_dashboards.mkdir()
        (orig_dashboards / "dash.json").write_text('{"raw": true}')
        (originals_dir / "sample_data.csv").write_text("col1\nval1")

        restore_from_originals(str(bundle_dir), str(repo_root))

        # Configured content replaced with raw originals
        with open(str(bundle_dir / "dashboards" / "dash.json")) as f:
            assert json.loads(f.read()) == {"raw": True}
        assert os.path.isfile(str(bundle_dir / "sample_data.csv"))
        # bundle.json (a configured artifact) should be gone
        assert not os.path.exists(str(bundle_dir / "bundle.json"))

    def test_restore_preserves_bundle_config_json(self, tmp_path):
        """After restore, bundle-config.json should still exist with its original content."""
        repo_root = tmp_path / "repo"
        repo_root.mkdir()

        bundle_dir = repo_root / "aws" / "test_bundle"
        bundle_dir.mkdir(parents=True)

        config_content = '{"project": "myproject", "table": "mytable"}'
        (bundle_dir / "bundle-config.json").write_text(config_content)
        (bundle_dir / "dashboards").mkdir()
        (bundle_dir / "dashboards" / "dash.json").write_text('{"configured": true}')

        # Set up .originals/
        originals_dir = repo_root / ".originals" / "aws" / "test_bundle"
        originals_dir.mkdir(parents=True)
        (originals_dir / "dashboards").mkdir()
        (originals_dir / "dashboards" / "dash.json").write_text('{"raw": true}')

        restore_from_originals(str(bundle_dir), str(repo_root))

        assert os.path.isfile(str(bundle_dir / "bundle-config.json"))
        with open(str(bundle_dir / "bundle-config.json")) as f:
            assert f.read() == config_content


class TestUpdateOriginals:
    """Tests for update_originals() — replacing stale .originals/ with fresh backup."""

    def test_update_replaces_old_originals(self, tmp_path):
        """Given existing .originals/ with old content and bundle_dir with new content,
        update should delete old .originals/ and create new backup from current bundle_dir."""
        repo_root = tmp_path / "repo"
        repo_root.mkdir()

        bundle_dir = repo_root / "aws" / "test_bundle"
        bundle_dir.mkdir(parents=True)

        # Set up old .originals/ with outdated content
        old_originals = repo_root / ".originals" / "aws" / "test_bundle"
        old_originals.mkdir(parents=True)
        (old_originals / "old_file.txt").write_text("old content")

        # Current bundle has new content
        (bundle_dir / "dashboards").mkdir()
        (bundle_dir / "dashboards" / "new_dash.json").write_text('{"new": true}')
        (bundle_dir / "new_data.csv").write_text("new,data")

        originals_path = update_originals(str(bundle_dir), str(repo_root))

        # Old content should be gone
        assert not os.path.exists(os.path.join(originals_path, "old_file.txt"))
        # New content should be present
        assert os.path.isfile(os.path.join(originals_path, "dashboards", "new_dash.json"))
        assert os.path.isfile(os.path.join(originals_path, "new_data.csv"))


class TestVersionedPathSupport:
    """Tests for versioned bundle paths like trafficpeak/default_shared/1.0.6/."""

    def test_backup_with_versioned_path(self, tmp_path):
        """backup_to_originals mirrors versioned paths like
        trafficpeak/default_shared/1.0.6/ to .originals/trafficpeak/default_shared/1.0.6/."""
        repo_root = tmp_path / "repo"
        repo_root.mkdir()

        bundle_dir = repo_root / "trafficpeak" / "default_shared" / "1.0.6"
        bundle_dir.mkdir(parents=True)

        (bundle_dir / "dashboards").mkdir()
        (bundle_dir / "dashboards" / "overview.json").write_text('{"raw": true}')
        (bundle_dir / "transforms").mkdir()
        (bundle_dir / "transforms" / "ingest.sql").write_text("SELECT 1")

        originals_path = backup_to_originals(str(bundle_dir), str(repo_root))

        expected = os.path.join(str(repo_root), ".originals", "trafficpeak", "default_shared", "1.0.6")
        assert originals_path == expected
        assert os.path.isfile(os.path.join(originals_path, "dashboards", "overview.json"))
        assert os.path.isfile(os.path.join(originals_path, "transforms", "ingest.sql"))
