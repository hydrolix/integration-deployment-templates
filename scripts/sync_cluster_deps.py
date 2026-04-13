#!/usr/bin/env python3
"""Sync missing functions/dictionaries to a Hydrolix cluster.

Reads bundle.json to determine which functions and dictionaries are needed,
queries the target cluster to see what already exists, and uploads any missing
resources that have local files in the bundle directory.

Designed to run as a pipeline stage between configure and validate.
Skips silently if cluster env vars are not set.

Usage:
    python scripts/sync_cluster_deps.py \
        --bundle-dir trafficpeak/bot-insights-cdn \
        [--verbose] [--json] [--dry-run]
"""

import argparse
import json
import os
import sys
import urllib.error
import urllib.request
import uuid

SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(SCRIPTS_DIR)

sys.path.insert(0, SCRIPTS_DIR)

from configurator.constants import PREFIX_MAP

HTTP_TIMEOUT = 120
DATA_FILE_EXTENSIONS = ("csv", "yaml", "yml", "tsv")


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def parse_args():
    parser = argparse.ArgumentParser(
        description="Sync missing functions/dictionaries to a Hydrolix cluster.",
    )
    parser.add_argument(
        "--bundle-dir", required=True,
        help="Path to bundle directory (relative to repo root)",
    )
    parser.add_argument("--verbose", action="store_true")
    parser.add_argument("--json", action="store_true", help="Output structured JSON report")
    parser.add_argument("--dry-run", action="store_true",
                        help="Report what would be synced without uploading")
    return parser.parse_args()


# ---------------------------------------------------------------------------
# HTTP helpers
# ---------------------------------------------------------------------------

def _api_request(url, token, method="GET", body=None, content_type="application/json"):
    """Make an authenticated API request. Returns parsed JSON or raises."""
    headers = {"Authorization": f"Bearer {token}", "Accept": "application/json"}
    data = None
    if body is not None:
        data = json.dumps(body).encode("utf-8")
        headers["Content-Type"] = content_type

    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=HTTP_TIMEOUT) as resp:
            resp_body = resp.read().decode("utf-8")
            return json.loads(resp_body) if resp_body.strip() else {}
    except urllib.error.HTTPError as e:
        error_body = ""
        try:
            error_body = e.read().decode("utf-8")[:500]
        except Exception:
            pass
        raise RuntimeError(f"HTTP {e.code} {e.reason}: {error_body}") from e


def _upload_multipart(url, token, field_name, file_name, file_bytes, mime_type):
    """Upload a file via multipart/form-data POST."""
    boundary = f"----SyncDeps{uuid.uuid4().hex}"
    body = bytearray()

    # name field
    body.extend(f"--{boundary}\r\n".encode())
    body.extend(f'Content-Disposition: form-data; name="name"\r\n\r\n'.encode())
    stem = os.path.splitext(file_name)[0]
    body.extend(f"{stem}\r\n".encode())

    # file field
    body.extend(f"--{boundary}\r\n".encode())
    body.extend(
        f'Content-Disposition: form-data; name="{field_name}"; filename="{file_name}"\r\n'.encode()
    )
    body.extend(f"Content-Type: {mime_type}\r\n\r\n".encode())
    body.extend(file_bytes)
    body.extend(b"\r\n")
    body.extend(f"--{boundary}--\r\n".encode())

    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": f"multipart/form-data; boundary={boundary}",
    }
    req = urllib.request.Request(url, data=bytes(body), headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=HTTP_TIMEOUT) as resp:
            resp_body = resp.read().decode("utf-8")
            try:
                return json.loads(resp_body) if resp_body.strip() else {}
            except json.JSONDecodeError:
                return {}
    except urllib.error.HTTPError as e:
        error_body = ""
        try:
            error_body = e.read().decode("utf-8")[:500]
        except Exception:
            pass
        raise RuntimeError(f"HTTP {e.code} {e.reason}: {error_body}") from e


def _extract_list(response_data, key_hints):
    """Extract a list from an API response that may be an array or an object
    with a known key containing the array (handles inconsistent Hydrolix API shapes)."""
    if isinstance(response_data, list):
        return response_data
    if isinstance(response_data, dict):
        for key in key_hints:
            if key in response_data and isinstance(response_data[key], list):
                return response_data[key]
    return []


# ---------------------------------------------------------------------------
# Cluster interaction
# ---------------------------------------------------------------------------

