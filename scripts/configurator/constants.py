"""Constants for bundle configuration."""

# Top-level Grafana folders
VALID_FOLDERS = (
    "api-context",
    "cdn",
    "dns",
    "media",
    "security",
)

# Valid subfolders nested under each folder
VALID_SUBFOLDERS = {
    "cdn": ("multi-cdn",),
    "security": ("bots", "ds2", "siem"),
}

# Keep legacy aliases so any remaining internal references don't break
VALID_CATEGORIES = VALID_FOLDERS
VALID_SUBCATEGORIES = VALID_SUBFOLDERS

# Prefix mapping based on bundle location
PREFIX_MAP = {
    "aws": "commons",
    "trafficpeak": "akamai",
}

# All known prefixes that should be replaced
KNOWN_PREFIXES = ("reference", "commons", "akamai")

# Valid bundle methods
VALID_METHODS = (
    "firehose",
    "s3",
    "kinesis",
    "lambda",
    "http_streaming",
    "http",
    "multi_stream",
)

# Valid channel types
VALID_CHANNEL_TYPES = ("AWS", "Azure", "GCP", "3rdParty", "Internal")

# Valid data categories
VALID_DATA_CATEGORIES = ("video", "cdn", "security")

# Maps data_category to its Grafana folder for CaC bundle export
DATA_CATEGORY_FOLDER_MAP = {
    "security": "security",
    "cdn":      "cdn",
    "video":    "media",
}

# Default channel type inference from source path
CHANNEL_TYPE_MAP = {
    "aws": "AWS",
    "trafficpeak": "3rdParty",
}

# Transform metadata fields to strip
TRANSFORM_METADATA_FIELDS = ("uuid", "created", "modified", "url", "table")

# Primary dashboard detection priority
PRIMARY_DASHBOARD_NAMES = ("home.json", "default.json", "overview.json")

# Method detection keywords in filenames/directory names
METHOD_KEYWORDS = {
    "firehose": "firehose",
    "kinesis": "kinesis",
}

# Method UI mapping
METHOD_UI = {
    "http_streaming": {
        "full_title": "Http Streaming",
        "icon_url": "https://hydrolix-public.s3.us-east-2.amazonaws.com/partner_logos/http.png",
    },
    "firehose": {
        "full_title": "Firehose",
        "icon_url": "https://hydrolix-public.s3.us-east-2.amazonaws.com/partner_logos/firehose.png",
    },
    "kinesis": {
        "full_title": "Kinesis",
        "icon_url": "https://hydrolix-public.s3.us-east-2.amazonaws.com/partner_logos/kinesis.png",
    },
    "multi_stream": {
        "full_title": "Http Streaming",
        "icon_url": "https://hydrolix-public.s3.us-east-2.amazonaws.com/partner_logos/http.png",
    },
}

# Grafana special datasource UIDs to preserve
GRAFANA_SPECIAL_UIDS = ("-- Grafana --", "-- Mixed --", "-- Dashboard --")

# Datasource template variable
DATASOURCE_TEMPLATE = "__DATASOURCE__"

# Dashboard UUID template
DASHBOARD_UUID_TEMPLATE = "__DASHBOARD_UUID__"

# Datasource element model
DATASOURCE_ELEMENT_MODEL = {
    "model": {
        "datasource": {
            "type": "hydrolix-hydrolix-datasource",
            "uid": "__DATASOURCE__",
        }
    }
}
