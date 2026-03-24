#!/usr/bin/env python3
"""Generate a structured runbook from bundle.json metadata.

Reads a bundle.json file and produces a Confluence Storage Format (HTML) page
suitable for publishing to the Technical Enablement workspace.

Usage:
    python scripts/generate_runbook.py --bundle-dir aws/cloudfront-to-kinesis
    python scripts/generate_runbook.py --bundle-dir aws/cloudfront-to-kinesis --output runbook.json
"""

import argparse
import html
import json
import os
import sys
from datetime import date


SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(SCRIPTS_DIR)


def main():
    args = parse_args()

    bundle_dir = args.bundle_dir
    if not os.path.isabs(bundle_dir):
        bundle_dir = os.path.join(REPO_ROOT, bundle_dir)

    bundle_json_path = os.path.join(bundle_dir, "bundle.json")
    if not os.path.isfile(bundle_json_path):
        print(f"Error: bundle.json not found at {bundle_json_path}", file=sys.stderr)
        sys.exit(1)

    with open(bundle_json_path, "r", encoding="utf-8") as f:
        bundle = json.load(f)

    result = generate_runbook(bundle, bundle_dir)

    if args.output:
        with open(args.output, "w", encoding="utf-8") as f:
            json.dump(result, f, indent=2)
        print(f"Runbook written to {args.output}", file=sys.stderr)
    else:
        print(json.dumps(result, indent=2))


def parse_args():
    parser = argparse.ArgumentParser(
        description="Generate a Confluence runbook from bundle.json metadata.",
    )
    parser.add_argument(
        "--bundle-dir", required=True,
        help="Path to bundle directory (relative to repo root or absolute)",
    )
    parser.add_argument(
        "--output", default="",
        help="Write JSON output to this file instead of stdout",
    )
    return parser.parse_args()


def generate_runbook(bundle: dict, bundle_dir: str) -> dict:
    """Generate a runbook dict from bundle metadata.

    Returns:
        {
            "title": str,
            "content": str,   # Confluence Storage Format HTML
            "bundle_name": str,
            "version": str,
            "beta": bool,
            "data_category": str,
        }
    """
    name = bundle.get("name", os.path.basename(bundle_dir))
    metadata = bundle.get("metadata", {})
    version = metadata.get("version", "1.0.0")
    beta = bundle.get("beta", True)
    ui = bundle.get("ui", {})
    data_category = ui.get("data_category", "")
    source_title = ui.get("source", {}).get("full_title", bundle.get("source", name))
    method_title = ui.get("method", {}).get("full_title", bundle.get("method", ""))
    description = metadata.get("description", "")
    maintainer = metadata.get("maintainer", "")
    channel_type = metadata.get("channel_type", "")
    base_url = bundle.get("base_url", "")
    primary_url = ui.get("primary_url", "")
    beta_label = "Beta" if beta else "GA"

    title = f"{source_title} Bundle Runbook"

    content = _build_content(
        bundle=bundle,
        name=name,
        version=version,
        beta=beta,
        beta_label=beta_label,
        description=description,
        maintainer=maintainer,
        channel_type=channel_type,
        source_title=source_title,
        method_title=method_title,
        data_category=data_category,
        base_url=base_url,
        primary_url=primary_url,
        bundle_dir=bundle_dir,
    )

    return {
        "title": title,
        "content": content,
        "bundle_name": name,
        "version": version,
        "beta": beta,
        "data_category": data_category,
    }


