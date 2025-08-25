// geo-data.ts
// Standalone module for geo data lookup functionality
// Usage: import { getRandomGeoData, getGeoDataByPostalCode } from "https://raw.githubusercontent.com/yourusername/yourrepo/main/geo-data.ts";
//

export interface GeoData {
  postalCode: string;
  city: string;
  region: string;
  regionCode: string;
  country: string;
  continent: string;
  latitude: string;
  longitude: string;
  timezone: string;
  colo: string;
  clientIP: string;
  metroCode: string;
}

const GEO_DATA_LOOKUP: Record<string, GeoData> = {
  "10001": {
    postalCode: "10001",
    city: "New York",
    region: "New York",
    regionCode: "NY",
    country: "US",
    continent: "NA",
    latitude: "40.75066",
    longitude: "-73.99670",
    timezone: "America/New_York",
    colo: "JFK",
    clientIP: "74.125.224.72",
    metroCode: "501"
  },
  "90210": {
    postalCode: "90210",
    city: "Beverly Hills",
    region: "California",
    regionCode: "CA",
    country: "US",
    continent: "NA",
    latitude: "34.10369",
    longitude: "-118.41022",
    timezone: "America/Los_Angeles",
    colo: "LAX",
    clientIP: "172.217.164.46",
    metroCode: "803"
  },
  "94102": {
    postalCode: "94102",
    city: "San Francisco",
    region: "California",
    regionCode: "CA",
    country: "US",
    continent: "NA",
    latitude: "37.78008",
    longitude: "-122.42016",
    timezone: "America/Los_Angeles",
    colo: "SFO",
    clientIP: "216.58.194.174",
    metroCode: "807"
  },
  "60601": {
    postalCode: "60601",
    city: "Chicago",
    region: "Illinois",
    regionCode: "IL",
    country: "US",
    continent: "NA",
    latitude: "41.88425",
    longitude: "-87.63245",
    timezone: "America/Chicago",
    colo: "ORD",
    clientIP: "72.21.91.29",
    metroCode: "602"
  },
  "33101": {
    postalCode: "33101",
    city: "Miami",
    region: "Florida",
    regionCode: "FL",
    country: "US",
    continent: "NA",
    latitude: "25.77427",
    longitude: "-80.19366",
    timezone: "America/New_York",
    colo: "MIA",
    clientIP: "54.239.28.85",
    metroCode: "528"
  },
  "98101": {
    postalCode: "98101",
    city: "Seattle",
    region: "Washington",
    regionCode: "WA",
    country: "US",
    continent: "NA",
    latitude: "47.60357",
    longitude: "-122.32945",
    timezone: "America/Los_Angeles",
    colo: "SEA",
    clientIP: "13.107.42.14",
    metroCode: "819"
  },
  "M5V 3A8": {
    postalCode: "M5V 3A8",
    city: "Toronto",
    region: "Ontario",
    regionCode: "ON",
    country: "CA",
    continent: "NA",
    latitude: "43.64258",
    longitude: "-79.38781",
    timezone: "America/Toronto",
    colo: "YYZ",
    clientIP: "142.250.191.78",
    metroCode: "508"
  },
  "SW1A 1AA": {
    postalCode: "SW1A 1AA",
    city: "London",
    region: "England",
    regionCode: "ENG",
    country: "GB",
    continent: "EU",
    latitude: "51.50007",
    longitude: "-0.12497",
    timezone: "Europe/London",
    colo: "LHR",
    clientIP: "185.199.108.153",
    metroCode: "100"
  },
  "75001": {
    postalCode: "75001",
    city: "Paris",
    region: "Île-de-France",
    regionCode: "11",
    country: "FR",
    continent: "EU",
    latitude: "48.86007",
    longitude: "2.34118",
    timezone: "Europe/Paris",
    colo: "CDG",
    clientIP: "213.186.33.5",
    metroCode: "101"
  },
  "10115": {
    postalCode: "10115",
    city: "Berlin",
    region: "Berlin",
    regionCode: "BE",
    country: "DE",
    continent: "EU",
    latitude: "52.53048",
    longitude: "13.38262",
    timezone: "Europe/Berlin",
    colo: "TXL",
    clientIP: "217.160.0.112",
    metroCode: "102"
  },
  "28001": {
    postalCode: "28001",
    city: "Madrid",
    region: "Madrid",
    regionCode: "MD",
    country: "ES",
    continent: "EU",
    latitude: "40.41650",
    longitude: "-3.70256",
    timezone: "Europe/Madrid",
    colo: "MAD",
    clientIP: "185.60.216.35",
    metroCode: "103"
  },
  "00118": {
    postalCode: "00118",
    city: "Rome",
    region: "Lazio",
    regionCode: "62",
    country: "IT",
    continent: "EU",
    latitude: "41.89277",
    longitude: "12.48366",
    timezone: "Europe/Rome",
    colo: "FCO",
    clientIP: "151.101.1.140",
    metroCode: "104"
  },
  "1012": {
    postalCode: "1012",
    city: "Amsterdam",
    region: "North Holland",
    regionCode: "NH",
    country: "NL",
    continent: "EU",
    latitude: "52.37403",
    longitude: "4.88969",
    timezone: "Europe/Amsterdam",
    colo: "AMS",
    clientIP: "195.69.146.17",
    metroCode: "105"
  },
  "100-0001": {
    postalCode: "100-0001",
    city: "Tokyo",
    region: "Tokyo",
    regionCode: "13",
    country: "JP",
    continent: "AS",
    latitude: "35.67614",
    longitude: "139.65031",
    timezone: "Asia/Tokyo",
    colo: "NRT",
    clientIP: "210.155.141.200",
    metroCode: "200"
  },
  "110-001": {
    postalCode: "110-001",
    city: "Seoul",
    region: "Seoul",
    regionCode: "11",
    country: "KR",
    continent: "AS",
    latitude: "37.56826",
    longitude: "126.97783",
    timezone: "Asia/Seoul",
    colo: "ICN",
    clientIP: "203.248.252.2",
    metroCode: "201"
  },
  "100001": {
    postalCode: "100001",
    city: "Beijing",
    region: "Beijing",
    regionCode: "11",
    country: "CN",
    continent: "AS",
    latitude: "39.90469",
    longitude: "116.40717",
    timezone: "Asia/Shanghai",
    colo: "PEK",
    clientIP: "39.156.69.79",
    metroCode: "202"
  },
  "110001": {
    postalCode: "110001",
    city: "New Delhi",
    region: "Delhi",
    regionCode: "DL",
    country: "IN",
    continent: "AS",
    latitude: "28.61394",
    longitude: "77.20902",
    timezone: "Asia/Kolkata",
    colo: "DEL",
    clientIP: "13.127.70.75",
    metroCode: "203"
  },
  "018956": {
    postalCode: "018956",
    city: "Singapore",
    region: "Singapore",
    regionCode: "SG",
    country: "SG",
    continent: "AS",
    latitude: "1.35208",
    longitude: "103.81983",
    timezone: "Asia/Singapore",
    colo: "SIN",
    clientIP: "18.141.177.75",
    metroCode: "204"
  },
  "71606": {
    postalCode: "71606",
    city: "Ho Chi Minh City",
    region: "Hải Dương Province",
    regionCode: "61",
    country: "VN",
    continent: "AS",
    latitude: "10.82302",
    longitude: "106.62965",
    timezone: "Asia/Ho_Chi_Minh",
    colo: "SGN",
    clientIP: "103.67.184.73",
    metroCode: "205"
  },
  "10110": {
    postalCode: "10110",
    city: "Bangkok",
    region: "Bangkok",
    regionCode: "10",
    country: "TH",
    continent: "AS",
    latitude: "13.75398",
    longitude: "100.50144",
    timezone: "Asia/Bangkok",
    colo: "BKK",
    clientIP: "180.183.255.96",
    metroCode: "206"
  },
  "12930": {
    postalCode: "12930",
    city: "Jakarta",
    region: "Jakarta",
    regionCode: "JK",
    country: "ID",
    continent: "AS",
    latitude: "-6.17511",
    longitude: "106.86504",
    timezone: "Asia/Jakarta",
    colo: "CGK",
    clientIP: "103.28.148.52",
    metroCode: "207"
  },
  "1000": {
    postalCode: "1000",
    city: "Manila",
    region: "Metro Manila",
    regionCode: "NCR",
    country: "PH",
    continent: "AS",
    latitude: "14.59995",
    longitude: "120.98422",
    timezone: "Asia/Manila",
    colo: "MNL",
    clientIP: "122.2.180.51",
    metroCode: "208"
  },
  "50050": {
    postalCode: "50050",
    city: "Kuala Lumpur",
    region: "Kuala Lumpur",
    regionCode: "14",
    country: "MY",
    continent: "AS",
    latitude: "3.13982",
    longitude: "101.68685",
    timezone: "Asia/Kuala_Lumpur",
    colo: "KUL",
    clientIP: "175.139.142.25",
    metroCode: "209"
  },
  "999077": {
    postalCode: "999077",
    city: "Hong Kong",
    region: "Hong Kong",
    regionCode: "HK",
    country: "HK",
    continent: "AS",
    latitude: "22.39643",
    longitude: "114.10949",
    timezone: "Asia/Hong_Kong",
    colo: "HKG",
    clientIP: "103.2.28.146",
    metroCode: "210"
  },
  "100": {
    postalCode: "100",
    city: "Taipei",
    region: "Taipei",
    regionCode: "TPE",
    country: "TW",
    continent: "AS",
    latitude: "25.03463",
    longitude: "121.56486",
    timezone: "Asia/Taipei",
    colo: "TPE",
    clientIP: "61.216.4.9",
    metroCode: "211"
  },
  "2000": {
    postalCode: "2000",
    city: "Sydney",
    region: "New South Wales",
    regionCode: "NSW",
    country: "AU",
    continent: "OC",
    latitude: "-33.86785",
    longitude: "151.20732",
    timezone: "Australia/Sydney",
    colo: "SYD",
    clientIP: "13.210.67.224",
    metroCode: "300"
  },
  "3000": {
    postalCode: "3000",
    city: "Melbourne",
    region: "Victoria",
    regionCode: "VIC",
    country: "AU",
    continent: "OC",
    latitude: "-37.81411",
    longitude: "144.96328",
    timezone: "Australia/Melbourne",
    colo: "MEL",
    clientIP: "3.105.171.164",
    metroCode: "301"
  },
  "1010": {
    postalCode: "1010",
    city: "Auckland",
    region: "Auckland",
    regionCode: "AUK",
    country: "NZ",
    continent: "OC",
    latitude: "-36.84846",
    longitude: "174.76333",
    timezone: "Pacific/Auckland",
    colo: "AKL",
    clientIP: "203.97.116.52",
    metroCode: "302"
  },
  "01310-100": {
    postalCode: "01310-100",
    city: "São Paulo",
    region: "São Paulo",
    regionCode: "SP",
    country: "BR",
    continent: "SA",
    latitude: "-23.56168",
    longitude: "-46.65557",
    timezone: "America/Sao_Paulo",
    colo: "GRU",
    clientIP: "18.231.105.75",
    metroCode: "400"
  },
  "C1001": {
    postalCode: "C1001",
    city: "Buenos Aires",
    region: "Buenos Aires",
    regionCode: "BA",
    country: "AR",
    continent: "SA",
    latitude: "-34.61315",
    longitude: "-58.37723",
    timezone: "America/Argentina/Buenos_Aires",
    colo: "EZE",
    clientIP: "181.209.82.68",
    metroCode: "401"
  },
  "110111": {
    postalCode: "110111",
    city: "Bogotá",
    region: "Bogotá",
    regionCode: "DC",
    country: "CO",
    continent: "SA",
    latitude: "4.71099",
    longitude: "-74.07209",
    timezone: "America/Bogota",
    colo: "BOG",
    clientIP: "186.154.164.2",
    metroCode: "402"
  },
  "11000": {
    postalCode: "11000",
    city: "Cairo",
    region: "Cairo",
    regionCode: "C",
    country: "EG",
    continent: "AF",
    latitude: "30.04420",
    longitude: "31.23571",
    timezone: "Africa/Cairo",
    colo: "CAI",
    clientIP: "156.160.0.1",
    metroCode: "500"
  },
  "8001": {
    postalCode: "8001",
    city: "Cape Town",
    region: "Western Cape",
    regionCode: "WC",
    country: "ZA",
    continent: "AF",
    latitude: "-33.91828",
    longitude: "18.42322",
    timezone: "Africa/Johannesburg",
    colo: "CPT",
    clientIP: "13.246.70.49",
    metroCode: "501"
  },
  "2001": {
    postalCode: "2001",
    city: "Johannesburg",
    region: "Gauteng",
    regionCode: "GP",
    country: "ZA",
    continent: "AF",
    latitude: "-26.20227",
    longitude: "28.04363",
    timezone: "Africa/Johannesburg",
    colo: "JNB",
    clientIP: "105.186.12.53",
    metroCode: "502"
  },
  "101001": {
    postalCode: "101001",
    city: "Nairobi",
    region: "Nairobi",
    regionCode: "30",
    country: "KE",
    continent: "AF",
    latitude: "-1.29207",
    longitude: "36.82194",
    timezone: "Africa/Nairobi",
    colo: "NBO",
    clientIP: "197.248.21.10",
    metroCode: "503"
  },
  "119991": {
    postalCode: "119991",
    city: "Moscow",
    region: "Moscow",
    regionCode: "77",
    country: "RU",
    continent: "EU",
    latitude: "55.75583",
    longitude: "37.61729",
    timezone: "Europe/Moscow",
    colo: "SVO",
    clientIP: "87.250.250.242",
    metroCode: "106"
  },
  "02-679": {
    postalCode: "02-679",
    city: "Warsaw",
    region: "Mazowieckie",
    regionCode: "14",
    country: "PL",
    continent: "EU",
    latitude: "52.22977",
    longitude: "21.01178",
    timezone: "Europe/Warsaw",
    colo: "WAW",
    clientIP: "213.59.243.9",
    metroCode: "107"
  },
  "111250": {
    postalCode: "111250",
    city: "Istanbul",
    region: "Istanbul",
    regionCode: "34",
    country: "TR",
    continent: "EU",
    latitude: "41.00527",
    longitude: "28.97696",
    timezone: "Europe/Istanbul",
    colo: "IST",
    clientIP: "185.125.190.36",
    metroCode: "108"
  },
  "1000-001": {
    postalCode: "1000-001",
    city: "Lisbon",
    region: "Lisboa",
    regionCode: "11",
    country: "PT",
    continent: "EU",
    latitude: "38.71667",
    longitude: "-9.13333",
    timezone: "Europe/Lisbon",
    colo: "LIS",
    clientIP: "185.17.126.10",
    metroCode: "109"
  },
  "08001": {
    postalCode: "08001",
    city: "Barcelona",
    region: "Catalonia",
    regionCode: "CT",
    country: "ES",
    continent: "EU",
    latitude: "41.38879",
    longitude: "2.15899",
    timezone: "Europe/Madrid",
    colo: "BCN",
    clientIP: "94.142.241.111",
    metroCode: "110"
  },
  "1011": {
    postalCode: "1011",
    city: "Oslo",
    region: "Oslo",
    regionCode: "03",
    country: "NO",
    continent: "EU",
    latitude: "59.91387",
    longitude: "10.75225",
    timezone: "Europe/Oslo",
    colo: "OSL",
    clientIP: "129.241.1.99",
    metroCode: "111"
  },
  "111 21": {
    postalCode: "111 21",
    city: "Stockholm",
    region: "Stockholm",
    regionCode: "AB",
    country: "SE",
    continent: "EU",
    latitude: "59.32938",
    longitude: "18.06871",
    timezone: "Europe/Stockholm",
    colo: "ARN",
    clientIP: "213.115.18.10",
    metroCode: "112"
  },
  "1050": {
    postalCode: "1050",
    city: "Brussels",
    region: "Brussels",
    regionCode: "BRU",
    country: "BE",
    continent: "EU",
    latitude: "50.85045",
    longitude: "4.34878",
    timezone: "Europe/Brussels",
    colo: "BRU",
    clientIP: "195.238.2.21",
    metroCode: "113"
  },
  "8000": {
    postalCode: "8000",
    city: "Zurich",
    region: "Zurich",
    regionCode: "ZH",
    country: "CH",
    continent: "EU",
    latitude: "47.36667",
    longitude: "8.55000",
    timezone: "Europe/Zurich",
    colo: "ZUR",
    clientIP: "130.59.138.1",
    metroCode: "114"
  },
  "1010": {
    postalCode: "1010",
    city: "Vienna",
    region: "Vienna",
    regionCode: "9",
    country: "AT",
    continent: "EU",
    latitude: "48.20849",
    longitude: "16.37208",
    timezone: "Europe/Vienna",
    colo: "VIE",
    clientIP: "128.130.6.7",
    metroCode: "115"
  },
  "1000": {
    postalCode: "1000",
    city: "Copenhagen",
    region: "Capital Region",
    regionCode: "84",
    country: "DK",
    continent: "EU",
    latitude: "55.67594",
    longitude: "12.56553",
    timezone: "Europe/Copenhagen",
    colo: "CPH",
    clientIP: "130.225.98.98",
    metroCode: "116"
  },
  "00100": {
    postalCode: "00100",
    city: "Helsinki",
    region: "Uusimaa",
    regionCode: "01",
    country: "FI",
    continent: "EU",
    latitude: "60.16952",
    longitude: "24.93545",
    timezone: "Europe/Helsinki",
    colo: "HEL",
    clientIP: "128.214.248.6",
    metroCode: "117"
  },
  "101": {
    postalCode: "101",
    city: "Reykjavik",
    region: "Capital Region",
    regionCode: "1",
    country: "IS",
    continent: "EU",
    latitude: "64.13548",
    longitude: "-21.89541",
    timezone: "Atlantic/Reykjavik",
    colo: "KEF",
    clientIP: "130.208.165.207",
    metroCode: "118"
  },
  "11222": {
    postalCode: "11222",
    city: "Dubai",
    region: "Dubai",
    regionCode: "DU",
    country: "AE",
    continent: "AS",
    latitude: "25.20485",
    longitude: "55.27078",
    timezone: "Asia/Dubai",
    colo: "DXB",
    clientIP: "185.46.212.97",
    metroCode: "212"
  },
  "11693": {
    postalCode: "11693",
    city: "Riyadh",
    region: "Riyadh",
    regionCode: "01",
    country: "SA",
    continent: "AS",
    latitude: "24.71139",
    longitude: "46.67529",
    timezone: "Asia/Riyadh",
    colo: "RUH",
    clientIP: "188.161.165.2",
    metroCode: "213"
  },
  "1431": {
    postalCode: "1431",
    city: "Tel Aviv",
    region: "Tel Aviv",
    regionCode: "TA",
    country: "IL",
    continent: "AS",
    latitude: "32.06404",
    longitude: "34.76647",
    timezone: "Asia/Jerusalem",
    colo: "TLV",
    clientIP: "212.150.161.84",
    metroCode: "214"
  }
};

