SELECT 
arrayMap(x -> REPLACE(x, '\"', ''), 
JSONExtractArrayRaw(assumeNotNull(metadataTagsString))) AS metadataTags,
arrayMap(x -> REPLACE(x, '\"', ''), 
JSONExtractArrayRaw(assumeNotNull(projectMetaTagsString))) AS projectMetaTags,
arrayMap(x -> REPLACE(x, '\"', ''), 
JSONExtractArrayRaw(assumeNotNull(contextOwnersString))) AS contextOwners,
arrayMap(x -> REPLACE(x, '\"', ''), 
JSONExtractArrayRaw(assumeNotNull(contextReferencesString))) AS contextReferences,
arrayMap(x -> REPLACE(x, '\"', ''), 
JSONExtractArrayRaw(assumeNotNull(contextViewedString))) AS contextViewed,
* FROM {STREAM}
