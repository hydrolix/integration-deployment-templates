"""Tests for sync_cluster_deps.py — cluster dependency sync stage."""

import json
import os
import sys
import types

import pytest

# Stub out utils.file_utils before importing scripts modules
_utils_pkg = types.ModuleType("utils")
_utils_pkg.file_utils = types.ModuleType("utils.file_utils")


def _stub_read_json(path, *a, **kw):
    with open(path) as f:
        return json.load(f)


_utils_pkg.file_utils.read_json = _stub_read_json
_utils_pkg.file_utils.write_json = lambda *a, **kw: None
sys.modules.setdefault("utils", _utils_pkg)
sys.modules.setdefault("utils.file_utils", _utils_pkg.file_utils)

from scripts.sync_cluster_deps import (
    collect_dependencies,
    derive_project_name,
    find_local_function,
    find_local_dictionary,
    strip_project_prefix,
    _build_dict_definition,
    _extract_list,
)


# ---------------------------------------------------------------------------
# collect_dependencies
# ---------------------------------------------------------------------------

class TestCollectDependencies:
    def test_collects_all_four_arrays(self, tmp_path):
        bundle_json = tmp_path / "bundle.json"
        bundle_json.write_text(json.dumps({
            "dependencies": {
                "hydrolix": {
                    "required_functions": ["func_a"],
                    "shared_functions": ["func_b", "func_c"],
                    "required_dictionaries": ["dict_a"],
                    "shared_dictionaries": ["dict_b"],
                }
            }
        }))
        funcs, dicts = collect_dependencies(str(bundle_json))
        assert funcs == {"func_a", "func_b", "func_c"}
        assert dicts == {"dict_a", "dict_b"}

    def test_handles_empty_deps(self, tmp_path):
        bundle_json = tmp_path / "bundle.json"
        bundle_json.write_text(json.dumps({
            "dependencies": {"hydrolix": {}}
        }))
        funcs, dicts = collect_dependencies(str(bundle_json))
        assert funcs == set()
        assert dicts == set()

    def test_handles_missing_hydrolix_key(self, tmp_path):
        bundle_json = tmp_path / "bundle.json"
        bundle_json.write_text(json.dumps({"dependencies": {}}))
        funcs, dicts = collect_dependencies(str(bundle_json))
        assert funcs == set()
        assert dicts == set()

    def test_deduplicates_across_required_and_shared(self, tmp_path):
        bundle_json = tmp_path / "bundle.json"
        bundle_json.write_text(json.dumps({
            "dependencies": {
                "hydrolix": {
                    "required_functions": ["city_name"],
                    "shared_functions": ["city_name", "breadcrumbs"],
                }
            }
        }))
        funcs, _ = collect_dependencies(str(bundle_json))
        assert funcs == {"city_name", "breadcrumbs"}


# ---------------------------------------------------------------------------
# derive_project_name
# ---------------------------------------------------------------------------

class TestDeriveProjectName:
    def test_aws_maps_to_commons(self):
        assert derive_project_name("aws/cdn-insights") == "commons"

    def test_trafficpeak_maps_to_akamai(self):
        assert derive_project_name("trafficpeak/bot-insights-cdn") == "akamai"

    def test_strips_leading_slashes(self):
        assert derive_project_name("/aws/foo") == "commons"

    def test_unknown_vendor_raises(self):
        with pytest.raises(ValueError, match="Cannot derive project name"):
            derive_project_name("azure/something")


# ---------------------------------------------------------------------------
# strip_project_prefix
# ---------------------------------------------------------------------------

class TestStripProjectPrefix:
    def test_strips_prefix(self):
        existing = {"commons_city_name", "commons_breadcrumbs"}
        bases = strip_project_prefix(existing, "commons")
        assert "city_name" in bases
        assert "breadcrumbs" in bases
        # Also keeps the full name
        assert "commons_city_name" in bases

    def test_handles_unprefixed_names(self):
        existing = {"city_name"}
        bases = strip_project_prefix(existing, "commons")
        assert "city_name" in bases

    def test_mixed_prefixed_and_unprefixed(self):
        existing = {"commons_func_a", "func_b"}
        bases = strip_project_prefix(existing, "commons")
        assert "func_a" in bases
        assert "func_b" in bases


# ---------------------------------------------------------------------------
# find_local_function
# ---------------------------------------------------------------------------

