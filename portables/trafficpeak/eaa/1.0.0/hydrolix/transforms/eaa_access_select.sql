SELECT
datediff('s', datetime, now64(3)) as hdx_source_latency,
IF(internalHost != '-', internalHost, originHost) as origin,
* FROM {STREAM}
