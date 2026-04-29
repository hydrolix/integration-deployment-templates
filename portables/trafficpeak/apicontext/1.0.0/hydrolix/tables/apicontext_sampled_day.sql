SELECT
  toStartOfDay (startTime) AS startTime,
  httpCode,
  contextCategory,
  locationID,
  IF(
    cityHash64 (metadataDomain, startTime, resultID) % 100 < 10,
    metadataDomain,
    '~~~SAMPLED_OUT~~~'
  ) AS metadataDomain,
  cloud,
  targetCloud,
  IF(
    cityHash64 (ipAddr, startTime, resultID) % 100 < 10,
    ipAddr,
    '~~~SAMPLED_OUT~~~'
  ) AS ipAddr,
  IF(
    cityHash64 (targetIPAddr, startTime, resultID) % 100 < 10,
    targetIPAddr,
    '~~~SAMPLED_OUT~~~'
  ) AS targetIPAddr,
  city,
  continent,
  IF(
    cityHash64 (callURL, startTime, resultID) % 100 < 10,
    callURL,
    '~~~SAMPLED_OUT~~~'
  ) AS callURL,
  IF(
    cityHash64 (callID, startTime, resultID) % 100 < 10,
    callID,
    '~~~SAMPLED_OUT~~~'
  ) AS callID,
  IF(
    cityHash64 (resultID, startTime) % 100 < 1,
    resultID,
    '~~~SAMPLED_OUT~~~'
  ) AS resultID,
  IF(
    cityHash64 (apiCreationTime, startTime, resultID) % 100 < 10,
    apiCreationTime,
    null
  ) AS apiCreationTime,
  COUNT(*) AS cnt_all,
  AVG(responseTime) AS avg_responseTime,
  AVG(dns) AS avg_dns_lookup_time,
  AVG(connect) AS avg_tcp_connect_time,
  AVG(tlsHandshake) AS avg_tls_handshake_time,
  AVG(upload) AS avg_upload_time,
  AVG(processing) AS avg_processing_time,
  AVG(download) AS avg_download_time,
  SUM(dns) AS sum_dns,
  SUM(connect) AS sum_connect,
  SUM(tlsHandshake) AS sum_tlsHandshake,
  sumIf (1, contextCategory = 'FAIL') AS failed_requests,
  (failed_requests / cnt_all) * 100 AS error_rate
FROM
  akamai.apicontext
GROUP BY
  startTime,
  httpCode,
  locationID,
  contextCategory,
  metadataDomain,
  cloud,
  targetCloud,
  city,
  continent,
  ipAddr,
  targetIPAddr,
  callURL,
  callID,
  resultID,
  apiCreationTime SETTINGS hdx_primary_key = 'startTime'
