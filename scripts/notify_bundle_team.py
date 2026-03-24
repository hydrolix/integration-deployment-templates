#!/usr/bin/env python3
"""Notify the bundle deployment team via Slack after a runbook is published.

Sends a structured Slack message with bundle details and a Confluence link.
When `beta` is false (GA bundle), appends an enablement session prompt.

Required environment variables:
    SLACK_WEBHOOK_URL   Incoming Webhook URL for the bundle deployment channel

Usage:
    python scripts/notify_bundle_team.py \
        --runbook runbook.json \
        --confluence-url https://yourorg.atlassian.net/wiki/spaces/TE/pages/12345
"""

import argparse
import json
import os
import sys
from urllib import request, error as urllib_error


def main():
    args = parse_args()

    webhook_url = os.environ.get("SLACK_WEBHOOK_URL", "")
    if not webhook_url:
        print("Error: SLACK_WEBHOOK_URL environment variable is not set", file=sys.stderr)
        sys.exit(1)

    with open(args.runbook, "r", encoding="utf-8") as f:
        runbook = json.load(f)

    if args.dry_run:
        payload = build_payload(runbook, args.confluence_url)
        print("[dry-run] Slack payload:", file=sys.stderr)
        print(json.dumps(payload, indent=2), file=sys.stderr)
        return

    send_notification(runbook, args.confluence_url, webhook_url)


def parse_args():
    parser = argparse.ArgumentParser(
        description="Send a Slack notification after a bundle runbook is published.",
    )
    parser.add_argument("--runbook", required=True, help="Path to runbook.json")
    parser.add_argument("--confluence-url", required=True,
                        help="URL of the published Confluence page")
    parser.add_argument("--dry-run", action="store_true",
                        help="Print the Slack payload without sending")
    return parser.parse_args()


def send_notification(runbook: dict, confluence_url: str, webhook_url: str):
    payload = build_payload(runbook, confluence_url)
    data = json.dumps(payload).encode("utf-8")
    req = request.Request(webhook_url, data=data, method="POST", headers={
        "Content-Type": "application/json",
    })
    try:
        with request.urlopen(req) as resp:
            status = resp.read().decode("utf-8")
            if status != "ok":
                print(f"Warning: Slack returned unexpected response: {status!r}", file=sys.stderr)
            else:
                print("Slack notification sent successfully", file=sys.stderr)
    except urllib_error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace")
        print(f"Slack API error {e.code}: {body}", file=sys.stderr)
        sys.exit(1)
    except urllib_error.URLError as e:
        print(f"Network error sending Slack notification: {e.reason}", file=sys.stderr)
        sys.exit(1)


def build_payload(runbook: dict, confluence_url: str) -> dict:
    bundle_name = runbook.get("bundle_name", "unknown")
    version = runbook.get("version", "1.0.0")
    beta = runbook.get("beta", True)
    data_category = runbook.get("data_category", "")
    title = runbook.get("title", bundle_name)

    status_emoji = ":warning: *Beta*" if beta else ":white_check_mark: *GA*"
    category_text = f" | Category: `{data_category}`" if data_category else ""

    blocks = [
        {
            "type": "header",
            "text": {
                "type": "plain_text",
                "text": ":page_facing_up: Bundle Runbook Published",
                "emoji": True,
            },
        },
        {
            "type": "section",
            "fields": [
                {"type": "mrkdwn", "text": f"*Bundle:*\n`{bundle_name}`"},
                {"type": "mrkdwn", "text": f"*Version:*\n`{version}`"},
                {"type": "mrkdwn", "text": f"*Status:*\n{status_emoji}"},
                {"type": "mrkdwn", "text": f"*Data Category:*\n`{data_category or 'unset'}`"},
            ],
        },
        {
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": f"*Runbook:* <{confluence_url}|{title}>",
            },
        },
        {"type": "divider"},
    ]

    # Enablement session prompt for GA bundles
    if not beta:
        blocks.append({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": (
                    ":mega: *This bundle is GA — please schedule a Technical Enablement session.*\n"
                    "Coordinate with the Solutions team to deliver enablement to customers and "
                    "internal teams before wide distribution."
                ),
            },
        })

    return {"blocks": blocks}


if __name__ == "__main__":
    main()
