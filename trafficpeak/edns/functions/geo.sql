(type, ip) -> multiIf(
            type = 'asn', toString(akamai_asn_name(assumeNotNull(ip))),
            type = 'org', akamai_asn_org(assumeNotNull(ip)),
            type = 'country', akamai_country_name(assumeNotNull(ip)),
            type = 'subdivision1', akamai_subdivision_1_name(assumeNotNull(ip)),
            type = 'state', akamai_subdivision_2_name(assumeNotNull(ip)),
            type = 'city', akamai_city_name(assumeNotNull(ip)),
            type = 'time_zone', akamai_time_zone(assumeNotNull(ip)),
            type = 'metro_code', toString(akamai_metro_code(assumeNotNull(ip))),
            type = 'continent', akamai_continent_name(assumeNotNull(ip)),
            type = 'continent_iso_code', akamai_country_iso_code(assumeNotNull(ip)),
            type = 'country_iso_code', akamai_country_iso_code(assumeNotNull(ip)),
            type = 'locale_code', akamai_locale_code(assumeNotNull(ip)),
            type = 'accuracy_radius', toString(akamai_accuracy_radius(assumeNotNull(ip))),
            type = 'longitude', toString(akamai_longitude(assumeNotNull(ip))),
            type = 'latitude', toString(akamai_latitude(assumeNotNull(ip))),
            type = 'european_union', toString(
                akamai_is_in_european_union(assumeNotNull(ip))
                ),
            NULL
)
