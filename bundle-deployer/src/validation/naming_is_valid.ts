// Validation: Check that naming conventions are followed

import type { Bundle } from "../types/bundle.ts";

export function run(bundle: Bundle): void {
  // Check method consistency
  const method = bundle.method;
  const methodTitle = bundle.ui.method.full_title;

  const expectedTitles: Record<string, string[]> = {
    "firehose": [
      "Amazon Data Firehose",
      "AWS Firehose",
      "Kinesis Data Firehose",
    ],
    "s3": ["Amazon S3", "AWS S3"],
    "kinesis": ["Amazon Kinesis", "AWS Kinesis"],
  };

  const titles = expectedTitles[method] || [];
  
  if (titles.length > 0 && !titles.some(title => methodTitle.includes(title))) {
    throw new Error(
      `docs.method.full_title '${methodTitle}' does not match method '${method}'`
    );
  }

  // Check source consistency
  const source = bundle.source;
  const sourceTitle = bundle.ui.source.full_title;

  if (source === "waf" && !sourceTitle.toLowerCase().includes("waf")) {
    throw new Error("Source title should contain 'WAF' when source is 'waf'");
  }

  // Check name consistency with source and method
  const name = bundle.name;
  const nameLower = name.toLowerCase();

  if (!nameLower.includes(source.toLowerCase()) || 
      !nameLower.includes(method.toLowerCase())) {
    throw new Error(
      `Name '${name}' must include '${source}' and '${method}'`
    );
  }

  // Check version format (basic semantic versioning)
  const version = bundle.metadata.version;
  const dotCount = (version.match(/\./g) || []).length;
  
  if (dotCount !== 2) {
    throw new Error(
      `Version ${version} should follow semantic versioning format (e.g., 1.0.0)`
    );
  }

  // Check maintainer email format
  const maintainer = bundle.metadata.maintainer;
  if (!maintainer.includes('@') || !maintainer.includes('.')) {
    throw new Error("Maintainer should be a valid email address");
  }

  // Check description is not empty
  if (bundle.metadata.description.trim().length === 0) {
    throw new Error("Description cannot be empty");
  }
}