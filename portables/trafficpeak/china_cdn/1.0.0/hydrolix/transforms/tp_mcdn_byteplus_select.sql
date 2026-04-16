SELECT 
datediff('s', reqTimeSec, now64(3)) AS hdx_source_latency,
positionCaseInsensitive(result_type, 'hit') > 0 AS cacheStatus,
dictGet('__SHARED_PROJECT___ua_cat_dict', 'ua_category', assumeNotNull(UA)) AS user_agent_category,
multiIf(positionCaseInsensitive(assumeNotNull(reqPath), 'robots.txt') > 0, 'robots.txt',
    positionCaseInsensitive(assumeNotNull(reqPath), 'llms.txt') > 0, 'llms.txt',
    'other')
AS resource_category,
dictGet('__SHARED_PROJECT___ua_cat_dict', 'ai_category', assumeNotNull(UA)) AS ai_category,
dictGet('__SHARED_PROJECT___ua_cat_dict', 'is_bot', assumeNotNull(UA)) AS is_bot_traffic,
 (ds_req_time*1000)::UInt64 as transferTimeMSec,
 (totalBytes/2) / (totalBytes/transferTimeMSec) as timeToFirstByte,
dictGetString('__SHARED_PROJECT___country_mapping', 'country_iso_code', initcap(assumeNotNull(client_country_name))) AS country,
 splitByChar('=', assumeNotNull(rangeStr))[2] as range,
 * 
FROM {STREAM}