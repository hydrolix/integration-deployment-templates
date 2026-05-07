SELECT datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
result_type IN ('hit', 'stale', 'revalidated', 'updating') as cache_was_cached,
decodeURLComponent(queryString(request_full_path)) as request_query_string,
dictGet('commons_ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
dictGet('commons_ua_cat_dict', 'is_bot', assumeNotNull(user_agent)) AS is_bot_traffic,
*
 FROM {STREAM}