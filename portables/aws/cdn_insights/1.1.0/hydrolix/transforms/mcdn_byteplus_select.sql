SELECT datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
result_type = 'HIT' AS cache_was_cached,
toUInt64(toFloat64OrZero(ds_req_time) * 1000) AS response_time_to_last_byte_ms,
__SHARED_PROJECT___city_name(client_ip) AS client_city,
__SHARED_PROJECT___country_iso_code(client_ip) AS client_country_iso_code,
dictGet('__SHARED_PROJECT___ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
*
 FROM {STREAM}