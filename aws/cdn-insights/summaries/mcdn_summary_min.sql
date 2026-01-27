SELECT
    toStartOfMinute (timestamp) as timestamp,
    cache_was_cached,
    response_status_code,
    request_host,
    client_country_iso_code,
    client_city,
    client_asn,
    edge_pop,
    user_agent_category,
    hdx_cdn,
    count() as cnt_all,
    sum(response_total_bytes) as response_total_bytes,
    avg(response_time_to_first_byte_ms) as response_ttfb_ms,
    avg(response_time_to_last_byte_ms) as response_ttlb_ms,
    quantiles (0.25, 0.5, 0.75, 0.9, 0.95, 0.99) (response_time_to_first_byte_ms) AS quantiles_response_ttfb_ms,
    quantiles (0.25, 0.5, 0.75, 0.9, 0.95, 0.99) (response_time_to_last_byte_ms) AS quantiles_response_ttlb_ms,
    quantiles (0.25, 0.5, 0.75, 0.9, 0.95, 0.99) (origin_time_to_first_byte_ms) AS quantiles_origin_ttfb_ms,
    quantiles (0.25, 0.5, 0.75, 0.9, 0.95, 0.99) (origin_time_to_last_byte_ms) AS quantiles_origin_ttlb_ms
FROM
    __PROJECT_NAME__.__TABLE_NAME__
GROUP BY
    timestamp,
    cache_was_cached,
    response_status_code,
    request_host,
    client_country_iso_code,
    client_city,
    client_asn,
    edge_pop,
    user_agent_category,
    hdx_cdn
    SETTINGS hdx_primary_key = 'timestamp'
