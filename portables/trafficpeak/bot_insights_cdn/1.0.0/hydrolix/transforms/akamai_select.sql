SELECT datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
akamai_breadcrumbs(breadcrumbs, '(\[[^[]*c=o[^]]*\])', 'k=([^,\]]+)') as origin_time_to_last_byte_ms,
akamai_breadcrumbs(breadcrumbs, '(\[[^[]*c=g[^]]*\])', 'n=([^,\]]+)') as edge_pop,
akamai_breadcrumbs(breadcrumbs, '(\[[^[]*c=o[^]]*\])', 'a=([^,\]]+)') AS origin_ip,
akamai_breadcrumbs(breadcrumbs, '(\[[^[]*c=o[^]]*\])', 'l=([^,\]]+)') AS origin_time_to_first_byte_ms,
akamai_breadcrumbs(breadcrumbs, '(\[[^[]*c=g[^]]*\])', 'a=([^,\]]+)') as edge_ip,
multiIf(
cacheStatus=1,1,
breadcrumbs IS NULL OR empty(breadcrumbs),0,
origin_ip IS NULL OR empty(origin_ip) OR origin_ip = '127.0.0.1',1,0
) AS cache_was_cached,
decodeURLComponent(assumeNotNull(queryStr)) AS request_query_string,
concat('/', assumeNotNull(request_path)) as request_path,
decodeURLComponent(assumeNotNull(referer)) AS request_referer,
decodeURLComponent(assumeNotNull(UA)) as user_agent,
multiIf(positionCaseInsensitive(assumeNotNull(request_path), 'robots.txt') > 0, 'robots.txt',
    positionCaseInsensitive(assumeNotNull(request_path), 'llms.txt') > 0, 'llms.txt',
    'other')
AS resource_category,
dictGet('akamai_ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
dictGet('akamai_ua_cat_dict', 'is_bot', assumeNotNull(user_agent)) AS is_bot_traffic,
 * 
FROM {STREAM}