SELECT datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
is_cached = 'true' AS cache_was_cached,
if(is_cached = 'true', 'hit', 'miss') AS result_type,
concat(request_path, if(request_query_string != '', concat('?', request_query_string), '')) AS request_full_path,
dictGet('__SHARED_PROJECT___ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
concat('IO River: ', multiIf(ioriver_provider = 'cloudfront', 'CloudFront', ioriver_provider = 'fastly', 'Fastly', ioriver_provider = 'akamai', 'Akamai', ioriver_provider = 'cloudflare', 'Cloudflare', ioriver_provider = 'gcore', 'Gcore', initCap(ioriver_provider))) AS hdx_cdn,
*
FROM {STREAM}