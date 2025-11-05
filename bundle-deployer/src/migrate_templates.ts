// Automatic migration tool to update template variables based on resource type
// Usage: deno run --allow-all migrate_templates.ts <bundle-name>

import type { Bundle } from "./types/bundle.ts";
import { getAllDictionaries, getAllFunctions } from "./types/bundle.ts";

async function migrateBundle(bundlePath: string): Promise<void> {
  console.log(`\n🔄 Migrating bundle: ${bundlePath}\n`);

  // Load bundle
  const bundleContent = await Deno.readTextFile(`${bundlePath}/bundle.json`);
  const bundle: Bundle = JSON.parse(bundleContent);

  // Extract resource categorization from bundle.json
  const { bundleSpecific: bundleFuncs, shared: sharedFuncs } = getAllFunctions(bundle);
  const { bundleSpecific: bundleDicts, shared: sharedDicts } = getAllDictionaries(bundle);

  console.log(`📊 Resource inventory from bundle.json:`);
  console.log(`   Bundle-specific functions: ${bundleFuncs.length}`);
  console.log(`   Shared functions: ${sharedFuncs.length}`);
  console.log(`   Bundle-specific dictionaries: ${bundleDicts.length}`);
  console.log(`   Shared dictionaries: ${sharedDicts.length}\n`);
  
  if (sharedFuncs.length === 0 && sharedDicts.length === 0) {
    console.log(`⚠️  No shared resources declared in bundle.json`);
    console.log(`   If you have shared resources, add them to bundle.json first:\n`);
    console.log(`   "dependencies": {`);
    console.log(`     "hydrolix": {`);
    console.log(`       "shared_functions": ["city_name", "breadcrumbs"],`);
    console.log(`       "shared_dictionaries": ["geoip_city", "ua_categories"]`);
    console.log(`     }`);
    console.log(`   }\n`);
    console.log(`   Proceeding with migration (no changes will be made)...\n`);
  }

  // Migrate transform files
  for (const table of bundle.tables) {
    for (const transform of table.transforms) {
      const transformPath = `${bundlePath}/${transform.path}`;
      await migrateTransformFile(
        transformPath,
        new Set(sharedFuncs),
        new Set(sharedDicts)
      );
    }
  }

  // Migrate function files (if they reference other functions/dicts)
  const allFunctions = [...bundleFuncs, ...sharedFuncs];
  for (const funcName of allFunctions) {
    const funcPath = `${bundlePath}/functions/${funcName}.json`;
    try {
      await migrateFunctionFile(
        funcPath,
        new Set(sharedFuncs),
        new Set(sharedDicts)
      );
    } catch {
      // Function file might not exist locally
    }
  }

  console.log(`\n✅ Migration complete!`);
}

