#!/usr/bin/env python3
"""Publish (upsert) a runbook page to Confluence.

Reads a runbook JSON file (produced by generate_runbook.py) and creates or
updates a page in the configured Confluence space.

Required environment variables:
    CONFLUENCE_BASE_URL        e.g. https://yourorg.atlassian.net/wiki
    CONFLUENCE_USER            e.g. bot@yourorg.io
    CONFLUENCE_API_TOKEN       Atlassian API token
    CONFLUENCE_SPACE_KEY       e.g. TE  (Technical Enablement space)
    CONFLUENCE_PARENT_PAGE_ID  Numeric ID of the parent page for new runbooks

Usage:
    python scripts/publish_to_confluence.py --runbook runbook.json
    python scripts/publish_to_confluence.py --runbook runbook.json --dry-run
"""

import argparse
import json
import os
import sys
from urllib import request, parse, error as urllib_error
from base64 import b64encode


def main():
    args = parse_args()

    with open(args.runbook, "r", encoding="utf-8") as f:
        runbook = json.load(f)

    config = load_config()

    if args.dry_run:
        print(f"[dry-run] Would publish: {runbook['title']!r} to space {config['space_key']!r}",
              file=sys.stderr)
        print(json.dumps({"url": "https://example.atlassian.net/wiki/dry-run", "action": "dry-run"}))
        return

    url = publish(runbook, config)
    print(json.dumps({"url": url, "title": runbook["title"]}))


def parse_args():
    parser = argparse.ArgumentParser(
        description="Publish a runbook JSON to Confluence (upsert).",
    )
    parser.add_argument("--runbook", required=True, help="Path to runbook.json")
    parser.add_argument("--dry-run", action="store_true",
                        help="Print what would be published without making API calls")
    return parser.parse_args()


def load_config() -> dict:
    """Load and validate required environment variables."""
    required = {
        "base_url": "CONFLUENCE_BASE_URL",
        "user": "CONFLUENCE_USER",
        "api_token": "CONFLUENCE_API_TOKEN",
        "space_key": "CONFLUENCE_SPACE_KEY",
        "parent_page_id": "CONFLUENCE_PARENT_PAGE_ID",
    }
    config = {}
    missing = []
    for key, env_var in required.items():
        val = os.environ.get(env_var, "")
        if not val:
            missing.append(env_var)
        config[key] = val

    if missing:
        print(f"Error: missing required environment variables: {', '.join(missing)}", file=sys.stderr)
        sys.exit(1)

    config["base_url"] = config["base_url"].rstrip("/")
    return config


def publish(runbook: dict, config: dict) -> str:
    """Create or update the Confluence page. Returns the page URL."""
    existing = find_page(runbook["title"], config)

    if existing:
        page_id = existing["id"]
        current_version = existing["version"]["number"]
        _update_page(page_id, current_version + 1, runbook, config)
        print(f"Updated existing page (id={page_id}): {runbook['title']!r}", file=sys.stderr)
    else:
        page_id = _create_page(runbook, config)
        print(f"Created new page (id={page_id}): {runbook['title']!r}", file=sys.stderr)

    return f"{config['base_url']}/spaces/{config['space_key']}/pages/{page_id}"


def find_page(title: str, config: dict) -> dict | None:
    """Search for an existing page by title in the space. Returns the page dict or None."""
    encoded_title = parse.quote(title)
    url = (
        f"{config['base_url']}/rest/api/content"
        f"?title={encoded_title}&spaceKey={config['space_key']}&expand=version"
    )
    resp = _api_get(url, config)
    results = resp.get("results", [])
    return results[0] if results else None


def _create_page(runbook: dict, config: dict) -> str:
    """Create a new Confluence page. Returns the new page ID."""
    payload = {
        "type": "page",
        "title": runbook["title"],
        "space": {"key": config["space_key"]},
        "ancestors": [{"id": config["parent_page_id"]}],
        "body": {
            "storage": {
                "value": runbook["content"],
                "representation": "storage",
            }
        },
    }
    url = f"{config['base_url']}/rest/api/content"
    resp = _api_post(url, payload, config)
    return resp["id"]


def _update_page(page_id: str, new_version: int, runbook: dict, config: dict):
    """Update an existing Confluence page."""
    payload = {
        "type": "page",
        "title": runbook["title"],
        "version": {"number": new_version},
        "body": {
            "storage": {
                "value": runbook["content"],
                "representation": "storage",
            }
        },
    }
    url = f"{config['base_url']}/rest/api/content/{page_id}"
    _api_put(url, payload, config)


def _auth_header(config: dict) -> str:
    credentials = f"{config['user']}:{config['api_token']}"
    encoded = b64encode(credentials.encode("utf-8")).decode("utf-8")
    return f"Basic {encoded}"


def _api_get(url: str, config: dict) -> dict:
    req = request.Request(url, headers={
        "Authorization": _auth_header(config),
        "Accept": "application/json",
    })
    return _do_request(req)


def _api_post(url: str, payload: dict, config: dict) -> dict:
    data = json.dumps(payload).encode("utf-8")
    req = request.Request(url, data=data, method="POST", headers={
        "Authorization": _auth_header(config),
        "Content-Type": "application/json",
        "Accept": "application/json",
    })
    return _do_request(req)


def _api_put(url: str, payload: dict, config: dict) -> dict:
    data = json.dumps(payload).encode("utf-8")
    req = request.Request(url, data=data, method="PUT", headers={
        "Authorization": _auth_header(config),
        "Content-Type": "application/json",
        "Accept": "application/json",
    })
    return _do_request(req)


def _do_request(req: request.Request) -> dict:
    try:
        with request.urlopen(req) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except urllib_error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace")
        print(f"Confluence API error {e.code}: {body}", file=sys.stderr)
        sys.exit(1)
    except urllib_error.URLError as e:
        print(f"Network error calling Confluence: {e.reason}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
