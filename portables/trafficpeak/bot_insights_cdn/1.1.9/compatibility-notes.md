# Bot Insights CDN 1.1.9 Demo Compatibility Notes

This bundle contains one-off SIEM transform compatibility patches for the
`demo.trafficpeak.live / akamai` deployment path. The canonical transform JSON
files and `hydrolix/resources.hdp.yaml` keep the intended Bot Insights 1.1.9
behavior by default; the compatibility behavior is opt-in and must be requested
explicitly when generating an impact report or preparing a deployment:

- `hydrolix/compatibility/siem_demo_compat.yaml`
- `hydrolix/compatibility/siem_gz_demo_compat.yaml`

For the impact report, pass both patches explicitly:

```bash
uv run python scripts/bundle_impact_report.py \
  --bundle trafficpeak_bot_insights_cdn \
  --version 1.1.9 \
  --project akamai \
  --cluster-url https://demo.trafficpeak.live \
  --upgrade-from 1.0.0 \
  --compatibility-patch siem/siem=compatibility/siem_demo_compat.yaml \
  --compatibility-patch siem/siem-gz=compatibility/siem_gz_demo_compat.yaml
```

For deployment, use the compatibility-aware wrapper and pass the same patches:

```bash
uv run python scripts/deploy_bundle_with_compatibility.py \
  --bundle trafficpeak_bot_insights_cdn \
  --version 1.1.9 \
  --project akamai \
  --cluster-url https://demo.trafficpeak.live \
  --upgrade-from 1.0.0 \
  --compatibility-patch siem/siem=compatibility/siem_demo_compat.yaml \
  --compatibility-patch siem/siem-gz=compatibility/siem_gz_demo_compat.yaml
```

The live Akamai SIEM transform already existed before the Bot Insights 1.1.9
bundle deployment. To keep the upgrade additive for this deployment, the bundle
preserves the live expressions for existing SIEM aliases that would otherwise
change historical field semantics:

- `bot_category`
- `botnet_id`
- `is_bot_traffic`
- `referer`
- `requestHeaders`
- `requestHeadersStr`
- `resource_category`
- `responseHeaders`
- `responseHeadersStr`
- `ruleMessage`

The compatibility patch keeps the new additive Bot Insights fields and summary
tables, but defers these behavior-changing SIEM improvements:

- richer `resource_category` classification
- bot-score based `is_bot_traffic`
- decoded header-map parsing
- parsed-map `referer` lookup
- filtered bot category indexing
- empty-string `ruleMessage` fallback

This is not intended as a general product contract. It is a deployment
compatibility compromise for an already-live demo project so the bundle can be
applied without changing existing SIEM field meanings.