def authenticate(cluster, username, password):
    """Authenticate and return bearer token."""
    url = f"https://{cluster}/config/v1/login"
    body = json.dumps({"username": username, "password": password}).encode("utf-8")
    req = urllib.request.Request(
        url, data=body, headers={"Content-Type": "application/json"}, method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=HTTP_TIMEOUT) as resp:
            data = json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        error_body = ""
        try:
            error_body = e.read().decode("utf-8")[:500]
        except Exception:
            pass
        raise RuntimeError(f"HTTP {e.code} {e.reason}: {error_body}") from e

    if "auth_token" not in data or "access_token" not in data.get("auth_token", {}):
        raise RuntimeError(f"Unexpected login response (no auth_token): {json.dumps(data)[:200]}")
    return data["auth_token"]["access_token"]


def find_org_uuid(cluster, token):
    """Get the first org UUID from the cluster."""
    url = f"https://{cluster}/config/v1/orgs/"
    orgs = _api_request(url, token)
    org_list = _extract_list(orgs, ["results", "data"])
    if not org_list:
        # Response might be the org directly or a single-item list
        if isinstance(orgs, dict) and "uuid" in orgs:
            return orgs["uuid"]
        raise RuntimeError("No organizations found on cluster")
    return org_list[0]["uuid"]


def find_or_create_project(cluster, token, org_uuid, project_name):
    """Find a project by name, or create it. Returns (uuid, created_bool)."""
    url = f"https://{cluster}/config/v1/orgs/{org_uuid}/projects/"
    projects = _api_request(url, token)
    project_list = _extract_list(projects, ["results", "data"])

    for proj in project_list:
        if proj.get("name") == project_name:
            return proj["uuid"], False

    # Create the project (handle 409 race condition from parallel runs)
    body = {"name": project_name, "description": "Auto-created by sync_cluster_deps for bundle testing"}
    try:
        result = _api_request(url, token, method="POST", body=body)
        return result["uuid"], True
    except RuntimeError as e:
        if "409" in str(e):
            # Another process created the project — re-fetch
            projects = _api_request(url, token)
            project_list = _extract_list(projects, ["results", "data"])
            for proj in project_list:
                if proj.get("name") == project_name:
                    return proj["uuid"], False
        raise


def list_existing_resources(cluster, token, org_uuid, proj_uuid, resource_type):
    """List existing functions or dictionaries on the cluster.
    Returns a set of base names (with project prefix stripped)."""
    url = f"https://{cluster}/config/v1/orgs/{org_uuid}/projects/{proj_uuid}/{resource_type}/"
    response = _api_request(url, token)
    items = _extract_list(response, ["results", resource_type, "data"])
    names = set()
    for item in items:
        name = item.get("name", "") if isinstance(item, dict) else ""
        if name:
            names.add(name)
    return names


def strip_project_prefix(names, project_name):
    """Given a set of names that may be prefixed with '{project_name}_',
    return a set of base names (both prefixed and unprefixed forms)."""
    base_names = set()
    prefix = f"{project_name}_"
    for name in names:
        base_names.add(name)
        if name.startswith(prefix):
            base_names.add(name[len(prefix):])
    return base_names


# ---------------------------------------------------------------------------
# Dependency collection
# ---------------------------------------------------------------------------

def collect_dependencies(bundle_json_path):
    """Read bundle.json and return (functions_needed, dictionaries_needed) as sets."""
    with open(bundle_json_path, "r", encoding="utf-8") as f:
        bundle = json.load(f)

    deps = bundle.get("dependencies", {}).get("hydrolix", {})
    functions = set()
    dictionaries = set()

    for key in ("required_functions", "shared_functions"):
        functions.update(deps.get(key) or [])
    for key in ("required_dictionaries", "shared_dictionaries"):
        dictionaries.update(deps.get(key) or [])

    return functions, dictionaries


# ---------------------------------------------------------------------------
# Local file discovery
# ---------------------------------------------------------------------------

def find_local_function(bundle_dir, name):
    """Look for a function JSON file in the bundle. Returns path or None."""
    for subdir in ("functions/.extracted", "functions"):
        path = os.path.join(bundle_dir, subdir, f"{name}.json")
        if os.path.isfile(path):
            return path
    return None


