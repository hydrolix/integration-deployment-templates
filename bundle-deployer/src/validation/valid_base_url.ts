// Validation: Check that base_url follows expected format

import type { Bundle } from "../types/bundle.ts";

export function run(base: string, bundle: Bundle): void {
  const checkBaseUrl = 
    `https://github.com/hydrolix/integration-deployment-templates/blob/main/${base}`;
  
  if (bundle.base_url !== checkBaseUrl) {
    throw new Error(
      `Invalid ${bundle.name} base_url should be this: '${checkBaseUrl}'`
    );
  }
}