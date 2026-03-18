"""Configuration dataclasses for bundle configurator."""

import os
import re
from dataclasses import dataclass, field
from typing import Optional

from .constants import CHANNEL_TYPE_MAP, PREFIX_MAP

SEMVER_RE = re.compile(r'^\d+\.\d+\.\d+$')

def is_semver(s: str) -> bool:
    """Check if a string is a semver version (e.g., '1.0.0')."""
    return bool(SEMVER_RE.match(s))


@dataclass
class BundleConfig:
    """Immutable configuration derived from CLI args."""

    bundle_dir: str
    table_name: str
    data_category: str
    source_name: str = ""
    bundle_name: str = ""
    channel_type: str = ""
    maintainer: str = "Hydrolix Team <team@hydrolix.io>"
    description: str = ""
    version: str = "1.0.0"
    method: str = ""
    primary_dashboard: str = ""
    beta: bool = True
    verbose: bool = False
    dry_run: bool = False

    def __post_init__(self):
        # Normalize bundle_dir to absolute path
        self.bundle_dir = os.path.abspath(self.bundle_dir)

        # Infer source_name from path if not provided
        if not self.source_name:
            self.source_name = self._infer_source_name()

        # Infer bundle_name from path if not provided
        if not self.bundle_name:
            self.bundle_name = self._infer_bundle_name()

        # Infer channel_type if not provided
        if not self.channel_type:
            self.channel_type = CHANNEL_TYPE_MAP.get(
                self.source_name.lower(), "AWS"
            )

        # Auto-generate description if not provided
        if not self.description:
            source_title = self.source_name.replace("_", " ").replace("-", " ").title()
            bundle_title = self.bundle_name.replace("_", " ").replace("-", " ").title()
            self.description = f"{source_title} {bundle_title} Integration"

    def _infer_source_name(self):
        """Infer source name from directory path."""
        parts = self.bundle_dir.rstrip("/").split("/")
        if len(parts) >= 3 and is_semver(parts[-1]):
            return parts[-3]
        if len(parts) >= 2:
            return parts[-2]
        return "unknown"

    def _infer_bundle_name(self):
        """Infer bundle name from directory path."""
        parts = self.bundle_dir.rstrip("/").split("/")
        if len(parts) >= 2 and is_semver(parts[-1]):
            return parts[-2]
        if parts:
            return parts[-1]
        return "unknown"

    @property
    def correct_prefix(self):
        """Get the correct SQL prefix for this bundle's location."""
        return PREFIX_MAP.get(self.source_name.lower(), "commons")

    @property
    def bundle_name_normalized(self):
        """Get bundle name normalized for bundle.json name field."""
        return re.sub(r"[^a-zA-Z0-9_]", "_", self.bundle_name)

    @property
    def base_url(self):
        """Generate the base_url for bundle.json."""
        parts = self.bundle_dir.rstrip("/").split("/")
        if is_semver(parts[-1]):
            return (
                f"https://github.com/hydrolix/integration-deployment-templates"
                f"/blob/main/{self.source_name}/{self.bundle_name}/{self.version}"
            )
        return (
            f"https://github.com/hydrolix/integration-deployment-templates"
            f"/blob/main/{self.source_name}/{self.bundle_name}"
        )


@dataclass
class TransformInfo:
    """Info about a discovered transform file."""

    original_path: str
    provider_name: str = ""
    has_sql_transform: bool = False
    has_sample_data: bool = False
    final_dir: str = ""
    final_path: str = ""
    sample_data_path: str = ""
    method: str = "http_streaming"
    shared_functions: list = field(default_factory=list)
    shared_dictionaries: list = field(default_factory=list)


@dataclass
class SummaryInfo:
    """Info about a discovered summary SQL file."""

    path: str
    filename: str = ""
    name: str = ""
    dashboard_var: str = ""


@dataclass
class DashboardInfo:
    """Info about a discovered dashboard file."""

    path: str
    filename: str = ""
    is_primary: bool = False


@dataclass
class BundleState:
    """Mutable state accumulated across phases."""

    transforms: list = field(default_factory=list)  # List[TransformInfo]
    summaries: list = field(default_factory=list)  # List[SummaryInfo]
    dashboards: list = field(default_factory=list)  # List[DashboardInfo]
    primary_dashboard: Optional[str] = None

    # Accumulated from SQL analysis
    all_shared_functions: list = field(default_factory=list)
    all_shared_dictionaries: list = field(default_factory=list)

    # Bundle method (detected or overridden)
    detected_method: str = ""

    # Tracking
    files_modified: list = field(default_factory=list)
    files_created: list = field(default_factory=list)
    files_renamed: list = field(default_factory=list)
    warnings: list = field(default_factory=list)
    errors: list = field(default_factory=list)
    phases_completed: list = field(default_factory=list)
