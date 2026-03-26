"""Data models for bundle conversion."""

from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Optional


@dataclass
class Transform:
    """Represents a transform file."""
    name: str
    file_path: Path
    table_name: str


@dataclass
class DashboardInput:
    """Represents a dashboard input variable."""
    name: str
    type: str  # 'datasource', 'constant', etc.
    value: Optional[str] = None


@dataclass
class Dashboard:
    """Represents a dashboard file."""
    filename: str
    file_path: Path
    relative_path: str  # Relative to dashboards/ folder
    folder_uid: str
    inputs: List[DashboardInput] = field(default_factory=list)


@dataclass
class Summary:
    """Represents a summary table."""
    name: str
    sql_file_path: Path


@dataclass
class BundleAssets:
    """Container for all discovered bundle assets."""
    transforms_folder: Optional[Path] = None
    dashboards_folder: Optional[Path] = None
    summaries_folder: Optional[Path] = None
    transforms: List[Transform] = field(default_factory=list)
    dashboards: List[Dashboard] = field(default_factory=list)
    summaries: List[Summary] = field(default_factory=list)


@dataclass
class BundleMetadata:
    """Metadata required for bundle generation."""
    customer_type: str = ""
    bundle_name: str = ""
    version: str = ""
    description: str = ""
    maintainer: str = ""
    table_name: str = "logs"
    home_dashboard: Optional[str] = None
    folder_path: List[str] = field(default_factory=list)
