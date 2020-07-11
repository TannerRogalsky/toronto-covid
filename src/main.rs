use geojson::GeoJson;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
struct Entry {
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

fn get_name(data: &serde_json::Map<String, serde_json::Value>) -> Result<String, ()> {
    let name = match data.get("AREA_NAME").ok_or(())? {
        Value::String(str) => str,
        _ => return Err(()),
    };
    // munge the name to make it match with the covid data
    let name = name.split(" (").next().ok_or(())?.to_owned();
    let name = if name == "Briar Hill-Belgravia" {
        "Briar Hill - Belgravia".to_owned()
    } else {
        name
    };
    let name = if name == "Cabbagetown-South St.James Town" {
        "Cabbagetown-South St. James Town".to_owned()
    } else {
        name
    };
    let name = if name == "North St.James Town" {
        "North St. James Town".to_owned()
    } else {
        name
    };
    let name = if name == "Danforth East York" {
        "Danforth-East York".to_owned()
    } else {
        name
    };
    let name = if name == "Mimico" {
        "Mimico (includes Humber Bay Shores)".to_owned()
    } else {
        name
    };
    Ok(name)
}

fn main() -> quicli::prelude::CliResult {
    let neighbourhoods = {
        // origin: https://open.toronto.ca/dataset/neighbourhoods/
        let path = std::path::Path::new("Neighbourhoods.geojson");
        let data = std::fs::read_to_string(path)?;
        data.parse::<geojson::GeoJson>()?
    };

    let covid_data: Vec<Entry> = {
        // origin: https://open.toronto.ca/dataset/covid-19-cases-in-toronto/
        let path = std::path::Path::new("COVID19 cases.json");
        let file = std::io::BufReader::new(std::fs::File::open(path)?);
        serde_json::from_reader(file)?
    };

    let mut per_neighbourhood_count = std::collections::HashMap::new();
    for e in covid_data.iter() {
        if let Some(neighbourhood) = &e.neighbourhood {
            *per_neighbourhood_count.entry(neighbourhood).or_insert(0u32) += 1;
        }
    }

    let neighbourhoods = match neighbourhoods {
        GeoJson::FeatureCollection(mut neighbourhoods) => {
            for feature in neighbourhoods.features.iter_mut() {
                if let Some(properties) = &mut feature.properties {
                    let name = get_name(properties).unwrap();
                    let v = serde_json::Value::Number(
                        (*per_neighbourhood_count.get(&name).unwrap()).into(),
                    );
                    properties.insert("covid_case_count".to_owned(), v);
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
