SELECT 
  toStartOfMinute (timestamp) AS minute,
  count(*) as request_count,
  avg(durationMs) as avg_duration_ms,
  countIf(statusCode >= 200 AND statusCode < 300) as success_count,
  countIf(statusCode >= 400) as error_count,
  uniq(clientIP) as unique_clients,
  topK(5)(country) as top_countries
FROM __PROJECT_NAME__.__TABLE_NAME__
GROUP BY minute
SETTINGS hdx_primary_key = 'minute'