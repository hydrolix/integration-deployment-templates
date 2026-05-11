SELECT
  toStartOfHour (reqTimeSec) AS reqTimeSec,
  reqHost,
  asn,
  userAgentCategory,
  isBotTraffic,
  aiCategory,
  resourceCategory,
  reqMethod,
  cacheStatus,
  statusCode,
  requestPathPattern,
  country,
  aiSource,
  trafficCohort,
  count() AS cnt_all,
  sum(totalBytes) AS sum_totalBytes,
  sumIf (
    Origin_TurnAroundTime,
    isNotNull (Origin_TurnAroundTime)
    AND Origin_TurnAroundTime >= 0
  ) AS sum_originTurnAroundTime_ms,
  countIf (
    isNotNull (Origin_TurnAroundTime)
    AND Origin_TurnAroundTime >= 0
  ) AS cnt_originTurnAroundTime,
  sumIf (
    timeToFirstByte,
    isNotNull (timeToFirstByte)
    AND timeToFirstByte >= 0
  ) AS sum_timeToFirstByte_ms,
  countIf (
    isNotNull (timeToFirstByte)
    AND timeToFirstByte >= 0
  ) AS cnt_timeToFirstByte,
  countIf (coalesce(queryStr, '') != '') AS cnt_queryStringPresent,
  uniqIf (
    cityHash64 (coalesce(queryStr, '')),
    coalesce(queryStr, '') != ''
    ) AS cnt_distinctQueryStrings
FROM demo.logs
GROUP BY
  reqTimeSec,
  reqHost,
  asn,
  userAgentCategory,
  isBotTraffic,
  aiCategory,
  resourceCategory,
  reqMethod,
  cacheStatus,
  statusCode,
  requestPathPattern,
  country,
  aiSource,
  trafficCohort SETTINGS hdx_primary_key = 'reqTimeSec'