def find_local_dictionary(bundle_dir, name):
    """Look for a dictionary definition + data file in the bundle.
    Returns (definition_path, data_path) or None.

    Handles two layouts:
    1. Flat: dictionaries/{name}.json + dictionaries/{name}.csv
    2. Subdirectory: dictionaries/{name}/schema_definition.json + dictionaries/{name}/{name}.csv
    """
    for subdir in ("dictionaries/.extracted", "dictionaries"):
        base = os.path.join(bundle_dir, subdir)

        # Layout 1: flat files
        json_path = os.path.join(base, f"{name}.json")
        if os.path.isfile(json_path):
            for ext in DATA_FILE_EXTENSIONS:
                data_path = os.path.join(base, f"{name}.{ext}")
                if os.path.isfile(data_path):
                    return json_path, data_path

        # Layout 2: subdirectory with schema_definition.json
        subdir_path = os.path.join(base, name)
        schema_path = os.path.join(subdir_path, "schema_definition.json")
        if os.path.isfile(schema_path):
            # Try exact name match first
            for ext in DATA_FILE_EXTENSIONS:
                data_path = os.path.join(subdir_path, f"{name}.{ext}")
                if os.path.isfile(data_path):
                    return schema_path, data_path
            # Fall back: scan for any data file with a matching extension
            if os.path.isdir(subdir_path):
                for entry in os.listdir(subdir_path):
                    if any(entry.endswith(f".{ext}") for ext in DATA_FILE_EXTENSIONS):
                        return schema_path, os.path.join(subdir_path, entry)

    return None


