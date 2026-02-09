# How to Contribute Integrations

This guide outlines the process for external teams to contribute integration assets to the Hydrolix console.

## Contribution Workflow

### 1. Create a Jira Ticket

Create a Jira ticket in the **LOTC** project for your integration request.

**Jira Project**: [LOTC Board](https://hydrolix.atlassian.net/jira/software/projects/LOTC/boards/635)

Include the following information in your ticket:

#### Required Information

**Basic Details**
- **Integration name**: Technical name (lowercase, underscores allowed, e.g., `cdn_insights`)
- **Display title**: User-facing name (e.g., "CDN Insights")
- **Description**: Clear explanation of what the integration does and its purpose
- **Category**: AWS or TrafficPeak
- **Data category**: Type of data (e.g., security, observability, analytics)

**Table Configuration**
- **Primary table name**: Name for the main table (e.g., `cdn_insights_logs`)
- **Summary table names** (if applicable): Names for any summary/aggregation tables

**UI Assets**
- **Integration logo**: PNG image or URL for the integration icon
- **Method logo** (if applicable): Icon representing the data ingestion method
- **Documentation URL**: Link to integration documentation (if available)

**Technical Details**
- **Ingestion method**: How data will be ingested (e.g., `http_streaming`, `firehose`, `multi_stream`)
- **Dependencies**: Any shared dictionaries or functions required (e.g., GeoIP, user-agent parsing)
- **New or updated dictionaries**: If providing new dictionaries or updates to existing ones, specify the dictionary names and describe the changes
- **New or updated functions**: If providing new functions or updates to existing ones, specify the function names and describe their purpose
- **Sample data**: Representative data samples for each transformation

#### Optional Information
- **Maintainer contact**: Email address for the integration maintainer
- **Beta status**: Whether this integration is in beta
- **Multiple dashboards**: If providing multiple dashboard views (e.g., overview, detailed, raw)

### 2. Create a Branch

Create a branch named with your ticket number and a brief description:

```bash
git checkout main
git pull origin main
git checkout -b TICKET-123-integration-name
```

**Example:** `JIRA-456-cloudflare-logs`

### 3. Prepare and Load Assets

Organize your assets following the bundle structure. Assets should be placed in a newly created folder under either `aws/` or `trafficpeak/` depending on your integration category.

#### Repository Structure
```
integration-deployment-templates/
├── aws/
│   └── your-project-name/
│       ├── dashboards/
│       ├── dictionaries/
│       ├── functions/
│       ├── transformations/
│       └── summaries/
└── trafficpeak/
    └── your-project-name/
        ├── dashboards/
        ├── dictionaries/
        ├── functions/
        ├── transformations/
        └── summaries/
```

#### Asset Types and Formats

**Dashboards**
- Grafana JSON dashboard files
- Include primary dashboard and any additional views (global, detailed, raw, etc.)
- Place in `dashboards/` folder

**Dictionaries**
- CSV files with key-value mappings
- Used for lookups and data enrichment
- Place in `dictionaries/` folder

**Functions**
- SQL function definitions
- Custom functions used in transformations or queries
- Place in `functions/` folder

**Transformations**
- Transformation configuration JSON files
- **Must include sample data** (`sample_data.json`) for each transformation
- Organize by ingestion method or data source (e.g., `transformations/cloudflare/`, `transformations/firehose/`)
- Place in `transformations/` folder

**Summaries**
- SQL files for summary/aggregation table definitions
- Used for pre-computed metrics and roll-ups
- Place in `summaries/` folder

**Logos**
- PNG format recommended
- Minimum 200x200 pixels for clarity
- Transparent background preferred
- Can be provided as files or publicly accessible URLs

### 4. Commit and Push Assets

```bash
git add .
git commit -m "Add [integration name] assets for TICKET-123"
git push origin TICKET-123-integration-name
```

### 5. Create a Pull Request

Create a pull request to the `main` branch for asset review:

1. Go to the repository on GitHub
2. Create a new Pull Request from your branch to `main`
3. Use the title format: `[TICKET-123] Integration Name`
4. In the PR description:
   - Link to your Jira ticket
   - Summarize the integration and assets provided
   - Note any dependencies or special considerations
5. Request review from the Integration Engineer

**Note**: This PR will remain open throughout the process. The Integration Engineer will add bundling configuration and validation to your branch before merging. The PR will only be merged once the complete integration (assets + bundling) is tested and validated.

### 6. Assign to Integration Engineer

Update your Jira ticket and assign it to an integration engineer on the SaaS Engineering team. The team will:
- Review your submitted assets via the PR
- Provide feedback through PR comments and Jira
- Once assets are approved, add bundling work to your branch
- Create the `bundle.json` configuration file
- Adapt assets for validation and deployment
- Run validation checks

### 7. Wait for Feedback

The Integration Engineer will provide feedback through both the Pull Request and Jira ticket. You may receive:

**Technical Validation Feedback**
- Asset format issues
- Schema validation errors
- Configuration problems

**Functional Testing Feedback**
- Runtime errors
- Data processing issues
- Integration compatibility problems

### 8. Update Assets Based on Feedback

If issues are identified with the provided assets:

1. Make necessary corrections to your assets
2. Commit and push updates to your branch
```bash
git add .
git commit -m "Update assets based on feedback for TICKET-123"
git push origin TICKET-123-integration-name
```
3. Comment on the PR and/or Jira ticket to notify the Integration Engineer
4. The PR will automatically update with your new commits

### 9. Bundling and Final Validation

Once your assets are approved:
- The Integration Engineer will continue work on **your branch**
- They will add bundling configuration (`bundle.json`) and any necessary adaptations
- Additional commits will appear in your PR from the Integration Engineer
- The Integration Engineer will run validation and testing
- The PR will be merged to `main` only when the complete integration is validated and ready for deployment

You will receive confirmation via the PR and Jira ticket when the integration is complete.

## Best Practices

- **Follow naming conventions**: Use clear, descriptive names for all asset files
- **Test locally**: Validate SQL syntax and JSON formatting before submission
- **Document dependencies**: Note any external dependencies or data requirements
- **Provide examples**: Include sample data or usage examples where applicable
- **Respond promptly**: Address feedback quickly to expedite the integration process
- **Use PR comments**: Ask questions and clarify feedback directly on the PR for better context
- **Monitor your branch**: The Integration Engineer will add commits to your branch—this is expected and part of the workflow

## Need Help?

If you have questions or need assistance:
- Comment on your Jira ticket
- Contact an integration engineer on the SaaS engineering team (Slack: @eng-marketplace)
- Refer to existing integrations in the repository for examples
