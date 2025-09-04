# Hydrolix Bundles

## What is a Bundle?

A Hydrolix Bundle is a structured JSON configuration file that defines a complete data integration package for a given Data Source for the Hydrolix platform.

It serves as a blueprint for connecting data sources, transforming data, creating visualizations, and managing dependencies.

The specification of the Bundle format is located here: [BUNDLE-DETAILS.md](https://github.com/hydrolix/integration-deployment-templates/blob/main/BUNDLE-DETAILS.md)

Instructions on how to use validation tools to validate Bundles is here: [HOW-TO-TEST.md](https://github.com/hydrolix/integration-deployment-templates/blob/main/HOW-TO-TEST.md)


An explination of what is verified during the validation process is here: [WHAT-IS-CHECKED.md](https://github.com/hydrolix/integration-deployment-templates/blob/main/WHAT-IS-CHECKED.md)

## Key Components

- **Data Source Configuration** - Defines where data originates (CloudFront, WAF, VPC, etc.)
- **Integration Method** - Specifies how data is ingested (Kinesis, S3, HTTP, etc.)
- **Transformations** - Data processing and enrichment rules
- **Dashboard Templates** - Pre-built visualizations (usually Grafana)
- **Metadata** - Versioning, maintainer information, and descriptions
- **Dependencies** - Required plugins, versions, and external resources

## Why Bundles are Important

### 1. **Standardization**
- Provides consistent structure across all integrations
- Ensures compatibility with Hydrolix platform components
- Enables automated validation and deployment

### 2. **Reusability** 
- Packages complex integrations as shareable components
- Allows teams to reuse battle-tested configurations
- Reduces implementation time for common use cases

### 3. **Dependency Management**
- Explicitly defines required software versions
- Manages external resources (GeoIP databases, plugins)
- Prevents version conflicts and compatibility issues

### 4. **Validation & Safety**
- Built-in validation prevents misconfigurations
- Security checks ensure proper data validation
- Macro variable system prevents template errors

### 5. **Documentation**
- Self-documenting structure
- Clear metadata about purpose and maintainers
- Version tracking for updates and maintenance