def _build_dict_definition(schema_path, name):
    """Build a dictionary definition payload from either a full definition JSON
    or a schema_definition.json (array of columns)."""
    with open(schema_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    # Full definition: has 'name' and 'settings' keys
    if isinstance(data, dict) and "settings" in data:
        data["name"] = name
        return data

    # Schema-only: array of column definitions — wrap into full definition
    if isinstance(data, list):
        return {
            "name": name,
            "settings": {
                "filename": name,
                "output_columns": data,
            },
        }

    raise ValueError(f"Unexpected dictionary definition format in {schema_path}")


# ---------------------------------------------------------------------------
# Upload
# ---------------------------------------------------------------------------

def upload_function(cluster, token, org_uuid, proj_uuid, project_name, name, json_path):
    """Upload a function to the cluster. Returns True on success."""
    with open(json_path, "r", encoding="utf-8") as f:
        func_def = json.load(f)

    # Replace __PROJECT_NAME__ in SQL
    if "sql" in func_def:
        func_def["sql"] = func_def["sql"].replace("__PROJECT_NAME__", project_name)
    func_def["name"] = name

    url = f"https://{cluster}/config/v1/orgs/{org_uuid}/projects/{proj_uuid}/functions/"
    _api_request(url, token, method="POST", body=func_def)
    return True


def upload_dictionary(cluster, token, org_uuid, proj_uuid, project_name, name, def_path, data_path):
    """Upload a dictionary (data file first, then definition). Returns True on success."""
    # Upload data file
    data_file_name = os.path.basename(data_path)
    ext = os.path.splitext(data_file_name)[1].lower().lstrip(".")
    mime_type = "application/x-yaml" if ext in ("yaml", "yml") else "text/csv"

    with open(data_path, "rb") as f:
        file_bytes = f.read()

    files_url = f"https://{cluster}/config/v1/orgs/{org_uuid}/projects/{proj_uuid}/dictionaries/files/"
    _upload_multipart(files_url, token, "file", data_file_name, file_bytes, mime_type)

    # Upload definition
    definition = _build_dict_definition(def_path, name)
    dict_url = f"https://{cluster}/config/v1/orgs/{org_uuid}/projects/{proj_uuid}/dictionaries/"
    _api_request(dict_url, token, method="POST", body=definition)
    return True


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def derive_project_name(bundle_dir):
    """Derive the cluster project name from the bundle path.
    aws/* -> commons, trafficpeak/* -> akamai."""
    # Normalize: strip leading/trailing slashes
    parts = bundle_dir.strip("/").split("/")
    vendor = parts[0] if parts else ""
    project = PREFIX_MAP.get(vendor)
    if not project:
        raise ValueError(
            f"Cannot derive project name from bundle path '{bundle_dir}'. "
            f"Expected path starting with one of: {', '.join(PREFIX_MAP.keys())}"
        )
    return project


def main():
    args = parse_args()
    verbose = args.verbose
    dry_run = args.dry_run

    # Resolve bundle directory
    bundle_dir = args.bundle_dir
    if not os.path.isabs(bundle_dir):
        bundle_dir = os.path.join(REPO_ROOT, bundle_dir)

    bundle_json_path = os.path.join(bundle_dir, "bundle.json")
    if not os.path.isfile(bundle_json_path):
        print("No bundle.json found — skipping dependency sync", file=sys.stderr)
        sys.exit(0)

    # Collect dependencies
    functions_needed, dicts_needed = collect_dependencies(bundle_json_path)
    if not functions_needed and not dicts_needed:
        if verbose:
            print("No dependencies declared — nothing to sync", file=sys.stderr)
        _output_report(args, skipped_reason="no_dependencies")
        sys.exit(0)

    if verbose:
        print(f"Dependencies needed: {len(functions_needed)} function(s), "
              f"{len(dicts_needed)} dictionary(s)", file=sys.stderr)

    # Check env vars
    cluster = os.environ.get("BUNDLE_TESTING_CLUSTER", "")
    username = os.environ.get("BUNDLE_TESTING_USERNAME", "")
    password = os.environ.get("BUNDLE_TESTING_PASSWORD", "")

    if not cluster:
        if verbose:
            print("BUNDLE_TESTING_CLUSTER not set — skipping dependency sync", file=sys.stderr)
        _output_report(args, skipped_reason="no_cluster_env")
        sys.exit(0)

    if not username or not password:
        missing = []
        if not username:
            missing.append("BUNDLE_TESTING_USERNAME")
        if not password:
            missing.append("BUNDLE_TESTING_PASSWORD")
        print(f"BUNDLE_TESTING_CLUSTER is set but {', '.join(missing)} missing — "
              f"skipping dependency sync", file=sys.stderr)
        _output_report(args, skipped_reason="missing_credentials")
        sys.exit(0)

    # Derive project name from bundle path (use repo-relative path)
    project_name = derive_project_name(os.path.relpath(bundle_dir, REPO_ROOT))

    if verbose:
        print(f"Target: {cluster} / project: {project_name}", file=sys.stderr)

    # Authenticate
    try:
        token = authenticate(cluster, username, password)
    except Exception as e:
        print(f"Authentication failed: {e}", file=sys.stderr)
        _output_report(args, error=f"auth_failed: {e}")
        sys.exit(1)

    if verbose:
        print("Authenticated successfully", file=sys.stderr)

    # Resolve org and project
    try:
        org_uuid = find_org_uuid(cluster, token)
        proj_uuid, proj_created = find_or_create_project(cluster, token, org_uuid, project_name)
        if verbose:
            if proj_created:
                print(f"Created project '{project_name}' ({proj_uuid})", file=sys.stderr)
            else:
                print(f"Found project '{project_name}' ({proj_uuid})", file=sys.stderr)
    except Exception as e:
        print(f"Failed to resolve org/project: {e}", file=sys.stderr)
        _output_report(args, error=f"project_resolution_failed: {e}")
        sys.exit(1)

    # List existing resources
    try:
        existing_funcs = list_existing_resources(cluster, token, org_uuid, proj_uuid, "functions")
        existing_dicts = list_existing_resources(cluster, token, org_uuid, proj_uuid, "dictionaries")
    except Exception as e:
        print(f"Failed to list existing resources: {e}", file=sys.stderr)
        _output_report(args, error=f"list_failed: {e}")
        sys.exit(1)

    existing_func_bases = strip_project_prefix(existing_funcs, project_name)
    existing_dict_bases = strip_project_prefix(existing_dicts, project_name)

    # Diff
    missing_funcs = functions_needed - existing_func_bases
    missing_dicts = dicts_needed - existing_dict_bases
    present_funcs = functions_needed - missing_funcs
    present_dicts = dicts_needed - missing_dicts

    if not missing_funcs and not missing_dicts:
        if verbose:
            print("All dependencies already present on cluster", file=sys.stderr)
        _output_report(args, present_funcs=present_funcs, present_dicts=present_dicts)
        sys.exit(0)

    if verbose:
        if missing_funcs:
            print(f"Missing functions: {', '.join(sorted(missing_funcs))}", file=sys.stderr)
        if missing_dicts:
            print(f"Missing dictionaries: {', '.join(sorted(missing_dicts))}", file=sys.stderr)

    # Discover local files and categorize
    uploadable_funcs = {}   # name -> path
    uploadable_dicts = {}   # name -> (def_path, data_path)
    warn_funcs = []
    warn_dicts = []

    for name in sorted(missing_funcs):
        path = find_local_function(bundle_dir, name)
        if path:
            uploadable_funcs[name] = path
        else:
            warn_funcs.append(name)

    for name in sorted(missing_dicts):
        result = find_local_dictionary(bundle_dir, name)
        if result:
            uploadable_dicts[name] = result
        else:
            warn_dicts.append(name)

    # Report warnings
    if warn_funcs:
        print(f"Warning: {len(warn_funcs)} function(s) missing on cluster with no local files:",
              file=sys.stderr)
        for name in warn_funcs:
            print(f"  - {name}", file=sys.stderr)

    if warn_dicts:
        print(f"Warning: {len(warn_dicts)} dictionary(s) missing on cluster with no local files:",
              file=sys.stderr)
        for name in warn_dicts:
            print(f"  - {name}", file=sys.stderr)

    if dry_run:
        if uploadable_dicts:
            print(f"[dry-run] Would upload {len(uploadable_dicts)} dictionary(s):",
                  file=sys.stderr)
            for name in uploadable_dicts:
                print(f"  - {name}", file=sys.stderr)
        if uploadable_funcs:
            print(f"[dry-run] Would upload {len(uploadable_funcs)} function(s):",
                  file=sys.stderr)
            for name in uploadable_funcs:
                print(f"  - {name}", file=sys.stderr)
        _output_report(
            args, present_funcs=present_funcs, present_dicts=present_dicts,
            uploadable_funcs=uploadable_funcs, uploadable_dicts=uploadable_dicts,
            warn_funcs=warn_funcs, warn_dicts=warn_dicts, dry_run=True,
        )
        sys.exit(0)

    # Upload — dictionaries first (functions may depend on them)
    uploaded_dicts = []
    failed_dicts = []
    for name, (def_path, data_path) in uploadable_dicts.items():
        try:
            upload_dictionary(cluster, token, org_uuid, proj_uuid, project_name,
                              name, def_path, data_path)
            uploaded_dicts.append(name)
            if verbose:
                print(f"  Uploaded dictionary: {name}", file=sys.stderr)
        except Exception as e:
            failed_dicts.append((name, str(e)))
            print(f"  Warning: failed to upload dictionary '{name}': {e}", file=sys.stderr)

    uploaded_funcs = []
    failed_funcs = []
    for name, path in uploadable_funcs.items():
        try:
            upload_function(cluster, token, org_uuid, proj_uuid, project_name, name, path)
            uploaded_funcs.append(name)
            if verbose:
                print(f"  Uploaded function: {name}", file=sys.stderr)
        except Exception as e:
            failed_funcs.append((name, str(e)))
            print(f"  Warning: failed to upload function '{name}': {e}", file=sys.stderr)

    # Summary
    _output_report(
        args, present_funcs=present_funcs, present_dicts=present_dicts,
        uploaded_funcs=uploaded_funcs, uploaded_dicts=uploaded_dicts,
        failed_funcs=failed_funcs, failed_dicts=failed_dicts,
        warn_funcs=warn_funcs, warn_dicts=warn_dicts,
    )

    # Print human-readable summary to stderr
    total_uploaded = len(uploaded_funcs) + len(uploaded_dicts)
    total_failed = len(failed_funcs) + len(failed_dicts)
    total_warned = len(warn_funcs) + len(warn_dicts)

    parts = []
    if total_uploaded:
        parts.append(f"{total_uploaded} uploaded")
    if present_funcs or present_dicts:
        parts.append(f"{len(present_funcs) + len(present_dicts)} already present")
    if total_warned:
        parts.append(f"{total_warned} not found locally")
    if total_failed:
        parts.append(f"{total_failed} failed")

    print(f"Sync complete: {', '.join(parts)}", file=sys.stderr)


def _output_report(args, **kwargs):
    """Output structured JSON report to stdout if --json is set."""
    if not args.json:
        return

    report = {"stage": "sync"}

    if "skipped_reason" in kwargs:
        report["status"] = "skipped"
        report["reason"] = kwargs["skipped_reason"]
    elif "error" in kwargs:
        report["status"] = "error"
        report["error"] = kwargs["error"]
    else:
        report["status"] = "success"
        report["present"] = {
            "functions": sorted(kwargs.get("present_funcs") or []),
            "dictionaries": sorted(kwargs.get("present_dicts") or []),
        }

        if kwargs.get("dry_run"):
            report["would_upload"] = {
                "functions": sorted(kwargs.get("uploadable_funcs") or {}),
                "dictionaries": sorted(kwargs.get("uploadable_dicts") or {}),
            }
        else:
            report["uploaded"] = {
                "functions": kwargs.get("uploaded_funcs", []),
                "dictionaries": kwargs.get("uploaded_dicts", []),
            }
            report["failed"] = {
                "functions": [{"name": n, "error": e}
                              for n, e in (kwargs.get("failed_funcs") or [])],
                "dictionaries": [{"name": n, "error": e}
                                 for n, e in (kwargs.get("failed_dicts") or [])],
            }

        report["not_found_locally"] = {
            "functions": sorted(kwargs.get("warn_funcs") or []),
            "dictionaries": sorted(kwargs.get("warn_dicts") or []),
        }

    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\nSync cancelled.", file=sys.stderr)
        sys.exit(130)
