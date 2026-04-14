SELECT datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
positionCaseInsensitive(result_type, 'hit') > 0 AS cache_was_cached,
round(response_time_to_first_byte_sec * 1000) AS response_time_to_first_byte_ms,
round(response_time_to_last_byte_sec * 1000) AS response_time_to_last_byte_ms,
origin_time_to_first_byte_sec * 1000 AS origin_time_to_first_byte_ms,
origin_time_to_last_byte_sec * 1000 AS origin_time_to_last_byte_ms,
 __SHARED_PROJECT___city_name(client_ip) as client_city,
dictGet('__SHARED_PROJECT___ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
 * 
FROM {STREAM}