/**
 * Get geo data for a specific postal code
 */
export function getGeoDataByPostalCode(postalCode: string): GeoData | null {
  return GEO_DATA_LOOKUP[postalCode] || null;
}

/**
 * Get random geo data from the available options
 */
export function getRandomGeoData(): GeoData {
  const postalCodes = Object.keys(GEO_DATA_LOOKUP);
  const randomPostalCode = postalCodes[Math.floor(Math.random() * postalCodes.length)];
  return GEO_DATA_LOOKUP[randomPostalCode];
}

/**
 * Get all available postal codes
 */
export function getAllPostalCodes(): string[] {
  return Object.keys(GEO_DATA_LOOKUP);
}

/**
 * Get all available geo data entries
 */
export function getAllGeoData(): GeoData[] {
  return Object.values(GEO_DATA_LOOKUP);
}

/**
 * Get geo data filtered by continent
 */
export function getGeoDataByContinent(continent: string): GeoData[] {
  return Object.values(GEO_DATA_LOOKUP).filter(geo => geo.continent === continent);
}

/**
 * Get geo data filtered by country
 */
export function getGeoDataByCountry(country: string): GeoData[] {
  return Object.values(GEO_DATA_LOOKUP).filter(geo => geo.country === country);
}

/**
 * Get random geo data from a specific continent
 */
export function getRandomGeoDataByContinent(continent: string): GeoData | null {
  const continentData = getGeoDataByContinent(continent);
  if (continentData.length === 0) return null;
  return continentData[Math.floor(Math.random() * continentData.length)];
}

/**
 * Get random geo data from a specific country
 */
export function getRandomGeoDataByCountry(country: string): GeoData | null {
  const countryData = getGeoDataByCountry(country);
  if (countryData.length === 0) return null;
  return countryData[Math.floor(Math.random() * countryData.length)];
}

