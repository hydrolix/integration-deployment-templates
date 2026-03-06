SELECT datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
positionCaseInsensitive(result_type, 'hit') > 0 AS cache_was_cached,
round(response_time_to_first_byte_sec * 1000) AS response_time_to_first_byte_ms,
round(response_time_to_last_byte_sec * 1000) AS response_time_to_last_byte_ms,
IF(isIPv6String(assumeNotNull(client_ip)), NULLIF(toString(dictGet('akamai_geoip_asn_blocks_ipv6', 'autonomous_system_number', IPv6StringToNum(client_ip))),'0'), NULLIF(toString(dictGet('akamai_geoip_asn_blocks_ipv4', 'autonomous_system_number', toIPv4(client_ip))),'0')) as client_asn,
origin_time_to_first_byte_sec * 1000 AS origin_time_to_first_byte_ms,
origin_time_to_last_byte_sec * 1000 AS origin_time_to_last_byte_ms,
 akamai_city_name(client_ip) as client_city,
 multiIf(positionCaseInsensitive(assumeNotNull(request_path), 'robots.txt') > 0, 'robots.txt',
    positionCaseInsensitive(assumeNotNull(request_path), 'llms.txt') > 0, 'llms.txt',
    'other')
AS resource_category,
dictGet('akamai_ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
dictGet('akamai_ua_cat_dict', 'is_bot', assumeNotNull(user_agent)) AS is_bot_traffic,
 * 
FROM {STREAM}