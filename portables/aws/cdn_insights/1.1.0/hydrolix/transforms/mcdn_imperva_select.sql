SELECT datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
result_type LIKE 'REQ_CACHED%' AS cache_was_cached,
path(concat('http://', assumeNotNull(request))) AS request_path,
queryString(concat('http://', assumeNotNull(request))) AS request_query_string,
concat(path(concat('http://', assumeNotNull(request))), IF(length(queryString(concat('http://', assumeNotNull(request)))) > 0, concat('?', queryString(concat('http://', assumeNotNull(request)))), '')) AS request_full_path,
dictGet('__SHARED_PROJECT___ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
IF(isIPv6String(assumeNotNull(client_ip)), NULLIF(toString(dictGet('__SHARED_PROJECT___geoip_asn_blocks_ipv6', 'autonomous_system_number', IPv6StringToNumOrDefault(client_ip))),'0'), NULLIF(toString(dictGet('__SHARED_PROJECT___geoip_asn_blocks_ipv4', 'autonomous_system_number', toIPv4OrDefault(client_ip))),'0')) AS client_asn,
 * 
FROM {STREAM}