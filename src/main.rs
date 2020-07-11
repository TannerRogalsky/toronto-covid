use geojson::GeoJson;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
struct CovidEntry {
    /// Unique row identifier for Open Data database
    #[serde(rename = "_id")]
    id: u32,
    /// Outbreak associated cases are associated with outbreaks of COVID-19 in Toronto healthcare
    /// institutions and healthcare settings (e.g. long-term care homes, retirement homes,
    /// hospitals, etc.) and other Toronto congregate settings (such as homeless shelters).
    #[serde(rename = "Outbreak Associated")]
    outbreak_associated: String, // todo: Enum

    /// Age at time of illness. Age groups (in years): ≤19, 20-29, 30-39, 40-49, 50-59, 60-69,
    /// 70-79, 80-89, 90+, unknown.
    #[serde(rename = "Age Group")]
    age_group: Option<String>, // todo: u32 range

    /// Toronto is divided into 140 geographically distinct neighborhoods that were established to
    /// help government and community agencies with local planning by providing socio-economic data
    /// for a meaningful geographic area.
    #[serde(rename = "Neighbourhood Name")]
    neighbourhood: Option<String>,

    /// Forward sortation area (i.e. first three characters of postal code) based on the case's
    /// primary home address.
    #[serde(rename = "FSA")]
    fsa: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "Characteristic")]
enum CensusEntryCategory {
    #[serde(rename = "Neighbourhood Number")]
    NeighbourhoodInformation(CensusEntry),
    #[serde(rename = "Population, 2016")]
    Population2016(CensusEntry),
    #[serde(other)]
    Other,
}

#[derive(Serialize, Deserialize)]
struct CensusEntry {
    #[serde(rename = "_id")]
    id: u32,
    #[serde(rename = "Category")]
    category: String,
    #[serde(rename = "Topic")]
    topic: String,
    #[serde(rename = "Data Source")]
    data_source: String,
    #[serde(flatten)]
    neighbourhoods: HashMap<String, Option<String>>,
}

fn get_name(data: &serde_json::Map<String, serde_json::Value>) -> Result<String, ()> {
    let name = match data.get("AREA_NAME").ok_or(())? {
        Value::String(str) => str,
        _ => return Err(()),
    };
    // munge the name to make it match with the covid data
    let name = name.split(" (").next().ok_or(())?;
    let name = neighbourhood_names_normalizer(name);
    Ok(name.to_owned())
}

fn main() -> quicli::prelude::CliResult {
    let neighbourhoods = {
        // origin: https://open.toronto.ca/dataset/neighbourhoods/
        let path = std::path::Path::new("Neighbourhoods.geojson");
        let data = std::fs::read_to_string(path)?;
        data.parse::<geojson::GeoJson>()?
    };

    let covid_data: Vec<CovidEntry> = {
        // origin: https://open.toronto.ca/dataset/covid-19-cases-in-toronto/
        let path = std::path::Path::new("COVID19 cases.json");
        let file = std::io::BufReader::new(std::fs::File::open(path)?);
        serde_json::from_reader(file)?
    };

    let census: Vec<CensusEntryCategory> = {
        // origin: https://open.toronto.ca/dataset/neighbourhood-profiles/
        let path = std::path::Path::new("neighbourhood-profiles-2016-csv.json");
        let file = std::io::BufReader::new(std::fs::File::open(path)?);
        serde_json::from_reader(file)?
    };

    let populations = census
        .into_iter()
        .filter_map(|c| match c {
            CensusEntryCategory::Population2016(e) => Some(e),
            _ => None,
        })
        .next()
        .unwrap();
    let populations = populations
        .neighbourhoods
        .into_iter()
        .filter_map(|(n, pop)| {
            if let Some(pop) = pop {
                let n = neighbourhood_names_normalizer(&n).to_owned();
                pop.replace(",", "")
                    .parse::<u32>()
                    .ok()
                    .and_then(|pop| Some((n, pop)))
            } else {
                None
            }
        })
        .collect::<HashMap<_, _>>();
    // println!("{:#?}", populations.keys());

    let mut per_neighbourhood_count = std::collections::HashMap::new();
    for e in covid_data.iter() {
        if let Some(neighbourhood) = &e.neighbourhood {
            *per_neighbourhood_count
                .entry(neighbourhood_names_normalizer(neighbourhood).to_owned())
                .or_insert(0u32) += 1;
        }
    }

    let neighbourhoods = match neighbourhoods {
        GeoJson::FeatureCollection(mut neighbourhoods) => {
            for feature in neighbourhoods.features.iter_mut() {
                if let Some(properties) = &mut feature.properties {
                    let name = get_name(properties).unwrap();

                    let covid_case_count = *per_neighbourhood_count.get(&name).unwrap();
                    let v = serde_json::Value::Number(covid_case_count.into());
                    properties.insert("covid_case_count".to_owned(), v);

                    let population = *populations.get(&name).unwrap();
                    let v = serde_json::Value::Number(population.into());
                    properties.insert("population".to_owned(), v);
                }
            }
            neighbourhoods
        }
        _ => unimplemented!(),
    };

    let out_path = std::path::Path::new("docs").join("out.geojson");
    std::fs::write(out_path, neighbourhoods.to_string())?;

    Ok(())
}

