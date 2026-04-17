#!/usr/bin/env python3
"""
Shared helper script for Claude skills: bundle-deploy and hdx-ingest-sample-data.

Usage:
  python3 .claude/skills/hdx_helpers.py <command> [args...]

Commands:
  get-jwt                    <cluster_url> <user> <pwd>
  extract-credentials        <cluster_url> [grafana_url]
  discover-table-auth        <cluster_url> <jwt> <project> <table>
  find-sample-data           <transform_name>
  ingest                     <cluster_url> <jwt> <project> <table> <transform> <n_rows> <start_date> <end_date> [ingest_token]
  auto-ingest                <cluster_url> <jwt> <project> <n_rows> <start_date> <end_date>
  validate-bundle            <bundle_name>
  clean-hdc-password         <cluster_url>
  clean-gfo-credentials      <grafana_url> <project_name>
  patch-grafana-ds           <grafana_url> <grafana_user> <grafana_pwd> <project> <cluster_url> <hdx_user> <hdx_pwd>
  validate-transforms-local  <cluster_host> <project_name>
  deploy-report              <action> <project_name> [args...]
                             Actions: init, update, finalize
"""

import glob
import json
import os
import random
import re
import ssl
import sys
import time
import datetime
import urllib.parse
import urllib.request
import urllib.error

# Disable SSL verification globally (dev clusters use self-signed certs)
_SSL_CTX = ssl.create_default_context()
_SSL_CTX.check_hostname = False
_SSL_CTX.verify_mode = ssl.CERT_NONE


def _urlopen(req):
    return urllib.request.urlopen(req, context=_SSL_CTX)


_TRANSIENT_HTTP_CODES = {502, 503, 504}


def _urlopen_with_retry(req, max_retries=2, backoff=1):
    """urlopen with retry on transient HTTP errors (502/503/504)."""
    for attempt in range(max_retries + 1):
        try:
            return _urlopen(req)
        except urllib.error.HTTPError as e:
            if e.code in _TRANSIENT_HTTP_CODES and attempt < max_retries:
                time.sleep(backoff * (attempt + 1))
                # Re-create the request since the body stream may be consumed
                req = urllib.request.Request(
                    req.full_url, data=req.data,
                    headers=dict(req.headers), method=req.get_method()
                )
                continue
            raise


# ---------------------------------------------------------------------------
# Utilities
# ---------------------------------------------------------------------------

def _api_get(url, jwt):
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {jwt}"})
    return json.loads(_urlopen(req).read())


def _resolve_project_and_org(cluster_url, jwt, project_name):
    """Return (org_uuid, project_dict)."""
    org_uuid = _api_get(f"{cluster_url}/config/v1/orgs/", jwt)["results"][0]["uuid"]
    projects = _api_get(f"{cluster_url}/config/v1/orgs/{org_uuid}/projects/", jwt).get("results", [])
    proj = next((p for p in projects if p["name"] == project_name), None)
    if not proj:
        print(f"ERROR: project '{project_name}' not found", file=sys.stderr)
        sys.exit(1)
    return org_uuid, proj


def _set_nested(obj, dotted_key, value):
    keys = dotted_key.split(".")
    for k in keys[:-1]:
        obj = obj.setdefault(k, {})
    obj[keys[-1]] = value


# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------

def cmd_get_jwt(cluster_url, user, pwd):
    """Print JWT access token."""
    data = json.dumps({"username": user, "password": pwd}).encode()
    req = urllib.request.Request(
        f"{cluster_url}/config/v1/login", data=data,
        headers={"Content-Type": "application/json"}, method="POST"
    )
    try:
        resp = json.loads(_urlopen(req).read())
        print(resp["auth_token"]["access_token"])
    except urllib.error.HTTPError as e:
        print(f"ERROR: login failed HTTP {e.code}: {e.read().decode()}", file=sys.stderr)
        sys.exit(1)