class TestFindLocalFunction:
    def test_finds_json_in_functions_dir(self, tmp_path):
        funcs_dir = tmp_path / "functions"
        funcs_dir.mkdir()
        func_file = funcs_dir / "city_name.json"
        func_file.write_text(json.dumps({"name": "city_name", "sql": "() -> 1"}))

        result = find_local_function(str(tmp_path), "city_name")
        assert result == str(func_file)

    def test_finds_json_in_extracted_dir(self, tmp_path):
        extracted = tmp_path / "functions" / ".extracted"
        extracted.mkdir(parents=True)
        func_file = extracted / "my_func.json"
        func_file.write_text(json.dumps({"name": "my_func", "sql": "() -> 1"}))

        result = find_local_function(str(tmp_path), "my_func")
        assert result == str(func_file)

    def test_returns_none_when_missing(self, tmp_path):
        result = find_local_function(str(tmp_path), "nonexistent")
        assert result is None

    def test_ignores_sql_files(self, tmp_path):
        """Only .json files are uploadable — .sql files are raw assets."""
        funcs_dir = tmp_path / "functions"
        funcs_dir.mkdir()
        (funcs_dir / "breadcrumbs.sql").write_text("(x) -> x")

        result = find_local_function(str(tmp_path), "breadcrumbs")
        assert result is None


# ---------------------------------------------------------------------------
# find_local_dictionary
# ---------------------------------------------------------------------------

class TestFindLocalDictionary:
    def test_finds_flat_json_plus_csv(self, tmp_path):
        dicts_dir = tmp_path / "dictionaries"
        dicts_dir.mkdir()
        (dicts_dir / "geoip.json").write_text(json.dumps({
            "name": "geoip", "settings": {"filename": "geoip"}
        }))
        (dicts_dir / "geoip.csv").write_text("network,asn\n1.0.0.0/8,13335")

        result = find_local_dictionary(str(tmp_path), "geoip")
        assert result is not None
        assert result[0].endswith("geoip.json")
        assert result[1].endswith("geoip.csv")

    def test_finds_subdirectory_schema_layout(self, tmp_path):
        subdir = tmp_path / "dictionaries" / "geoip_asn"
        subdir.mkdir(parents=True)
        (subdir / "schema_definition.json").write_text(json.dumps([
            {"name": "network", "datatype": {"type": "string"}}
        ]))
        (subdir / "geoip_asn.csv").write_text("network\n1.0.0.0/8")

        result = find_local_dictionary(str(tmp_path), "geoip_asn")
        assert result is not None
        assert result[0].endswith("schema_definition.json")
        assert result[1].endswith("geoip_asn.csv")

    def test_returns_none_when_no_data_file(self, tmp_path):
        dicts_dir = tmp_path / "dictionaries"
        dicts_dir.mkdir()
        (dicts_dir / "lonely.json").write_text(json.dumps({"name": "lonely", "settings": {}}))

        result = find_local_dictionary(str(tmp_path), "lonely")
        assert result is None

    def test_returns_none_when_missing(self, tmp_path):
        result = find_local_dictionary(str(tmp_path), "nonexistent")
        assert result is None

    def test_finds_yaml_data_file(self, tmp_path):
        dicts_dir = tmp_path / "dictionaries"
        dicts_dir.mkdir()
        (dicts_dir / "ua_cat.json").write_text(json.dumps({
            "name": "ua_cat", "settings": {"filename": "ua_cat"}
        }))
        (dicts_dir / "ua_cat.yml").write_text("key: value")

        result = find_local_dictionary(str(tmp_path), "ua_cat")
        assert result is not None
        assert result[1].endswith("ua_cat.yml")


# ---------------------------------------------------------------------------
# _build_dict_definition
# ---------------------------------------------------------------------------

class TestBuildDictDefinition:
    def test_full_definition_preserves_settings(self, tmp_path):
        defn = {
            "name": "old_name",
            "settings": {
                "filename": "geoip",
                "layout": "ip_trie",
                "output_columns": [{"name": "net", "datatype": {"type": "string"}}],
            }
        }
        path = tmp_path / "def.json"
        path.write_text(json.dumps(defn))

        result = _build_dict_definition(str(path), "new_name")
        assert result["name"] == "new_name"
        assert result["settings"]["layout"] == "ip_trie"

    def test_schema_array_wrapped(self, tmp_path):
        schema = [
            {"name": "network", "datatype": {"type": "string"}},
            {"name": "asn", "datatype": {"type": "uint32"}},
        ]
        path = tmp_path / "schema_definition.json"
        path.write_text(json.dumps(schema))

        result = _build_dict_definition(str(path), "my_dict")
        assert result["name"] == "my_dict"
        assert result["settings"]["filename"] == "my_dict"
        assert len(result["settings"]["output_columns"]) == 2


