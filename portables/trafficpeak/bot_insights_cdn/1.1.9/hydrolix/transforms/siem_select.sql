SELECT datediff('s', timestamp, now64(3)) AS hdx_source_latency_sec,
multiIf(
  positionCaseInsensitive(path, 'robots.txt') > 0, 'robots.txt',
  positionCaseInsensitive(path, 'sitemap.xml') > 0, 'sitemap.xml',
  positionCaseInsensitive(path, 'ads.txt') > 0, 'ads.txt',
  positionCaseInsensitive(path, 'llms.txt') > 0, 'llms.txt',
  match(path, '^/api/') OR match(path, '/api/'), 'api',
  match(path, '\.(css|js|png|jpg|jpeg|gif|svg|ico|woff2?|ttf|eot)$'), 'static',
  match(path, '\.(html?|php|asp|jsp)$') OR path = '/' OR NOT match(path, '\.'), 'page',
  'other'
) AS resource_category,
'' AS response_content_type,
akamai_extract_key_pair(decodeURLComponent(assumeNotNull(requestHeadersStr)), '\r\n', ':') AS requestHeaders,
akamai_extract_key_pair(decodeURLComponent(assumeNotNull(responseHeadersStr)), '\r\n', ':') AS responseHeaders,
lower(cutQueryString(assumeNotNull(path))) AS request_path_norm,
if(
  empty(arrayFilter(segment -> notEmpty(segment), splitByChar('/', request_path_norm))),
  '/',
  concat('/', arrayElement(arrayFilter(segment -> notEmpty(segment), splitByChar('/', request_path_norm)), 1))
) AS request_path_root,
arrayStringConcat(arrayMap(x -> base64Decode(x), splitByChar(';', decodeURLComponent(assumeNotNull(ruleActionsStr)))), ';') AS action_taken,
assumeNotNull(ruleDataStr) AS attack_data,
akamai_siem_extract(rulesStr) AS rules,
akamai_siem_extract(ruleMessagesStr) AS ruleMessages,
akamai_siem_extract(ruleTagsStr) AS ruleTags,
arrayResize(akamai_siem_extract(ruleDataStr), length(ruleTags), '') AS ruleData,
akamai_siem_extract(ruleVersionsStr) AS ruleVersions,
arrayResize(akamai_siem_extract(ruleSelectorsStr), length(ruleTags), '') AS ruleSelectors,
akamai_siem_extract(ruleActionsStr) AS ruleActions,
arrayExists(x -> multiSearchAny(x, ['AKAMAI/BOT/', 'BOT/', 'CUSTOM/BOT/', 'MBS_CL', 'AKAMAI/BOT/CUST_DEFINED_BOTS']), ruleTags) AS attack_bot,
arrayExists(x -> multiSearchAny(x, ['ASE/', 'AKAMAI/POLICY/', 'OWASP_CRS/', 'AKAMAI/WAF', 'AKAMAI/WEB_ATTACK/']), ruleTags) AS attack_waf,
arrayExists(x -> x = 'REPUTATION', ruleTags) AS attack_reputation,
(arrayExists(x -> startsWith(x, 'AKAMAI/CUSTOM'), ruleTags) OR arrayExists(x -> startsWith(x, '6'), rules)) AS attack_custom,
arrayExists(x -> multiSearchAny(x, ['USER-RISK']), rules) AS attack_risk,
arrayExists(x -> x = 'IPBLOCK', ruleTags) AS attack_firewall,
arrayExists(x -> positionCaseInsensitive(x, 'BLOCK/') > 0 OR position(x, 'IPBLOCK') != 0, ruleTags) AS attack_dos,
arrayFilter((x, y) -> y = 1, ['Bot', 'WAF', 'Reputation', 'Custom', 'DoS', 'User Risk', 'Network Firewall'], [attack_bot, attack_waf, attack_reputation, attack_custom, attack_dos, attack_risk, attack_firewall]) AS attack_types,
multiIf(
  arrayExists(x -> startsWith(x, 'Web Scrapers'), ruleMessages), 'Web Scrapers',
  arrayExists(x -> startsWith(x, 'Scanning Tools'), ruleMessages), 'Scanning Tools',
  arrayExists(x -> startsWith(x, 'Web Attackers'), ruleMessages), 'Web Attackers',
  arrayExists(x -> startsWith(x, 'DOS Attacker'), ruleMessages), 'DOS Attacker',
  ''
) AS ruleMessage,
multiIf(
  arrayExists(x -> multiSearchAny(x, ['AKAMAI/BOT/AKAMAI_CATEGORIZED']), ruleTags), 'Akamai',
  arrayExists(x -> multiSearchAny(x, ['AKAMAI/BOT/CUST_DEFINED_BOTS']), ruleTags), 'Customer',
  arrayExists(x -> multiSearchAny(x, ['AKAMAI/BOT/UNKNOWN_BOT']), ruleTags), 'Unknown',
  ''
) AS bot_category_source,
multiIf(
  arrayExists(x -> multiSearchAny(x, ['AKAMAI/BOT/AKAMAI_CATEGORIZED']), ruleTags), 'Akamai',
  arrayExists(x -> multiSearchAny(x, ['AKAMAI/BOT/CUST_DEFINED_BOTS']), ruleTags), 'Customer',
  arrayExists(x -> multiSearchAny(x, ['AKAMAI/BOT/UNKNOWN_BOT']), ruleTags), 'Unknown',
  ''
) AS bot_type,
multiIf(
  bot_type = 'Akamai', arrayFilter(x -> multiSearchAny(x, ['3991']), rules),
  bot_type = 'Customer', arrayFilter(x -> multiSearchAny(x, ['BOT-6']), rules),
  bot_type = 'Unknown', arrayFilter(x -> multiSearchAny(x, ['3900', '3903', '3910', '3912', '3990']), rules),
  emptyArrayString()
) AS bot_category_rule_ids,
multiIf(
  bot_type = 'Akamai', arrayFilter(x -> multiSearchAny(x, ['3991']), rules),
  bot_type = 'Customer', arrayFilter(x -> multiSearchAny(x, ['BOT-6']), rules),
  bot_type = 'Unknown', arrayFilter(x -> multiSearchAny(x, ['3900', '3903', '3910', '3912', '3990']), rules),
  ['']
) AS BotCategoryAkamai,
arrayMap(x -> (indexOf(rules, x)), BotCategoryAkamai) AS arrayMapIndex,
arrayMap(x -> indexOf(rules, x), bot_category_rule_ids) AS bot_category_indexes,
arrayStringConcat(arrayFilter(x -> length(x) > 0, arrayMap(x -> if(x > 0, ruleMessages[x], ''), bot_category_indexes)), ';') AS bot_category,
arrayStringConcat(arrayMap(x -> ruleData[x], bot_category_indexes)) AS botnet_id,
lowerUTF8(replaceAll(replaceAll(assumeNotNull(bot_category), ' ', '_'), '-', '_')) AS akamai_bot_category_norm,
multiIf(
  akamai_bot_category_norm = 'web_scraper', 'scraper',
  akamai_bot_category_norm = 'search_engine', 'seo_crawler',
  akamai_bot_category_norm = 'monitoring', 'monitoring_bot',
  akamai_bot_category_norm = 'social_media', 'social_crawler',
  akamai_bot_category_norm = 'ai' OR startsWith(akamai_bot_category_norm, 'ai_'), 'ai_crawler',
  akamai_bot_category_norm IN ('unknown', 'unknown_bot'), 'unknown_bot',
  empty(akamai_bot_category_norm) AND bot_type = 'Unknown', 'unknown_bot',
  'unknown_bot'
) AS akamai_canonical_bot_class,
dictGet('akamai_ua_cat_dict', 'ua_category', assumeNotNull(UA)) AS user_agent_category,
multiIf(
  assumeNotNull(botData_responseSegment) = 0, 'Human',
  assumeNotNull(botData_responseSegment) > 0, 'Bot',
  'empty'
) AS response_segment,
multiIf(
  assumeNotNull(botData_responseSegment) = 0, 'Human',
  assumeNotNull(botData_responseSegment) = 1, 'Cautious response',
  assumeNotNull(botData_responseSegment) = 2, 'Strict response',
  assumeNotNull(botData_responseSegment) = 3, 'Aggressive response',
  assumeNotNull(botData_responseSegment) = 4, 'Safeguard',
  'empty'
) AS response_segment_details,
multiIf(
  assumeNotNull(clientData_telemetryType) = 0, 'Web client (standard telemetry)',
  assumeNotNull(clientData_telemetryType) = 1, 'Web client (inline telemetry)',
  assumeNotNull(clientData_telemetryType) = 2, 'Native app (SDK)',
  'empty'
) AS telemetry_type,
multiIf(
  botScore = 0, '0',
  botScore BETWEEN 1 AND 10, '1-10',
  botScore BETWEEN 11 AND 20, '11-20',
  botScore BETWEEN 21 AND 30, '21-30',
  botScore BETWEEN 31 AND 40, '31-40',
  botScore BETWEEN 41 AND 50, '41-50',
  botScore BETWEEN 51 AND 60, '51-60',
  botScore BETWEEN 61 AND 70, '61-70',
  botScore BETWEEN 71 AND 80, '71-80',
  botScore BETWEEN 81 AND 90, '81-90',
  botScore BETWEEN 91 AND 100, '91-100',
  ''
) AS botScoreRange,
if(bot_score > 0, 1, 0) AS is_bot_traffic,
dictGet('akamai_ua_cat_dict', 'ai_category', assumeNotNull(UA)) AS ai_category,
decodeURLComponent(assumeNotNull(requestHeadersStr)) AS requestHeadersStr,
decodeURLComponent(assumeNotNull(responseHeadersStr)) AS responseHeadersStr,
requestHeaders['Referer'] AS referer,
requestHeaders['Sec-CH-UA'] AS CH_UA,
requestHeaders['User-Agent'] AS UA,
notEmpty(CH_UA) AS has_CH_UA,
arrayFilter((x, y) -> y = 1, ['Bot', 'WAF', 'Reputation', 'Custom', 'DoS'], [attack_bot, attack_waf, attack_reputation, attack_custom, attack_dos]) AS attackTypes,
arrayDistinct(arrayFilter(x -> multiSearchAny(x, ['AKAMAI/BOT/', 'BOT/', 'CUSTOM/BOT/', 'MBS_CL', ':']), ruleTags)) AS ruleTagsBot,
arrayDistinct(arrayFilter(x -> multiSearchAny(x, ['ASE/', 'AKAMAI/POLICY/', 'OWASP_CRS/', 'AKAMAI/WAF', 'AKAMAI/WEB_ATTACK/', ':']), ruleTags)) AS ruleTagsWAF,
arrayDistinct(arrayFilter(x -> multiSearchAny(x, ['IPBLOCK', ':']), ruleTags)) AS ruleTagsDOS,
*
FROM {STREAM}