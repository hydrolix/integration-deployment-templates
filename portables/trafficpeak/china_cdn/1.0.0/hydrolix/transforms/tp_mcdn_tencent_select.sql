SELECT datediff('s', reqTimeSec, now64(3)) AS hdx_source_latency,
positionCaseInsensitive(result_type, 'hit') > 0 as cacheStatus,
akamai_city_name(cliIP) as city,
 * 
FROM {STREAM}