# ---------------------------------------------------------------------------
# _extract_list
# ---------------------------------------------------------------------------

class TestExtractList:
    def test_direct_array(self):
        assert _extract_list([1, 2, 3], []) == [1, 2, 3]

    def test_object_with_results_key(self):
        assert _extract_list({"results": [1, 2]}, ["results"]) == [1, 2]

    def test_object_with_functions_key(self):
        assert _extract_list({"functions": [{"name": "a"}]}, ["functions"]) == [{"name": "a"}]

    def test_object_with_data_key(self):
        assert _extract_list({"data": [1]}, ["results", "data"]) == [1]

    def test_empty_fallback(self):
        assert _extract_list({"unknown": [1]}, ["results"]) == []

    def test_none_in_dict(self):
        assert _extract_list({"results": None}, ["results"]) == []


# ---------------------------------------------------------------------------
# Pipeline integration (run_pipeline.py)
# ---------------------------------------------------------------------------

class TestPipelineIntegration:
    def test_sync_stage_included_when_env_set(self, monkeypatch):
        monkeypatch.setenv("BUNDLE_TESTING_CLUSTER", "test.cluster.dev")
        monkeypatch.setattr(
            "sys.argv",
            ["run_pipeline.py", "--bundle-dir", "aws/test-bundle",
             "--table-name", "t", "--data-category", "cdn"],
        )
        from scripts.run_pipeline import parse_args, resolve_stages
        args = parse_args()
        stages = resolve_stages(args)
        stage_names = [name for name, _ in stages]
        assert "sync" in stage_names
        assert stage_names.index("sync") < stage_names.index("validate")

    def test_sync_stage_excluded_when_no_env(self, monkeypatch):
        monkeypatch.delenv("BUNDLE_TESTING_CLUSTER", raising=False)
        monkeypatch.setattr(
            "sys.argv",
            ["run_pipeline.py", "--bundle-dir", "aws/test-bundle",
             "--table-name", "t", "--data-category", "cdn"],
        )
        from scripts.run_pipeline import parse_args, resolve_stages
        args = parse_args()
        stages = resolve_stages(args)
        stage_names = [name for name, _ in stages]
        assert "sync" not in stage_names

    def test_skip_sync_flag(self, monkeypatch):
        monkeypatch.setenv("BUNDLE_TESTING_CLUSTER", "test.cluster.dev")
        monkeypatch.setattr(
            "sys.argv",
            ["run_pipeline.py", "--bundle-dir", "aws/test-bundle",
             "--table-name", "t", "--data-category", "cdn", "--skip-sync"],
        )
        from scripts.run_pipeline import parse_args, resolve_stages
        args = parse_args()
        stages = resolve_stages(args)
        stage_names = [name for name, _ in stages]
        assert "sync" not in stage_names

    def test_only_sync_flag(self, monkeypatch):
        monkeypatch.setattr(
            "sys.argv",
            ["run_pipeline.py", "--bundle-dir", "aws/test-bundle", "--only-sync"],
        )
        from scripts.run_pipeline import parse_args, resolve_stages
        args = parse_args()
        stages = resolve_stages(args)
        stage_names = [name for name, _ in stages]
        assert stage_names == ["sync"]

    def test_validate_only_track_includes_sync_when_env_set(self, monkeypatch):
        monkeypatch.setenv("BUNDLE_TESTING_CLUSTER", "test.cluster.dev")
        monkeypatch.setattr(
            "sys.argv",
            ["run_pipeline.py", "--bundle-dir", "aws/test-bundle",
             "--track", "validate-only"],
        )
        from scripts.run_pipeline import parse_args, resolve_stages, _apply_track
        args = parse_args()
        _apply_track(args)
        stages = resolve_stages(args)
        stage_names = [name for name, _ in stages]
        assert "sync" in stage_names
        assert "validate" in stage_names
        assert "portable" not in stage_names
        assert "configure" not in stage_names

    def test_validate_only_track_without_env_skips_sync(self, monkeypatch):
        monkeypatch.delenv("BUNDLE_TESTING_CLUSTER", raising=False)
        monkeypatch.setattr(
            "sys.argv",
            ["run_pipeline.py", "--bundle-dir", "aws/test-bundle",
             "--track", "validate-only"],
        )
        from scripts.run_pipeline import parse_args, resolve_stages, _apply_track
        args = parse_args()
        _apply_track(args)
        stages = resolve_stages(args)
        stage_names = [name for name, _ in stages]
        assert stage_names == ["validate"]
