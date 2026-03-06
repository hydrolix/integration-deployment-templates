SELECT datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
response_time_to_first_byte * 1000.0 as response_time_to_first_byte_ms,
response_time_to_last_byte * 1000.0 as response_time_to_last_byte_ms,
positionCaseInsensitive (result_type, 'hit') > 0 AS cache_was_cached,
splitByChar('?', assumeNotNull(original_url))[2] AS request_query_string,
splitByChar('?', assumeNotNull(original_url))[1] AS request_path,
multiIf(positionCaseInsensitive(assumeNotNull(request_path), 'robots.txt') > 0, 'robots.txt',
    positionCaseInsensitive(assumeNotNull(request_path), 'llms.txt') > 0, 'llms.txt',
    'other')
AS resource_category,
dictGet('akamai_ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
dictGet('akamai_ua_cat_dict', 'is_bot', assumeNotNull(user_agent)) AS is_bot_traffic,
 * 
FROM {STREAM}