def _build_content(*, bundle, name, version, beta, beta_label, description, maintainer,
                   channel_type, source_title, method_title, data_category, base_url,
                   primary_url, bundle_dir):
    """Build Confluence Storage Format HTML for the runbook."""
    today = date.today().isoformat()
    sections = []

    # ── Header info panel ────────────────────────────────────────────────────
    panel_type = "warning" if beta else "info"
    status_icon = "⚠️ Beta" if beta else "✅ GA"
    sections.append(f"""
<ac:structured-macro ac:name="{panel_type}">
  <ac:rich-text-body>
    <p><strong>Status:</strong> {status_icon} &nbsp;|&nbsp;
       <strong>Version:</strong> {h(version)} &nbsp;|&nbsp;
       <strong>Last Updated:</strong> {today} &nbsp;|&nbsp;
       <strong>Maintainer:</strong> {h(maintainer)}
    </p>
  </ac:rich-text-body>
</ac:structured-macro>
""".strip())

    # ── Overview ─────────────────────────────────────────────────────────────
    overview_rows = [
        ("Bundle Name", h(name)),
        ("Source", h(source_title)),
        ("Ingest Method", h(method_title)),
        ("Channel Type", h(channel_type)),
        ("Data Category", h(data_category)),
        ("Release Status", beta_label),
        ("Version", h(version)),
    ]
    if primary_url:
        overview_rows.append(("Documentation", f'<a href="{h(primary_url)}">{h(primary_url)}</a>'))
    if base_url:
        overview_rows.append(("Source Repository", f'<a href="{h(base_url)}">{h(base_url)}</a>'))

    sections.append(_section("Overview", _table(overview_rows) + (
        f"\n<p>{h(description)}</p>" if description else ""
    )))

    # ── What This Bundle Shows ───────────────────────────────────────────────
    dashboard = bundle.get("dashboard", {})
    other_dashboards = bundle.get("other_dashboards", [])
    all_dashboards = []
    if dashboard:
        all_dashboards.append(dashboard.get("path", ""))
    all_dashboards += [d.get("path", "") for d in other_dashboards if d.get("path")]
    all_dashboards = [d for d in all_dashboards if d]

    if all_dashboards:
        dash_items = "".join(f"<li>{h(d)}</li>" for d in all_dashboards)
        sections.append(_section(
            "What This Bundle Shows",
            f"<p>This bundle provides the following Grafana dashboards:</p>"
            f"<ul>{dash_items}</ul>",
        ))
    else:
        sections.append(_section(
            "What This Bundle Shows",
            "<p>No dashboards are defined for this bundle.</p>",
        ))

    # ── What This Bundle Does ────────────────────────────────────────────────
    tables = bundle.get("tables", [])
    summary_tables = bundle.get("summary_tables", [])

    table_rows = []
    for t in tables:
        tname = t.get("name", "")
        transforms = t.get("transforms", [])
        methods = ", ".join(
            tr.get("method", bundle.get("method", "")) for tr in transforms
        ) or bundle.get("method", "")
        table_rows.append((h(tname), h(methods), "Primary"))

    for t in summary_tables:
        tname = t.get("name", "")
        parent = t.get("parent_table_name", "")
        table_rows.append((h(tname), f"Summary of {h(parent)}", "Summary"))

    if table_rows:
        table_html = (
            "<table><tbody>"
            "<tr><th>Table Name</th><th>Ingest Method</th><th>Type</th></tr>"
            + "".join(f"<tr><td>{r[0]}</td><td>{r[1]}</td><td>{r[2]}</td></tr>" for r in table_rows)
            + "</tbody></table>"
        )
        sections.append(_section("What This Bundle Does", table_html))
    else:
        sections.append(_section(
            "What This Bundle Does",
            "<p>No tables are defined for this bundle.</p>",
        ))

    # ── Dependencies ─────────────────────────────────────────────────────────
    deps = bundle.get("dependencies", {})
    dep_content_parts = []

    grafana_deps = deps.get("grafana", {})
    grafana_version = grafana_deps.get("version", "")
    grafana_plugins = grafana_deps.get("plugins", [])
    hydrolix_deps = deps.get("hydrolix", {})
    cluster_version = hydrolix_deps.get("cluster_version", "")
    req_functions = hydrolix_deps.get("required_functions", [])
    shared_functions = hydrolix_deps.get("shared_functions", [])
    req_dicts = hydrolix_deps.get("required_dictionaries", [])
    shared_dicts = hydrolix_deps.get("shared_dictionaries", [])
    all_functions = list(req_functions) + list(shared_functions)
    all_dicts = list(req_dicts) + list(shared_dicts)

    dep_rows = []
    if grafana_version:
        dep_rows.append(("Grafana", f"version {h(grafana_version)}"))
    if cluster_version:
        dep_rows.append(("Hydrolix Cluster", f"version {h(cluster_version)}"))
    if grafana_plugins:
        plugin_list = ", ".join(
            f"{p.get('name', '')} {p.get('version', '')}".strip() for p in grafana_plugins
        )
        dep_rows.append(("Grafana Plugins", h(plugin_list)))
    if all_functions:
        dep_rows.append(("Required Functions", h(", ".join(all_functions))))
    if all_dicts:
        dep_rows.append(("Required Dictionaries", h(", ".join(all_dicts))))

    if dep_rows:
        dep_content_parts.append(_table(dep_rows))
    else:
        dep_content_parts.append("<p>No external dependencies declared.</p>")

    sections.append(_section("Dependencies", "\n".join(dep_content_parts)))

    # ── Non-Default Configuration Notes ──────────────────────────────────────
    method_overrides = bundle.get("method_overrides", {})
    config_notes_parts = []

    if method_overrides:
        override_rows = [(h(k), h(str(v))) for k, v in method_overrides.items()]
        config_notes_parts.append(
            "<p>This bundle requires non-default method overrides:</p>"
            + _table(override_rows)
        )

    # Highlight any non-standard methods
    method = bundle.get("method", "")
    standard_methods = {"firehose", "s3", "kinesis", "lambda", "http_streaming", "http"}
    if method and method not in standard_methods:
        config_notes_parts.append(
            f"<p><strong>Non-standard ingest method:</strong> {h(method)} — "
            f"verify cluster support before deployment.</p>"
        )

    if not config_notes_parts:
        config_notes_parts.append(
            "<p>No non-default configuration is required. "
            "Default settings are sufficient for deployment.</p>"
        )

    sections.append(_section("Non-Default Configuration Notes", "\n".join(config_notes_parts)))

    # ── Release Notes ─────────────────────────────────────────────────────────
    sections.append(_section(
        "Release Notes",
        f"<h3>v{h(version)}</h3>"
        f"<p><em>Published: {today}</em></p>"
        f"<ul><li>Initial release of the {h(source_title)} bundle.</li></ul>"
        f"<ac:structured-macro ac:name='info'>"
        f"<ac:rich-text-body><p>Update this section with specific changes for each version.</p>"
        f"</ac:rich-text-body></ac:structured-macro>",
    ))

    # ── Quick Start ───────────────────────────────────────────────────────────
    quick_start_steps = [
        f"Ensure your Hydrolix cluster meets the minimum version requirement"
        + (f" ({h(cluster_version)})" if cluster_version else "") + ".",
        "Deploy the bundle via the Hydrolix Marketplace or CAC tools.",
        "Configure the data source in Grafana using the "
        + (f"{h(method_title)} integration" if method_title else "configured ingest method") + ".",
        "Import the provided dashboard(s) into your Grafana instance.",
        "Verify data is flowing by checking the primary dashboard.",
    ]
    steps_html = "".join(f"<li>{s}</li>" for s in quick_start_steps)
    docs_link = (
        f'<p>Full documentation: <a href="{h(primary_url)}">{h(primary_url)}</a></p>'
        if primary_url else ""
    )
    sections.append(_section(
        "Quick Start",
        f"<ol>{steps_html}</ol>{docs_link}",
    ))

    return "\n\n".join(sections)


# ── Helpers ───────────────────────────────────────────────────────────────────

def h(text: str) -> str:
    """HTML-escape a string."""
    return html.escape(str(text))


def _section(title: str, body: str) -> str:
    return f"<h2>{title}</h2>\n{body}"


def _table(rows: list) -> str:
    """Build a two-column key/value HTML table."""
    rows_html = "".join(
        f"<tr><th>{r[0]}</th><td>{r[1]}</td></tr>" for r in rows
    )
    return f"<table><tbody>{rows_html}</tbody></table>"


if __name__ == "__main__":
    main()
