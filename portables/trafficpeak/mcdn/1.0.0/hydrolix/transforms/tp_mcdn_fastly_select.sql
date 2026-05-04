SELECT datediff('s', reqTimeSec, now64(3)) AS hdx_source_latency_sec,
positionCaseInsensitive(result_type, 'hit') > 0 as cacheStatus,
akamai_city_name(cliIP) as city,
dictGet('akamai_ua_cat_dict', 'ua_category', assumeNotNull(user_agent_category)) AS user_agent_category,
akamai_breadcrumbs(breadcrumbs, '(\\[[^[]*c=o[^]]*\\])', 'l=([^,\\]]+)') as origin_time_to_first_byte_ms,
akamai_breadcrumbs(breadcrumbs, '(\\[[^[]*c=o[^]]*\\])', 'k=([^,\\]]+)') as origin_time_to_last_byte_ms,
 * 
FROM {STREAM}