async function migrateTransformFile(
  filePath: string,
  sharedFunctions: Set<string>,
  sharedDictionaries: Set<string>
): Promise<void> {
  console.log(`🔍 Migrating: ${filePath}`);

  const content = await Deno.readTextFile(filePath);
  const transform = JSON.parse(content);

  const sql = transform?.settings?.sql_transform;
  if (!sql || typeof sql !== 'string') {
    console.log(`   ⏭️  Skipped - no SQL transform`);
    return;
  }

  let updated = sql;
  let changes = 0;

  // Migrate shared function calls
  // Pattern: __PROJECT_NAME___function_name( → __SHARED_PROJECT___function_name(
  for (const funcName of sharedFunctions) {
    const oldPattern = new RegExp(`__PROJECT_NAME___${funcName}\\s*\\(`, 'g');
    const newPattern = `__SHARED_PROJECT___${funcName}(`;
    
    if (oldPattern.test(updated)) {
      updated = updated.replace(oldPattern, newPattern);
      changes++;
      console.log(`   ✏️  Updated function: ${funcName} → __SHARED_PROJECT__`);
    }
  }

  // Migrate shared dictionary references
  // Pattern: dictGet('__PROJECT_NAME___dict_name' → dictGet('__SHARED_PROJECT___dict_name'
  for (const dictName of sharedDictionaries) {
    const oldPattern = new RegExp(`dictGet(?:String|OrDefault)?\\s*\\(\\s*['"]__PROJECT_NAME___${dictName}['"]`, 'g');
    
    if (oldPattern.test(updated)) {
      updated = updated.replace(
        new RegExp(`'__PROJECT_NAME___${dictName}'`, 'g'),
        `'__SHARED_PROJECT___${dictName}'`
      );
      updated = updated.replace(
        new RegExp(`"__PROJECT_NAME___${dictName}"`, 'g'),
        `"__SHARED_PROJECT___${dictName}"`
      );
      changes++;
      console.log(`   ✏️  Updated dictionary: ${dictName} → __SHARED_PROJECT__`);
    }
  }

  if (changes === 0) {
    console.log(`   ✅ No changes needed`);
    return;
  }

  // Write updated file
  transform.settings.sql_transform = updated;
  await Deno.writeTextFile(
    filePath,
    JSON.stringify(transform, null, 2) + '\n'
  );

  console.log(`   ✅ Saved with ${changes} change(s)`);
}

async function migrateFunctionFile(
  filePath: string,
  sharedFunctions: Set<string>,
  sharedDictionaries: Set<string>
): Promise<void> {
  console.log(`🔍 Migrating function: ${filePath}`);

  const content = await Deno.readTextFile(filePath);
  const func = JSON.parse(content);

  const sql = func?.sql;
  if (!sql || typeof sql !== 'string') {
    console.log(`   ⏭️  Skipped - no SQL`);
    return;
  }

  let updated = sql;
  let changes = 0;

  // Migrate shared function calls (functions calling other functions)
  for (const funcName of sharedFunctions) {
    const oldPattern = new RegExp(`__PROJECT_NAME___${funcName}\\s*\\(`, 'g');
    const newPattern = `__SHARED_PROJECT___${funcName}(`;
    
    if (oldPattern.test(updated)) {
      updated = updated.replace(oldPattern, newPattern);
      changes++;
      console.log(`   ✏️  Updated function: ${funcName} → __SHARED_PROJECT__`);
    }
  }

  // Migrate dictionary references
  for (const dictName of sharedDictionaries) {
    const oldPattern = new RegExp(`dictGet(?:String|OrDefault)?\\s*\\(\\s*['"]__PROJECT_NAME___${dictName}['"]`, 'g');
    
    if (oldPattern.test(updated)) {
      updated = updated.replace(
        new RegExp(`'__PROJECT_NAME___${dictName}'`, 'g'),
        `'__SHARED_PROJECT___${dictName}'`
      );
      updated = updated.replace(
        new RegExp(`"__PROJECT_NAME___${dictName}"`, 'g'),
        `"__SHARED_PROJECT___${dictName}"`
      );
      changes++;
      console.log(`   ✏️  Updated dictionary: ${dictName} → __SHARED_PROJECT__`);
    }
  }

  if (changes === 0) {
    console.log(`   ✅ No changes needed`);
    return;
  }

  // Write updated file
  func.sql = updated;
  await Deno.writeTextFile(
    filePath,
    JSON.stringify(func, null, 2) + '\n'
  );

  console.log(`   ✅ Saved with ${changes} change(s)`);
}

// Main entry point
if (import.meta.main) {
  const bundleName = Deno.args[0];
  
  if (!bundleName) {
    console.error("Usage: deno run --allow-all migrate_templates.ts <bundle-name>");
    Deno.exit(1);
  }

  const bundlePath = `my-bundles/${bundleName}`;
  
  try {
    await Deno.stat(`${bundlePath}/bundle.json`);
  } catch {
    console.error(`❌ Bundle not found: ${bundlePath}`);
    Deno.exit(1);
  }

  await migrateBundle(bundlePath);
}