fn neighbourhood_names_normalizer(name: &str) -> &str {
    match name {
        "Weston-Pellam Park" => "Weston-Pelham Park",
        "Briar Hill - Belgravia" => "Briar Hill-Belgravia",
        "Cabbagetown-South St.James Town" => "Cabbagetown-South St. James Town",
        "North St.James Town" => "North St. James Town",
        "Mimico (includes Humber Bay Shores)" => "Mimico",
        "Danforth East York" => "Danforth-East York",
        _ => name,
    }
}

const NEIGHBOURHOOD_NAMES: [&str; 141] = [
    "Lambton Baby Point",
    "Yonge-Eglinton",
    "Ionview",
    "Flemingdon Park",
    "Banbury-Don Mills",
    "Mount Dennis",
    "Alderwood",
    "Clanton Park",
    "Bay Street Corridor",
    "Don Valley Village",
    "Bridle Path-Sunnybrook-York Mills",
    "Downsview-Roding-CFB",
    "Clairlea-Birchmount",
    "North Riverdale",
    "Mount Pleasant West",
    "Westminster-Branson",
    "Eringate-Centennial-West Deane",
    "Oakridge",
    "Tam O'Shanter-Sullivan",
    "South Riverdale",
    "Birchcliffe-Cliffside",
    "Palmerston-Little Italy",
    "Kingsview Village-The Westway",
    "Morningside",
    "Oakwood Village",
    "Runnymede-Bloor West Village",
    "Princess-Rosethorn",
    "Kensington-Chinatown",
    "O'Connor-Parkview",
    "Agincourt North",
    "Lawrence Park North",
    "Dorset Park",
    "Wychwood",
    "Yonge-St.Clair",
    "Kingsway South",
    "Parkwoods-Donalda",
    "Rexdale-Kipling",
    "Church-Yonge Corridor",
    "Brookhaven-Amesbury",
    "Bayview Village",
    "Humber Heights-Westmount",
    "Bayview Woods-Steeles",
    "Niagara",
    "Long Branch",
    "Leaside-Bennington",
    "St.Andrew-Windfields",
    "Corso Italia-Davenport",
    "Wexford/Maryvale",
    "Cliffcrest",
    "Steeles",
    "Broadview North",
    "Etobicoke West Mall",
    "L'Amoreaux",
    "South Parkdale",
    "Willowdale East",
    "Bedford Park-Nortown",
    "North St. James Town",
    "Woodbine Corridor",
    "Playter Estates-Danforth",
    "Lawrence Park South",
    "Casa Loma",
    "Scarborough Village",
    "Edenbridge-Humber Valley",
    "Beechborough-Greenbrook",
    "Pleasant View",
    "Danforth",
    "Old East York",
    "Islington-City Centre West",
    "Humewood-Cedarvale",
    "York University Heights",
    "Taylor-Massey",
    "Mount Olive-Silverstone-Jamestown",
    "Roncesvalles",
    "Trinity-Bellwoods",
    "Mount Pleasant East",
    "Humbermede",
    "Keelesdale-Eglinton West",
    "Highland Creek",
    "Thorncliffe Park",
    "Rosedale-Moore Park",
    "Junction Area",
    "Lansing-Westgate",
    "Regent Park",
    "Thistletown-Beaumond Heights",
    "Markland Wood",
    "Guildwood",
    "Henry Farm",
    "Maple Leaf",
    "Danforth East York",
    "Woburn",
    "High Park-Swansea",
    "Milliken",
    "Victoria Village",
    "Yorkdale-Glen Park",
    "Glenfield-Jane Heights",
    "City of Toronto",
    "High Park North",
    "Waterfront Communities-The Island",
    "Centennial Scarborough",
    "The Beaches",
    "Agincourt South-Malvern West",
    "West Hill",
    "Englemount-Lawrence",
    "Rockcliffe-Smythe",
    "Dovercourt-Wallace Emerson-Junction",
    "Stonegate-Queensway",
    "Bathurst Manor",
    "Newtonbrook West",
    "Rustic",
    "Forest Hill South",
    "Mimico (includes Humber Bay Shores)",
    "Woodbine-Lumsden",
    "Caledonia-Fairbank",
    "Greenwood-Coxwell",
    "Annex",
    "Eglinton East",
    "Malvern",
    "Hillcrest Village",
    "Willowdale West",
    "Little Portugal",
    "Black Creek",
    "Kennedy Park",
    "New Toronto",
    "University",
    "East End-Danforth",
    "Bendale",
    "Elms-Old Rexdale",
    "Blake-Jones",
    "West Humber-Clairville",
    "Dufferin Grove",
    "Briar Hill-Belgravia",
    "Willowridge-Martingrove-Richview",
    "Pelmo Park-Humberlea",
    "Cabbagetown-South St. James Town",
    "Weston-Pelham Park",
    "Moss Park",
    "Forest Hill North",
    "Weston",
    "Rouge",
    "Newtonbrook East",
    "Humber Summit",
];
