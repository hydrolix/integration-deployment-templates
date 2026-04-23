SELECT
  datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
  positionCaseInsensitive(result_type, 'hit') > 0 AS cache_was_cached,
  __SHARED_PROJECT___city_name(client_ip) as client_city,
  dictGet('__SHARED_PROJECT___ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
 if(OriginResponseHeaderDuration >= 0, toUInt64(OriginResponseHeaderDuration), NULL) AS origin_time_to_first_byte_ms,
 *
FROM {STREAM}