SELECT datediff('s', reqTimeSec, now64(3)) AS hdx_source_latency_sec,
__SHARED_PROJECT___breadcrumbs(breadcrumbs, '(\\[[^[]*c=o[^]]*\\])', 'k=([^,\\]]+)') as origin_time_to_last_byte_ms,
positionCaseInsensitive(result_type, 'hit') > 0 AS cacheStatus,
IF(isIPv6String(assumeNotNull(cliIP)), 
   NULLIF(toString(dictGet('__SHARED_PROJECT___geoip_asn_blocks_ipv6', 'autonomous_system_number', IPv6StringToNumOrDefault(assumeNotNull(cliIP)))), '0'), 
   NULLIF(toString(dictGet('__SHARED_PROJECT___geoip_asn_blocks_ipv4', 'autonomous_system_number', toIPv4OrDefault(cliIP))), '0')) AS Edge_GeoInfo,
__SHARED_PROJECT___city_name(cliIP) as city,
dictGet('__SHARED_PROJECT___ua_cat_dict', 'ua_category', assumeNotNull(UA)) AS user_agent_category,
 * 
FROM {STREAM}