def cmd_extract_credentials(cluster_url, grafana_url=None):
    """
    Print credentials from env vars.
    Output: hdx_user, hdx_pwd [, grafana_user, grafana_pwd]
    """
    raw_hdx = os.environ.get("CAC_HYDROLIX_CREDENTIALS")
    if not raw_hdx:
        print("ERROR: CAC_HYDROLIX_CREDENTIALS not set", file=sys.stderr)
        sys.exit(1)

    hdx_host = urllib.parse.urlparse(cluster_url).netloc
    c = json.loads(raw_hdx).get(hdx_host, {})
    u = c.get("user", "")
    p = c.get("password") or c.get("pwd", "")
    if not u or not p:
        print(f"ERROR: no HDX credentials for {hdx_host}", file=sys.stderr)
        sys.exit(1)
    print(u)
    print(p)

    if grafana_url:
        raw_grf = os.environ.get("CAC_GRAFANA_CREDENTIALS")
        if not raw_grf:
            print("ERROR: CAC_GRAFANA_CREDENTIALS not set", file=sys.stderr)
            sys.exit(1)
        grf_host = urllib.parse.urlparse(grafana_url).netloc
        g = json.loads(raw_grf).get(grf_host, {})
        gu = g.get("user", "")
        gp = g.get("password") or g.get("pwd", "")
        if not gu or not gp:
            print(f"ERROR: no Grafana credentials for {grf_host}", file=sys.stderr)
            sys.exit(1)
        print(gu)
        print(gp)


def cmd_discover_table_auth(cluster_url, jwt, project_name, table_name):
    """
    Print TOKEN_AUTH and INGEST_TOKEN for a table.
    """
    org_uuid, proj = _resolve_project_and_org(cluster_url, jwt, project_name)
    tables = _api_get(
        f"{cluster_url}/config/v1/orgs/{org_uuid}/projects/{proj['uuid']}/tables/", jwt
    ).get("results", [])
    table = next((t for t in tables if t["name"] == table_name), None)
    if not table:
        print(f"ERROR: table '{table_name}' not found in project '{project_name}'", file=sys.stderr)
        sys.exit(1)

    stream = table.get("settings", {}).get("stream", {})
    token_auth = bool(stream.get("token_auth_enabled"))
    ingest_token = (stream.get("token_list") or [""])[0]
    print(f"TOKEN_AUTH={token_auth}")
    print(f"INGEST_TOKEN={ingest_token}")


def cmd_find_sample_data(transform_name):
    """
    Locate sample_data and primary timestamp field for a transform.
    Outputs: TIMESTAMP_FIELD=... and SAMPLE_SOURCE=...
    Writes sample data to /tmp/hdx_sample_data.json.
    """
    sample_data = None
    sample_source = None

    # 1. Dedicated sample_data file
    for pattern in [
        f"data/bundles/**/*{transform_name}*sample_data*.json",
        f"data/bundles/**/*sample*{transform_name}*.json",
    ]:
        files = glob.glob(pattern, recursive=True)
        if files:
            with open(files[0]) as f:
                sample_data = json.load(f)
            sample_source = files[0]
            break

    # 2. Embedded inside transform JSON
    transform_file = None
    transform_files = glob.glob(f"data/bundles/**/{transform_name}.json", recursive=True)
    if transform_files:
        transform_file = transform_files[0]

    if sample_data is None:
        if not transform_file:
            print(f"ERROR: no transform file found for '{transform_name}'", file=sys.stderr)
            sys.exit(1)
        with open(transform_file) as f:
            t = json.load(f)
        sample_data = t.get("settings", {}).get("sample_data")
        if not sample_data:
            print(f"ERROR: no sample_data in transform '{transform_name}'", file=sys.stderr)
            sys.exit(1)
        sample_source = f"embedded in {transform_file}"

    # 3. Detect primary timestamp field
    timestamp_field = _detect_timestamp_field(transform_file, sample_data)
    if not timestamp_field:
        print("ERROR: could not detect primary timestamp field", file=sys.stderr)
        sys.exit(1)

    sample_path = f"/tmp/hdx_sample_data_{os.getpid()}.json"
    with open(sample_path, "w") as f:
        json.dump(sample_data, f)

    ts_path = f"/tmp/hdx_ts_field_{os.getpid()}"
    with open(ts_path, "w") as f:
        f.write(timestamp_field)

    print(f"TIMESTAMP_FIELD={timestamp_field}")
    print(f"SAMPLE_SOURCE={sample_source}")


def _detect_timestamp_field(transform_file, sample_data):
    ts_field = None
    if transform_file:
        with open(transform_file) as f:
            t = json.load(f)
        for col in t.get("settings", {}).get("output_columns", t.get("output_columns", [])):
            dt = col.get("datatype", {})
            if dt.get("primary"):
                src = dt.get("source", "")
                if isinstance(src, dict):
                    pointers = src.get("from_json_pointers", [])
                    ts_field = ".".join(pointers[0].strip("/").split("/")) if pointers else col["name"]
                elif isinstance(src, str) and src.startswith("/"):
                    ts_field = ".".join(src.strip("/").split("/"))
                else:
                    ts_field = col["name"]
                break

    if not ts_field:
        def find_key(obj, keys, prefix=""):
            if isinstance(obj, dict):
                for k, v in obj.items():
                    path = f"{prefix}.{k}".lstrip(".")
                    if k.lower() in keys:
                        return path
                    result = find_key(v, keys, path)
                    if result:
                        return result
            return None
        ts_field = find_key(sample_data, {"reqtimesec", "start", "timestamp", "time", "ts"})

    return ts_field


