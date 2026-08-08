SELECT
datediff('s', timestamp, now64(3)) as hdx_source_latency,
*
FROM {STREAM}
