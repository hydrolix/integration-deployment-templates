// Check if required functions and dictionaries exist in Hydrolix (Production Mode)
// COMPLETE FILE - Replace entire hdx_check_dependencies.ts with this
// UPDATED: Check with underscore prefix

import type { Bundle } from "./types/bundle.ts";

const BUNDLE_TESTING_CLUSTER = Deno.env.get("BUNDLE_TESTING_CLUSTER") || "";
const ORG_UUID = "b646d78a-5fb2-4d5f-afef-b705bf185174";
const PROJ_UUID = "469dbd34-6f06-4dfe-8fd1-9adf82123ecf";
const PROJ_NAME = "sample_project";

export async function checkDependenciesExist(
  bearerToken: string,
  bundle: Bundle,
  baseDir: string
): Promise<void> {
  const missingFunctions: string[] = [];
  const missingDictionaries: string[] = [];
  const missingFiles: string[] = [];
  
  // Check functions
  if (bundle.dependencies?.hydrolix?.required_functions) {
    const functionsUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/functions/`;
    
    try {
      const response = await fetch(functionsUrl, {
        headers: { 'Authorization': `Bearer ${bearerToken}` },
      });
      
      if (response.ok) {
        const responseData = await response.json();
        
        let existingFunctions: Array<{ name: string }> = [];
        if (Array.isArray(responseData)) {
          existingFunctions = responseData;
        } else if (responseData?.functions && Array.isArray(responseData.functions)) {
          existingFunctions = responseData.functions;
        } else if (responseData?.data && Array.isArray(responseData.data)) {
          existingFunctions = responseData.data;
        }
        
        const existingNames = new Set(existingFunctions.map(f => f.name));
        
        for (const functionName of bundle.dependencies.hydrolix.required_functions) {
          const fullName = `${PROJ_NAME}_${functionName}`;  // sample_project_city_name
          
          if (!existingNames.has(fullName)) {
            missingFunctions.push(functionName);
          }
          
          const filePath = `${baseDir}/functions/${functionName}.json`;
          try {
            await Deno.stat(filePath);
          } catch {
            missingFiles.push(`functions/${functionName}.json`);
          }
        }
      } else {
        throw new Error(`Failed to list functions: ${response.statusText}`);
      }
    } catch (e) {
      throw new Error(`Failed to check functions: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
  
  // Check dictionaries
  if (bundle.dependencies?.hydrolix?.required_dictionaries) {
    const dictsUrl = `https://${BUNDLE_TESTING_CLUSTER}/config/v1/orgs/${ORG_UUID}/projects/${PROJ_UUID}/dictionaries/`;
    
    try {
      const response = await fetch(dictsUrl, {
        headers: { 'Authorization': `Bearer ${bearerToken}` },
      });
      
      if (response.ok) {
        const responseData = await response.json();
        
        let existingDicts: Array<{ name: string }> = [];
        if (Array.isArray(responseData)) {
          existingDicts = responseData;
        } else if (responseData?.dictionaries && Array.isArray(responseData.dictionaries)) {
          existingDicts = responseData.dictionaries;
        } else if (responseData?.data && Array.isArray(responseData.data)) {
          existingDicts = responseData.data;
        }
        
        const existingNames = new Set(existingDicts.map(d => d.name));
        
        for (const dictionaryName of bundle.dependencies.hydrolix.required_dictionaries) {
          const fullName = `${PROJ_NAME}_${dictionaryName}`;  // sample_project_ua_cat_dict
          
          if (!existingNames.has(fullName)) {
            missingDictionaries.push(dictionaryName);
          }
          
          const jsonPath = `${baseDir}/dictionaries/${dictionaryName}.json`;
          try {
            await Deno.stat(jsonPath);
          } catch {
            missingFiles.push(`dictionaries/${dictionaryName}.json`);
          }
          
          const possibleExtensions = ['csv', 'yaml', 'yml', 'tsv'];
          let foundDataFile = false;
          for (const ext of possibleExtensions) {
            try {
              await Deno.stat(`${baseDir}/dictionaries/${dictionaryName}.${ext}`);
              foundDataFile = true;
              break;
            } catch {
              // Try next
            }
          }
          if (!foundDataFile) {
            missingFiles.push(`dictionaries/${dictionaryName}.[csv/yaml/yml/tsv]`);
          }
        }
      } else {
        throw new Error(`Failed to list dictionaries: ${response.statusText}`);
      }
    } catch (e) {
      throw new Error(`Failed to check dictionaries: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
  
  // Report results
  const errors: string[] = [];
  
  if (missingFunctions.length > 0) {
    errors.push(`\n❌ Missing functions on cluster:`);
    missingFunctions.forEach(name => {
      errors.push(`   - ${name} (expected as: ${PROJ_NAME}_${name})`);
    });
  }
  
  if (missingDictionaries.length > 0) {
    errors.push(`\n❌ Missing dictionaries on cluster:`);
    missingDictionaries.forEach(name => {
      errors.push(`   - ${name} (expected as: ${PROJ_NAME}_${name})`);
    });
  }
  
  if (missingFiles.length > 0) {
    errors.push(`\n⚠️  Missing local definition files:`);
    missingFiles.forEach(file => {
      errors.push(`   - ${file}`);
    });
  }
  
  if (errors.length > 0) {
    errors.push('\n📋 In production mode:');
    if (missingFunctions.length > 0 || missingDictionaries.length > 0) {
      errors.push('   • Resources must exist on cluster before deployment');
      errors.push('   • Either create them manually or run without --production flag first');
    }
    if (missingFiles.length > 0) {
      errors.push('   • Local files should be included for documentation and validation');
    }
    
    throw new Error(errors.join('\n'));
  }
  
  console.log('✓ All required dependencies exist on cluster');
  console.log('✓ All required local files present');
}