def cmd_ingest(cluster_url, jwt, project_name, table_name, transform_name,
               n_rows, start_date, end_date, ingest_token=""):
    """Ingest N rows into a specific table."""
    start_ts = int(datetime.datetime.fromisoformat(start_date).replace(tzinfo=datetime.timezone.utc).timestamp())
    # Cap at start of today UTC (midnight) — ensures timestamps are always
    # "today or yesterday" regardless of the user's local timezone offset.
    _today_midnight_ts = int(datetime.datetime.combine(
        datetime.date.today(), datetime.time.min, tzinfo=datetime.timezone.utc
    ).timestamp())
    end_ts = min(
        int(datetime.datetime.fromisoformat(end_date).replace(
            hour=23, minute=59, second=59, tzinfo=datetime.timezone.utc).timestamp()),
        _today_midnight_ts,
    )

    sample_path = f"/tmp/hdx_sample_data_{os.getpid()}.json"
    with open(sample_path) as f:
        sample_data = json.load(f)

    headers = {"Authorization": f"Bearer {jwt}", "Content-Type": "application/json"}
    if ingest_token:
        headers["x-hdx-token"] = ingest_token

    url = f"{cluster_url}/ingest/event?table={project_name}.{table_name}&transform={transform_name}"
    step = (end_ts - start_ts) // max(n_rows - 1, 1)
    ok = 0
    for i in range(n_rows):
        row = json.loads(json.dumps(sample_data))
        ts = min(start_ts + i * step + random.randint(0, max(step - 1, 0)), end_ts)
        _set_nested(row, _load_timestamp_field(), ts)

        payload = json.dumps(row).encode()
        req = urllib.request.Request(url, data=payload, headers=headers, method="POST")
        try:
            code = _urlopen_with_retry(req).status
        except urllib.error.HTTPError as e:
            code = e.code

        dt_str = datetime.datetime.fromtimestamp(ts, tz=datetime.timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC")
        status = "✓" if code == 200 else "✗"
        print(f"  {status} Row {i+1}/{n_rows}: HTTP {code} — {dt_str}")
        if code == 200:
            ok += 1

    print()
    if ok == n_rows:
        print(f"✓ All {ok} rows ingested into {project_name}.{table_name}")
    else:
        print(f"✗ {ok}/{n_rows} rows succeeded — check errors above")
        sys.exit(1)


def _load_timestamp_field():
    """Read TIMESTAMP_FIELD from /tmp/hdx_ts_field_<PID> (written by find-sample-data)."""
    ts_path = f"/tmp/hdx_ts_field_{os.getpid()}"
    try:
        with open(ts_path) as f:
            return f.read().strip()
    except FileNotFoundError:
        print(f"ERROR: {ts_path} not found. Run find-sample-data first.", file=sys.stderr)
        sys.exit(1)


def cmd_auto_ingest(cluster_url, jwt, project_name, n_rows, start_date, end_date):
    """Discover all tables with sample_data in project and ingest into each."""
    start_ts = int(datetime.datetime.fromisoformat(start_date).replace(tzinfo=datetime.timezone.utc).timestamp())
    # Cap at start of today UTC (midnight) — ensures timestamps are always
    # "today or yesterday" regardless of the user's local timezone offset.
    _today_midnight_ts = int(datetime.datetime.combine(
        datetime.date.today(), datetime.time.min, tzinfo=datetime.timezone.utc
    ).timestamp())
    end_ts = min(
        int(datetime.datetime.fromisoformat(end_date).replace(
            hour=23, minute=59, second=59, tzinfo=datetime.timezone.utc).timestamp()),
        _today_midnight_ts,
    )

    org_uuid, proj = _resolve_project_and_org(cluster_url, jwt, project_name)
    tables = _api_get(
        f"{cluster_url}/config/v1/orgs/{org_uuid}/projects/{proj['uuid']}/tables/", jwt
    )["results"]

    # Build sample_map from all local bundle files
    sample_map = {}
    for tf_file in glob.glob("data/bundles/**/*.json", recursive=True):
        try:
            with open(tf_file) as f:
                t = json.load(f)
        except Exception:
            continue
        if not isinstance(t, dict):
            continue
        tf_name = tf_file.rsplit("/", 1)[-1].replace(".json", "")
        # Skip *_sample_data.json files — they are loaded as companions below
        if tf_name.endswith("_sample_data"):
            continue
        sd = t.get("settings", {}).get("sample_data")
        if not sd:
            # Look for a companion <name>_sample_data.json in the same directory
            sd_path = tf_file.replace(".json", "_sample_data.json")
            if os.path.exists(sd_path):
                try:
                    with open(sd_path) as f:
                        sd = json.load(f)
                except Exception:
                    pass
        if not sd:
            continue
        ts_field = _detect_timestamp_field(tf_file, sd)
        sample_map[tf_name] = {"sample": sd, "ts_field": ts_field}

    headers_base = {"Authorization": f"Bearer {jwt}", "Content-Type": "application/json"}
    step = (end_ts - start_ts) // max(n_rows - 1, 1)
    ingested_any = False
    any_failure = False

    for table in tables:
        tname = table["name"]
        transforms = _api_get(
            f"{cluster_url}/config/v1/orgs/{org_uuid}/projects/{proj['uuid']}/tables/{table['uuid']}/transforms/",
            jwt
        )["results"]

        for tr in transforms:
            tr_name = tr["name"]
            if tr_name not in sample_map:
                continue
            sd = sample_map[tr_name]["sample"]
            ts_field = sample_map[tr_name]["ts_field"]
            if not ts_field:
                print(f"  ⚠ Skipping {tname}/{tr_name}: no timestamp field")
                continue

            stream = table.get("settings", {}).get("stream", {})
            token_auth = bool(stream.get("token_auth_enabled"))
            ingest_token = (stream.get("token_list") or [""])[0]

            headers = dict(headers_base)
            if token_auth and ingest_token:
                headers["x-hdx-token"] = ingest_token

            url = f"{cluster_url}/ingest/event?table={project_name}.{tname}&transform={tr_name}"
            ok = 0
            for i in range(n_rows):
                row = json.loads(json.dumps(sd))
                ts = min(start_ts + i * step + random.randint(0, max(step - 1, 0)), end_ts)
                _set_nested(row, ts_field, ts)
                payload = json.dumps(row).encode()
                req = urllib.request.Request(url, data=payload, headers=headers, method="POST")
                try:
                    code = _urlopen_with_retry(req).status
                except urllib.error.HTTPError as e:
                    code = e.code
                if code == 200:
                    ok += 1

            status = "✓" if ok == n_rows else f"✗ ({ok}/{n_rows})"
            print(f"  {status} {tname}/{tr_name}: {ok}/{n_rows} rows")
            if ok < n_rows:
                any_failure = True
            ingested_any = True
            break  # one transform per table

    if not ingested_any:
        print("  ⚠ No transforms with sample_data found for this project")
    elif any_failure:
        print("✗ Some tables had failures")
        sys.exit(1)
    else:
        print("✓ All tables ingested")


def cmd_validate_bundle(bundle_name):
    """Print bundle path if it exists in latest_versions.yaml, else exit 1."""
    import yaml
    with open("data/bundles/latest_versions.yaml") as f:
        versions = yaml.safe_load(f)
    if bundle_name not in versions:
        available = ", ".join(versions.keys())
        print(f"ERROR: bundle '{bundle_name}' not found. Available: {available}", file=sys.stderr)
        sys.exit(1)
    bdl_path = os.path.join("data/bundles", versions[bundle_name])
    if not os.path.exists(bdl_path):
        print(f"ERROR: bundle file not found at {bdl_path}", file=sys.stderr)
        sys.exit(1)
    print(f"OK:{versions[bundle_name]}")


def cmd_clean_hdc_password(cluster_url):
    """Remove dummy_placeholder password from ingest user in .hdc.yaml."""
    cluster_host = urllib.parse.urlparse(cluster_url).netloc
    files = glob.glob(f"data/hydrolix/**/{cluster_host}/*.hdc.yaml", recursive=True)
    if not files:
        print(f"ERROR: could not find .hdc.yaml under {cluster_host}", file=sys.stderr)
        sys.exit(1)
    path = files[0]
    with open(path) as f:
        content = f.read()
    cleaned = re.sub(r'(\n    - [^\n]+_ingest\n)    password:[^\n]+', r'\1', content)
    with open(path, "w") as f:
        f.write(cleaned)
    print("✓ Ingest user password removed")


def cmd_clean_gfo_credentials(grafana_url, project_name):
    """Remove secureJsonData from datasources in .gfo.yaml."""
    import yaml
    grf_host = urllib.parse.urlparse(grafana_url).netloc
    pattern = f"data/grafana/{grf_host}/{project_name}/{project_name}.gfo.yaml"
    files = glob.glob(pattern)
    if not files:
        print(f"ERROR: could not find .gfo.yaml at {pattern}", file=sys.stderr)
        sys.exit(1)
    path = files[0]
    with open(path) as f:
        data = yaml.safe_load(f)
    for ds in data.get("datasources", {}).values():
        ds.pop("secureJsonData", None)
    with open(path, "w") as f:
        yaml.dump(data, f, default_flow_style=False, allow_unicode=True)
    print("✓ Grafana datasource credentials placeholder removed")


_EXTRA_TYPES = {"float32", "float64", "int16", "uint16"}

_FALLBACK_TYPES = {
    "string", "uint8", "uint16", "uint32", "uint64",
    "int8", "int16", "int32", "int64",
    "float32", "float64", "double",
    "boolean", "epoch", "datetime", "ip",
    "array", "map", "uuid",
}

_TYPES_SOURCE = "fallback"


def _load_valid_types():
    """Load valid Hydrolix types from docs/transform_validations.json (source of truth)."""
    global _TYPES_SOURCE
    try:
        vpath = os.path.join(os.path.dirname(__file__), "..", "..", "docs", "transform_validations.json")
        with open(vpath) as f:
            t = json.load(f)
        types = set()
        for col in t.get("settings", {}).get("output_columns", []):
            dt = col.get("datatype", {}).get("type")
            if dt:
                types.add(dt)
        types |= _EXTRA_TYPES
        _TYPES_SOURCE = "file"
        return types
    except Exception:
        _TYPES_SOURCE = "fallback"
        return set(_FALLBACK_TYPES)


VALID_HYDROLIX_TYPES = None  # lazy-loaded by _get_valid_types()


def _get_valid_types():
    """Return valid types set, loading on first call."""
    global VALID_HYDROLIX_TYPES
    if VALID_HYDROLIX_TYPES is None:
        VALID_HYDROLIX_TYPES = _load_valid_types()
    return VALID_HYDROLIX_TYPES


def _resolve_hdp_extends(hdp_path, visited=None):
    """
    Recursively resolve __extend__ chains in an hdp.yaml file using regex
    (no yaml dependency required).

    Returns a list of (table_name, transform_name, json_file_path) tuples
    for every transform JSON file that is referenced.
    """
    if visited is None:
        visited = set()
    hdp_path = os.path.abspath(hdp_path)
    if hdp_path in visited:
        return []
    visited.add(hdp_path)

    if not os.path.exists(hdp_path):
        return []

    with open(hdp_path) as f:
        content = f.read()

    base_dir = os.path.dirname(hdp_path)
    results = []

    # Follow top-level __extend__ entries (list items or scalar)
    # Matches:  "- path/to/file.yaml"  or  "__extend__: path/to/file.yaml"
    for m in re.finditer(r'(?:^-\s+|__extend__:\s*)(\S+\.yaml)\s*$', content, re.MULTILINE):
        child_path = os.path.normpath(os.path.join(base_dir, m.group(1)))
        results.extend(_resolve_hdp_extends(child_path, visited))

    # Find transform JSON references using a two-pass approach:
    # 1. Find table+transform context lines, then __extend__ .json on next non-empty lines
    # Pattern: under "transforms:", find blocks like:
    #   <table_name>:
    #     <transform_name>:
    #       __extend__: path/to/file.json
    for m in re.finditer(
        r'^(\s{2})(\w+):\s*\n(?:.*\n)*?(?=\s{4}\w)',  # table block start
        content, re.MULTILINE
    ):
        pass  # fallback: scan all __extend__ .json lines with context

    # Simple approach: scan all __extend__ → .json lines, pair with nearest parent keys
    lines = content.splitlines()
    table_name = None
    transform_name = None
    indent_stack = []  # (indent_level, key)

    for line in lines:
        stripped = line.rstrip()
        if not stripped or stripped.lstrip().startswith('#'):
            continue
        indent = len(line) - len(line.lstrip())
        key_match = re.match(r'^(\s*)(\w+):\s*$', stripped)
        extend_match = re.match(r'^(\s*)__extend__:\s*(\S+\.json)\s*$', stripped)

        if key_match:
            key = key_match.group(2)
            # Pop stack to current indent level
            indent_stack = [(i, k) for i, k in indent_stack if i < indent]
            indent_stack.append((indent, key))
            # Update table/transform context based on depth
            if indent == 2:
                table_name = key
                transform_name = None
            elif indent == 4 and table_name:
                transform_name = key

        elif extend_match and table_name and transform_name:
            json_rel = extend_match.group(2)
            json_path = os.path.normpath(os.path.join(base_dir, json_rel))
            results.append((table_name, transform_name, json_path))

    return results


def cmd_validate_transforms_local(cluster_host, project_name):
    """
    Validate output_columns[].datatype.type in all transform JSON files
    referenced by the local project hdp.yaml.

    Exit 1 if any invalid type is found, 0 otherwise.
    """
    # Find the project hdp.yaml
    pattern = f"data/hydrolix/**/{cluster_host}/{project_name}/{project_name}.hdp.yaml"
    files = glob.glob(pattern, recursive=True)
    if not files:
        print(f"ERROR: could not find hdp.yaml for project '{project_name}' on '{cluster_host}'", file=sys.stderr)
        sys.exit(1)

    hdp_path = files[0]
    transform_refs = _resolve_hdp_extends(hdp_path)

    if not transform_refs:
        print(f"OK: no transform JSON files found for project '{project_name}'")
        return

    # Per-transform tracking: {transform_name: {"columns_checked": N, "failures": [...]}}
    transform_results = {}

    for table_name, transform_name, json_path in transform_refs:
        if not os.path.exists(json_path):
            continue
        try:
            with open(json_path) as f:
                t = json.load(f)
        except Exception as e:
            print(f"WARNING: could not parse {json_path}: {e}", file=sys.stderr)
            continue

        if isinstance(t, list):
            continue  # skip sample_data files (arrays)

        output_columns = t.get("settings", {}).get("output_columns", t.get("output_columns", []))
        if not output_columns:
            continue

        if transform_name not in transform_results:
            transform_results[transform_name] = {"columns_checked": 0, "failures": []}

        transform_results[transform_name]["columns_checked"] += len(output_columns)

        valid_types = _get_valid_types()
        for col in output_columns:
            dtype = col.get("datatype", {})
            type_val = dtype.get("type")
            if type_val and type_val not in valid_types:
                col_name = col.get("name", "<unknown>")
                transform_results[transform_name]["failures"].append(
                    f'column:{col_name} type:"{type_val}" is not a valid choice'
                )

    # Build summary for output and report
    any_failure = False
    total_fields = 0
    transforms_detail = {}

    for tr_name, info in transform_results.items():
        total_fields += info["columns_checked"]
        if info["failures"]:
            any_failure = True
            transforms_detail[tr_name] = {
                "status": "FAIL",
                "columns_checked": info["columns_checked"],
                "errors": info["failures"],
            }
        else:
            transforms_detail[tr_name] = {
                "status": "PASS",
                "columns_checked": info["columns_checked"],
            }

    # Output
    print(f"TYPES_SOURCE:{_TYPES_SOURCE}")
    if any_failure:
        for tr_name, info in transform_results.items():
            for fail in info["failures"]:
                print(f"FAILED: transform:{tr_name} {fail}")
    else:
        print("OK: all transforms valid")
    print(f"FIELDS_CHECKED:{total_fields}")
    print(f"TRANSFORMS_DETAIL:{json.dumps(transforms_detail)}")
    sys.exit(1 if any_failure else 0)



def cmd_patch_grafana_ds(grafana_url, grafana_user, grafana_pwd,
                         project_name, cluster_url, hdx_user, hdx_pwd):
    """Get Grafana org ID and patch the Hydrolix datasource credentials."""
    import base64
    cluster_host = urllib.parse.urlparse(cluster_url).netloc
    token = base64.b64encode(f"{grafana_user}:{grafana_pwd}".encode()).decode()
    headers = {"Authorization": f"Basic {token}", "Content-Type": "application/json"}

    req = urllib.request.Request(f"{grafana_url}/api/orgs", headers=headers)
    orgs = json.loads(_urlopen(req).read())
    org = next((o for o in orgs if o["name"] == project_name), None)
    if not org:
        print(f"ERROR: org '{project_name}' not found in Grafana", file=sys.stderr)
        sys.exit(1)

    headers["X-Grafana-Org-Id"] = str(org["id"])
    payload = json.dumps({
        "name": "Hydrolix",
        "type": "hydrolix-hydrolix-datasource",
        "access": "proxy",
        "isDefault": True,
        "jsonData": {
            "host": cluster_host,
            "defaultDatabase": project_name,
            "username": hdx_user,
            "adHocTableVariable": "table",
            "adHocTimeColumnVariable": "timestamp",
            "defaultRound": "1m",
            "path": "/query",
            "port": 9444,
            "protocol": "http",
            "secure": True,
            "useDefaultPort": False,
        },
        "secureJsonData": {"password": hdx_pwd},
    }).encode()

    req = urllib.request.Request(
        f"{grafana_url}/api/datasources/uid/hdx-hydrolix-datasource",
        data=payload, headers=headers, method="PUT"
    )
    res = json.loads(_urlopen(req).read())
    print(f"✓ Datasource patched: {res.get('message', res)}")


# ---------------------------------------------------------------------------
# Deploy report
# ---------------------------------------------------------------------------

REPORT_DIR = "data/reports"

_PG_CREDS_PATH = os.path.join(os.path.dirname(__file__), "..", "..", "docs", "postgres_credentials.toml")


def _load_pg_config():
    """Load Postgres connection config from env var or TOML file."""
    db_url = os.environ.get("DATABASE_URL")
    if db_url:
        parsed = urllib.parse.urlparse(db_url)
        return {
            "host": parsed.hostname, "port": str(parsed.port or 5432),
            "user": parsed.username, "password": parsed.password,
            "dbname": parsed.path.lstrip("/"),
        }
    try:
        config = {}
        with open(_PG_CREDS_PATH) as f:
            for line in f:
                line = line.strip()
                if "=" in line and not line.startswith("["):
                    k, v = line.split("=", 1)
                    config[k.strip()] = v.strip().strip('"')
        return {
            "host": config["host"], "port": config.get("port", "5432"),
            "user": config["username"], "password": config["password"],
            "dbname": config["db_name"],
        }
    except Exception:
        return None


def _pg_sync(report, project_name):
    """Sync a report dict to Postgres. Best-effort — failures don't block the deploy."""
    import subprocess
    pg = _load_pg_config()
    if not pg:
        return

    stages_json = json.dumps(report["stages"])
    overall = report.get("overall", "PENDING")
    bundle_name = report.get("bundle_name", "")
    bundle_id = report.get("bundle_id", "")
    user_bundle = report.get("user_bundle", "")

    def _pg_escape(s):
        """Escape a string for use in a PL/pgSQL literal (double single quotes)."""
        return str(s).replace("'", "''")

    pn = _pg_escape(project_name)
    bn = _pg_escape(bundle_name)
    bi = _pg_escape(bundle_id)
    ub = _pg_escape(user_bundle)
    ov = _pg_escape(overall)
    sj = _pg_escape(stages_json)

    sql = f"""
    DO $$
    DECLARE
      rid UUID;
    BEGIN
      SELECT id INTO rid FROM bundle_deploy_reports
        WHERE project_name = '{pn}' AND bundle_name = '{bn}'
        ORDER BY created_at DESC LIMIT 1;
      IF rid IS NOT NULL AND (
        SELECT overall FROM bundle_deploy_reports WHERE id = rid
      ) NOT IN ('PASS', 'FAIL') THEN
        UPDATE bundle_deploy_reports
          SET stages = '{sj}'::jsonb, overall = '{ov}', updated_at = now()
          WHERE id = rid;
      ELSE
        INSERT INTO bundle_deploy_reports (project_name, bundle_name, bundle_id, user_bundle, overall, stages)
        VALUES ('{pn}', '{bn}', '{bi}', '{ub}', '{ov}', '{sj}'::jsonb);
      END IF;
    END $$;
    """

    env = {**os.environ, "PGPASSWORD": pg["password"]}
    try:
        subprocess.run(
            ["psql", "-h", pg["host"], "-p", pg["port"], "-U", pg["user"],
             "-d", pg["dbname"], "-c", sql],
            env=env, capture_output=True, timeout=10,
        )
    except Exception:
        pass  # best-effort


def _report_path(project_name):
    return os.path.join(REPORT_DIR, f"{project_name}_deploy_report.json")


def cmd_deploy_report(action, project_name, *args):
    """
    Manage a deploy report JSON file.

    Sub-actions:
      init   <project_name> <bundle_name> <bundle_id> <user_bundle>
        Create a fresh report with all stages set to PENDING.

      update <project_name> <stage> <status> [key=value ...]
        Update a stage's status and optional extra fields.
        e.g.: update myproj transform_validate PASS fields_checked=14

      finalize <project_name>
        Compute overall status (PASS if all stages PASS, else FAIL) and print the report.
    """
    os.makedirs(REPORT_DIR, exist_ok=True)
    path = _report_path(project_name)

    if action == "init":
        bundle_name = args[0]
        bundle_id = args[1]
        user_bundle = args[2]
        report = {
            "skill": "bundle-deploy",
            "user_bundle": user_bundle,
            "bundle_name": bundle_name,
            "bundle_id": bundle_id,
            "timestamp": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "stages": {
                "deploy": {"status": "PENDING"},
                "shared_config": {"status": "PENDING"},
                "transform_validate": {"status": "PENDING"},
                "transform_apply": {"status": "PENDING"},
                "roundtrip": {"status": "PENDING"},
            },
            "overall": "PENDING",
        }
        with open(path, "w") as f:
            json.dump(report, f, indent=2)
        _pg_sync(report, project_name)
        print(f"✓ Report initialized: {path}")

    elif action == "update":
        stage = args[0]
        status = args[1]
        extras = {}
        for kv in args[2:]:
            k, v = kv.split("=", 1)
            # Try to parse as JSON (objects/arrays), then numeric, then string
            try:
                v = json.loads(v)
            except (json.JSONDecodeError, ValueError):
                try:
                    v = int(v)
                except ValueError:
                    try:
                        v = float(v)
                    except ValueError:
                        pass
            extras[k] = v

        # Stages that depend on earlier stages passing
        _SKIP_DOWNSTREAM = {
            "transform_validate": ["transform_apply", "roundtrip"],
            "deploy": ["roundtrip"],
            "transform_apply": ["roundtrip"],
        }

        with open(path) as f:
            report = json.load(f)
        report["stages"][stage] = {"status": status, **extras}

        # If a stage FAILs, mark its downstream dependents as SKIPPED
        if status == "FAIL":
            for downstream in _SKIP_DOWNSTREAM.get(stage, []):
                if report["stages"].get(downstream, {}).get("status") == "PENDING":
                    report["stages"][downstream] = {"status": "SKIPPED", "reason": f"{stage} failed"}

        with open(path, "w") as f:
            json.dump(report, f, indent=2)
        _pg_sync(report, project_name)
        print(f"✓ Stage '{stage}' → {status}")

    elif action == "finalize":
        with open(path) as f:
            report = json.load(f)
        statuses = [s["status"] for s in report["stages"].values()]
        if any(s == "FAIL" for s in statuses):
            report["overall"] = "FAIL"
        elif all(s in ("PASS", "SKIPPED") for s in statuses):
            # SKIPPED stages are acceptable (downstream of a FAIL)
            report["overall"] = "FAIL" if any(s == "SKIPPED" for s in statuses) else "PASS"
        else:
            report["overall"] = "INCOMPLETE"
        with open(path, "w") as f:
            json.dump(report, f, indent=2)
        _pg_sync(report, project_name)
        print(json.dumps(report, indent=2))

    else:
        print(f"ERROR: unknown report action '{action}'", file=sys.stderr)
        sys.exit(1)


# ---------------------------------------------------------------------------
# CLI dispatcher
# ---------------------------------------------------------------------------

COMMANDS = {
    "get-jwt":                    (cmd_get_jwt,                    3),
    "extract-credentials":        (cmd_extract_credentials,        1),  # +1 optional
    "discover-table-auth":        (cmd_discover_table_auth,        4),
    "find-sample-data":           (cmd_find_sample_data,           1),
    "ingest":                     (cmd_ingest,                     8),  # +1 optional
    "auto-ingest":                (cmd_auto_ingest,                6),
    "validate-bundle":            (cmd_validate_bundle,            1),
    "clean-hdc-password":         (cmd_clean_hdc_password,         1),
    "clean-gfo-credentials":      (cmd_clean_gfo_credentials,      2),
    "patch-grafana-ds":           (cmd_patch_grafana_ds,           7),
    "validate-transforms-local":  (cmd_validate_transforms_local,  2),
    "deploy-report":              (cmd_deploy_report,              2),  # action + project_name + varargs
}

if __name__ == "__main__":
    if len(sys.argv) < 2 or sys.argv[1] not in COMMANDS:
        print(__doc__)
        sys.exit(1)

    cmd_name = sys.argv[1]
    func, min_args = COMMANDS[cmd_name]
    args = sys.argv[2:]

    if len(args) < min_args:
        print(f"ERROR: '{cmd_name}' requires at least {min_args} argument(s), got {len(args)}", file=sys.stderr)
        sys.exit(1)

    # Special handling for commands with typed args
    if cmd_name == "ingest":
        func(args[0], args[1], args[2], args[3], args[4],
             int(args[5]), args[6], args[7],
             args[8] if len(args) > 8 else "")
    elif cmd_name == "auto-ingest":
        func(args[0], args[1], args[2], int(args[3]), args[4], args[5])
    elif cmd_name == "extract-credentials":
        func(args[0], args[1] if len(args) > 1 else None)
    elif cmd_name == "deploy-report":
        # deploy-report <action> <project_name> [extra args...]
        func(args[0], args[1], *args[2:])
    else:
        func(*args)