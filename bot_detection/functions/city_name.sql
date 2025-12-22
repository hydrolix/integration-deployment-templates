(ip) -> dictGetString (
  'reference_geoip_city_locations_en',
  'city_name',
  reference_geoname_id (assumeNotNull (ip))
)