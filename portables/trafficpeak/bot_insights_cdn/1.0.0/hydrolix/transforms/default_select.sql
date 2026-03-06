SELECT 
datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
dictGet('akamai_ua_cat_dict', 'ua_category', assumeNotNull(user_agent)) AS user_agent_category,
multiIf(positionCaseInsensitive(assumeNotNull(request_path), 'robots.txt') > 0, 'robots.txt',
    positionCaseInsensitive(assumeNotNull(request_path), 'llms.txt') > 0, 'llms.txt',
    'other')
AS resource_category,
dictGet('akamai_ua_cat_dict', 'ai_category', assumeNotNull(user_agent)) AS ai_category,
dictGet('akamai_ua_cat_dict', 'is_bot', assumeNotNull(user_agent)) AS is_bot_traffic,
 * 
FROM {STREAM}