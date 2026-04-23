SELECT datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
result_type IN ('hit', 'stale', 'revalidated', 'updating') as cache_was_cached,
decodeURLComponent(queryString(request_full_path)) as request_query_string,
dictGet('__SHARED_PROJECT___ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
toString(response_status_code) AS response_status_code,
toString(client_asn) AS client_asn,
*
 FROM {STREAM}