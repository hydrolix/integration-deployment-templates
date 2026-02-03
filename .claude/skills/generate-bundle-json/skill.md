---
name: generate-bundle-json
description: Generate a bundle.json template for a Hydrolix integration bundle. Use when the user wants to create a new bundle.json file.
user-invocable: true
---

# Generate bundle.json Template

When the user requests a bundle.json template, ask for these details:

1. **Source name** (e.g., "trafficpeak", "akamai")
2. **Bundle name** (e.g., "siem", "cdn", "logs")
3. **Table name** (e.g., "siem_logs", "cdn_logs")
4. **Maintainer email**
5. **Number of dashboards** (how many JSON files in dashboards/)
6. **Number of summary tables** (how many SQL files in summaries/)
7. **Shared functions** (comma-separated list, if any)

Then generate a complete bundle.json with all the proper structure.

## Template

```json
{
  "base_url": "https://github.com/hydrolix/integration-deployment-templates/blob/main/{source}/{bundle_name}",
  "beta": true,
  "dashboard": {
    "path": "dashboards/{FIRST_DASHBOARD}.json",
    "project_var": "__PROJECT_NAME__"
  },
  "dependencies": {
    "hydrolix": {
      "required_dictionaries": [],
      "required_functions": [],
      "shared_dictionaries": [],
      "shared_functions": {SHARED_FUNCTIONS_ARRAY}
    }
  },
  "metadata": {
    "channel_type": "AWS",
    "description": "{SOURCE} {BUNDLE_NAME} Integration",
    "maintainer": "{EMAIL}",
    "version": "1.0.0"
  },
  "method": "http_streaming",
  "name": "{source}_{bundle_name}",
  "other_dashboards": {OTHER_DASHBOARDS_ARRAY},
  "solution": true,
  "source": "{source}",
  "summary_tables": {SUMMARY_TABLES_ARRAY},
  "tables": [
    {
      "dashboard_var": "__TABLE_NAME__",
      "name": "{table_name}",
      "transforms": [
        {
          "method": "http_streaming",
          "path": "transformations/transform.json",
          "sample": "transformations/sample_data.json"
        }
      ]
    }
  ],
  "ui": {
    "data_category": "security",
    "method": {
      "full_title": "Http Streaming",
      "icon_url": "https://hydrolix-public.s3.us-east-2.amazonaws.com/partner_logos/http.png"
    },
    "primary_url": "https://docs.hydrolix.io/docs/{source}_{bundle_name}-integration",
    "source": {
      "full_title": "{SOURCE} {BUNDLE_NAME}",
      "icon_url": "https://hydrolix-public.s3.us-east-2.amazonaws.com/partner_logos/{source}.png"
    }
  }
}
```

After generating, save it to the bundle directory and remind the user to:
1. Verify ui.source.full_title is unique
2. Update dashboard paths to match actual filenames
3. Add summary table definitions if they exist
