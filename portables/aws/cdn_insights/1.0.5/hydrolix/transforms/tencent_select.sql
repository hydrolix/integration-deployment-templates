SELECT datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
positionCaseInsensitive(result_type, 'hit') > 0 AS cache_was_cached,
commons_city_name(client_ip) as client_city,
dictGet('commons_ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
 * 
FROM {STREAM}