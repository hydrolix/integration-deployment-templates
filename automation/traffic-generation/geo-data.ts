// geo-data.ts
// Standalone module for geo data lookup functionality
// Usage: import { getRandomGeoData, getGeoDataByPostalCode } from "https://raw.githubusercontent.com/yourusername/yourrepo/main/geo-data.ts";

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
    colo: "JFK"
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
    colo: "LAX"
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
    colo: "SFO"
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
    colo: "ORD"
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
    colo: "MIA"
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
    colo: "SEA"
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
    colo: "YYZ"
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
    colo: "LHR"
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
    colo: "CDG"
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
    colo: "TXL"
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
    colo: "MAD"
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
    colo: "FCO"
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
    colo: "AMS"
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
    colo: "NRT"
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
    colo: "ICN"
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
    colo: "PEK"
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
    colo: "DEL"
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
    colo: "SIN"
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
    colo: "SGN"
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
    colo: "BKK"
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
    colo: "CGK"
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
    colo: "MNL"
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
    colo: "KUL"
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
    colo: "HKG"
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
    colo: "TPE"
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
    colo: "SYD"
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
    colo: "MEL"
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
    colo: "AKL"
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
    colo: "GRU"
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
    colo: "EZE"
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
    colo: "BOG"
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
    colo: "CAI"
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
    colo: "CPT"
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
    colo: "JNB"
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
    colo: "NBO"
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
    colo: "SVO"
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
    colo: "WAW"
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
    colo: "IST"
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
    colo: "LIS"
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
    colo: "BCN"
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
    colo: "OSL"
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
    colo: "ARN"
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
    colo: "BRU"
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
    colo: "ZUR"
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
    colo: "VIE"
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
    colo: "CPH"
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
    colo: "HEL"
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
    colo: "KEF"
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
    colo: "DXB"
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
    colo: "RUH"
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
    colo: "TLV"
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

