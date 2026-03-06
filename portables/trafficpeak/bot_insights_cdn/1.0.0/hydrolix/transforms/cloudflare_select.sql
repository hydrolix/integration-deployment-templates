SELECT 
    CASE
        WHEN match(CAST(EdgeStartTimestamp AS String), '^[0-9]+$') THEN 
            toDateTime64(
                CAST(EdgeStartTimestamp AS Int64) / 1000000000.0, 9
            )
            ELSE parseDateTimeBestEffort(EdgeStartTimestamp)
    END AS timestamp,
    datediff(s, timestamp, now64(3)) AS hdx_source_latency_sec,
    TRUE as cache_was_cached,
    decodeURLComponent(queryString(request_full_path)) as request_query_string,
    multiIf(positionCaseInsensitive(assumeNotNull(request_path), 'robots.txt') > 0, 'robots.txt',
    positionCaseInsensitive(assumeNotNull(request_path), 'llms.txt') > 0, 'llms.txt',
    'other') AS resource_category,
    'NONE' AS user_agent_category,
    FALSE AS is_bot_traffic,
    *
FROM { STREAM }