SELECT datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
positionCaseInsensitive(result_type, 'hit') > 0 AS cache_was_cached,
if(varnish_tracing='b',true,false) AS is_origin_request,
concat(request_path, request_query_string) AS request_full_path,
if(NOT is_origin_request, toUInt64(varnish_time_firstbyte_sec * 1000.0), NULL) AS response_time_to_first_byte_ms,
if(NOT is_origin_request, varnish_time_to_last_byte_ms, NULL) AS response_time_to_last_byte_ms,
if(is_origin_request, toUInt64(varnish_time_firstbyte_sec * 1000.0), NULL) AS origin_time_to_first_byte_ms,
if(is_origin_request, varnish_time_to_last_byte_ms, NULL) AS origin_time_to_last_byte_ms,
if(varnish_client_asn != '', varnish_client_asn, IF(isIPv6String(assumeNotNull(client_ip)), NULLIF(toString(dictGet('__SHARED_PROJECT___geoip_asn_blocks_ipv6', 'autonomous_system_number', IPv6StringToNumOrDefault(assumeNotNull(client_ip)))),'0'), NULLIF(toString(dictGet('__SHARED_PROJECT___geoip_asn_blocks_ipv4', 'autonomous_system_number', toIPv4OrDefault(assumeNotNull(client_ip)))),'0'))) AS client_asn,
if(varnish_client_city != '', varnish_client_city, __SHARED_PROJECT___city_name(if(assumeNotNull(client_ip) IN ('', '-'), '0.0.0.0', client_ip))) AS client_city,
if(varnish_client_country_iso_code != '', varnish_client_country_iso_code, __SHARED_PROJECT___country_iso_code(if(assumeNotNull(client_ip) IN ('', '-'), '0.0.0.0', client_ip))) AS client_country_iso_code,
dictGet('__SHARED_PROJECT___ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
*
FROM {STREAM}