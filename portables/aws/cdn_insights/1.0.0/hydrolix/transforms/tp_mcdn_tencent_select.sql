SELECT datediff('s', reqTimeSec, now64(3)) AS hdx_source_latency_sec,
positionCaseInsensitive(result_type, 'hit') > 0 as cacheStatus,
__SHARED_PROJECT___city_name(cliIP) as city,
dictGet('__SHARED_PROJECT___ua_cat_dict', 'ua_category', assumeNotNull(user_agent_category)) AS user_agent_category,
__SHARED_PROJECT___breadcrumbs(breadcrumbs, '(\\[[^[]*c=o[^]]*\\])', 'l=([^,\\]]+)') as origin_time_to_first_byte_ms,
__SHARED_PROJECT___breadcrumbs(breadcrumbs, '(\\[[^[]*c=o[^]]*\\])', 'k=([^,\\]]+)') as origin_time_to_last_byte_ms,
 * 
FROM {STREAM}