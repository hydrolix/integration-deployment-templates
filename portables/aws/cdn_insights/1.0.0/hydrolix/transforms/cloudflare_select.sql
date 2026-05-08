SELECT datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
datediff('millisecond', timestamp, edge_end_timestamp) AS response_time_to_last_byte_ms,
concat(edge_colo_code, '-', edge_colo_id) as edge_pop,
result_type IN ('hit', 'stale', 'revalidated', 'updating') as cache_was_cached,
decodeURLComponent(queryString(request_full_path)) as request_query_string,
dictGet('commons_ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
*
 FROM {STREAM}