"""Tests for dictionary dependency discovery in Track 1 bundle formatting."""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))

from configurator.bundle_json_builder import run_bundle_json_build
from configurator.config import BundleConfig, BundleState, TransformInfo
from configurator.sql_analyzer import run_sql_analysis
from converters.discoverer import AssetDiscoverer


def test_sql_analyzer_records_full_required_dictionary_names(tmp_path):
    bundle_dir = tmp_path / "trafficpeak" / "bot_insights_ds2"
    transform_dir = bundle_dir / "transformations" / "akamai_default_datastream"
    transform_dir.mkdir(parents=True)
    transform_path = transform_dir / "transform.json"
    transform_path.write_text(json.dumps({
        "name": "akamai_default_datastream",
        "settings": {
            "sql_transform": (
                "SELECT "
                "dictGet('akamai_ua_cat_dict', 'ua_category', UA), "
                "dictGetOrDefault('akamai_asn_type_dict', 'asn_type', asn, 'unknown'), "
                "dictGetOrDefault('akamai_bot_ip_dict', 'bot_owner', cliIP, '') "
                "FROM {STREAM}"
            ),
            "output_columns": [],
        },
    }))

    config = BundleConfig(
        bundle_dir=str(bundle_dir),
        table_name="logs",
        data_category="security",
        source_name="trafficpeak",
        bundle_name="bot_insights_ds2",
        method="http_streaming",
    )
    state = BundleState(
        transforms=[TransformInfo(
            original_path=str(transform_path),
            final_path=str(transform_path),
            final_dir=str(transform_dir),
            sample_data_path=str(transform_dir / "sample_data.json"),
        )]
    )

    assert run_sql_analysis(config, state) is True
    assert state.all_shared_dictionaries == [
        "akamai_asn_type_dict",
        "akamai_bot_ip_dict",
        "akamai_ua_cat_dict",
    ]

    assert run_bundle_json_build(config, state) is True
    bundle_json = json.loads((bundle_dir / "bundle.json").read_text())
    hydrolix_deps = bundle_json["dependencies"]["hydrolix"]
    assert hydrolix_deps["required_dictionaries"] == [
        "akamai_asn_type_dict",
        "akamai_bot_ip_dict",
        "akamai_ua_cat_dict",
    ]
    assert hydrolix_deps["shared_dictionaries"] == []


def test_raw_dictionary_folders_are_discovered_without_bundle_json(tmp_path):
    bundle_dir = tmp_path / "trafficpeak" / "bot_insights_ds2"
    dictionaries_dir = bundle_dir / "dictionaries"
    for name in (
        "akamai_asn_type_dict",
        "akamai_bot_ip_dict",
        "akamai_ua_cat_dict",
    ):
        dict_dir = dictionaries_dir / name
        dict_dir.mkdir(parents=True)
        (dict_dir / "schema_definition.json").write_text(json.dumps({"name": name}))

    assets = AssetDiscoverer(bundle_dir).discover()

    assert [dictionary.name for dictionary in assets.dictionaries] == [
        "akamai_asn_type_dict",
        "akamai_bot_ip_dict",
        "akamai_ua_cat_dict",
    ]
