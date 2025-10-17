// Validation: Check for duplicates across ALL bundles

import type { Bundle } from "../types/bundle.ts";

export function run(bundles: Bundle[]): void {
  // Check for duplicated bundle names
  {
    const tokens = new Set<string>();
    for (const bundle of bundles) {
      if (tokens.has(bundle.name)) {
        throw new Error(
          `Duplicated-Bundle-Name url=${bundle.base_url} error=${bundle.name}`
        );
      }
      tokens.add(bundle.name);
    }
  }
  
  // Check for duplicated source names in the UI
  {
    const tokens = new Set<string>();
    for (const bundle of bundles) {
      if (tokens.has(bundle.ui.source.full_title)) {
        throw new Error(
          `Duplicated-UI-Source-Name url=${bundle.base_url} error=${bundle.ui.source.full_title}`
        );
      }
      tokens.add(bundle.ui.source.full_title);
    }
  }
  
  // Check for duplicated names across all bundles
  const tokens = new Map<string, number>();
  
  for (const bundle of bundles) {
    tokens.set(bundle.name, (tokens.get(bundle.name) || 0) + 1);
    
    for (const table of bundle.tables) {
      tokens.set(table.name, (tokens.get(table.name) || 0) + 1);
    }
    
    tokens.set(
      bundle.ui.source.full_title,
      (tokens.get(bundle.ui.source.full_title) || 0) + 1
    );
    
    tokens.set(bundle.base_url, (tokens.get(bundle.base_url) || 0) + 1);
  }
  
  // Check for duplicates
  for (const [name, count] of tokens.entries()) {
    if (count > 1) {
      throw new Error(`Duplicated-Name count=${count} table=${name}`);
    }
  }
}