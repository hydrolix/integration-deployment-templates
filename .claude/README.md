# Claude Code Skills for Integration Deployment Templates

This directory contains Claude Code skills specifically designed for working with Hydrolix integration deployment bundles.

## Available Skills

### `/configure-bundle`
**Primary skill for bundle configuration**

Guides you through the complete process of configuring a Hydrolix integration deployment bundle:

- **Phase 1: Discovery** - Identifies bundle structure and gathers requirements
- **Phase 2: bundle.json Creation** - Generates properly structured bundle.json with all metadata
- **Phase 3: Sample Data Validation** - Ensures sample_data.json is a single object (not array-wrapped)
- **Phase 4: Summary SQL Files** - Fixes hardcoded table references with template variables
- **Phase 5: Dashboard Configuration** - Validates dashboard structure and template variables
- **Phase 6: Dashboard Paths** - Updates bundle.json with correct dashboard paths
- **Phase 7: Validation Summary** - Provides comprehensive checklist of all changes

**When to use:** Setting up a new bundle or fixing an existing bundle's configuration

**Example:**
```
User: I need to configure the trafficpeak/security bundle
Claude: [Launches /configure-bundle skill and walks through all phases]
```

### `/bundle-checklist`
**Quick reference guide**

Displays a concise checklist of bundle configuration patterns without performing any actions. Useful for:
- Quick reminders of template variable patterns
- Understanding primary vs other dashboard differences
- Reference during manual configuration

**When to use:** You need a quick reminder of configuration patterns

### `/generate-bundle-json`
**Bundle.json template generator**

Generates a bundle.json template with proper structure and placeholders. Useful for:
- Starting a new bundle from scratch
- Creating a reference template
- Understanding bundle.json structure

**When to use:** Creating a new bundle and want a starting template

## Installation

These skills are automatically available when you run Claude Code from within this repository. No manual installation needed!

```bash
# Navigate to the repository
cd integration-deployment-templates

# Start Claude Code
claude

# Use any skill
/configure-bundle
```

## Skill Updates

Since these skills are version-controlled with the repository:
- `git pull` automatically updates skills for all team members
- Changes to skills can be reviewed in pull requests
- Skill versions stay in sync with repository conventions

## Key Features

### Automatic Detection
Skills automatically detect:
- Bundle directory structure (dashboards/, summaries/, transformations/, functions/)
- Existing bundle.json files
- Dashboard template variables
- Summary table definitions
- Shared function dependencies

### Template Variables
Skills understand all Hydrolix template variables:
- `__PROJECT_NAME__` - Project name
- `__TABLE_NAME__` - Base table name
- `__SUMMARY_TABLE_NAME_X__` - Summary table names
- `__DATASOURCE__` - Datasource UID
- `__DASHBOARD_UUID__` - Dashboard UID

### Critical Pattern: Dashboard Variables
Skills implement the critical distinction between primary and other dashboards:
- **Primary dashboard:** Summary variables use `__SUMMARY_TABLE_NAME_X__` (no prefix)
- **Other dashboards:** Summary variables use `__PROJECT_NAME__.__SUMMARY_TABLE_NAME_X__` (with prefix)

This difference exists because the Hydrolix validator processes dashboards differently.

## Sample Data Format Validation

The `/configure-bundle` skill automatically checks and fixes a common issue:

**Wrong:**
```json
[
  {
    "field1": "value1",
    "field2": "value2"
  }
]
```

**Correct:**
```json
{
  "field1": "value1",
  "field2": "value2"
}
```

The deployment system requires sample_data.json to be a single object, not an array.

## Contributing

When adding new bundle configuration patterns or requirements:

1. Update the relevant skill file in `.claude/skills/`
2. Test the skill on actual bundles
3. Document changes in this README
4. Submit pull request for review

## Support

For issues or questions about these skills:
- Open an issue in the repository
- Contact: kevin.borkman@hydrolix.io
- Reference: BUNDLE-DETAILS.md, HOW-TO-CONTRIBUTE.md

## Version History

- **v1.0** (2025-02-03) - Initial release
  - configure-bundle skill with 7-phase configuration process
  - Sample data validation and fixing
  - Dashboard template variable patterns
  - Summary table configuration
  - bundle-checklist quick reference
  - generate-bundle-json template generator
