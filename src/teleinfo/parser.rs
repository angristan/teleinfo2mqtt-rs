use std::collections::HashMap;
use std::error::Error;
use std::fmt;

// A teleinfo frame is a set of data sets
// Each data set is a key-value pair + a checksum
#[derive(Debug)]
pub struct TeleinfoFrame {
    pub adco: String,     // Adresse du compteur
    pub optarif: String,  // Option tarifaire
    pub isousc: String,   // Intensité souscrite, en A
    pub base: String,     // Index option base, en Wh
    pub ptec: String,     // Période tarifaire en cours
    pub iinst: String,    // Intensité instantanée, en A
    pub imax: String,     // Intensité maximale appelée, en A
    pub papp: String,     // Puissance apparente, en VA (arrondie à la dizaine la plus proche)
    pub hhphc: String,    // Horaire Heures Pleines Heures Creuses
    pub motdetat: String, // Mot d'état du compteur
}

/*

LABEL DATA CHECKSUM

ADCO 012345678901 B
OPTARIF BASE 0
ISOUSC 30 9
BASE 002809718 .
PTEC TH.. $
IINST 002 Y
IMAX 090 H
PAPP 00390 -
HHPHC A ,
MOTDETAT 000000 B
*/

impl PartialEq for TeleinfoFrame {
    fn eq(&self, other: &Self) -> bool {
        self.adco == other.adco
            && self.optarif == other.optarif
            && self.isousc == other.isousc
            && self.base == other.base
            && self.ptec == other.ptec
            && self.iinst == other.iinst
            && self.imax == other.imax
            && self.papp == other.papp
            && self.hhphc == other.hhphc
            && self.motdetat == other.motdetat
    }
}

// Hijack the Display trait to provide a JSON representation of the TeleinfoFrame
// that is compatible with Home Assistant's MQTT integration
impl fmt::Display for TeleinfoFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r#"{{
"ADCO": {{"raw": "{}", "value": {}}},
"OPTARIF": {{"raw": "{}", "value": "{}"}},
"ISOUSC": {{"raw": "{}", "value": {}}},
"BASE": {{"raw": "{}", "value": {}}},
"PTEC": {{"raw": "{}", "value": "{}"}},
"IINST": {{"raw": "{}", "value": {}}},
"IMAX": {{"raw": "{}", "value": {}}},
"PAPP": {{"raw": "{}", "value": {}}},
"HHPHC": {{"raw": "{}", "value": "{}"}}
}}"#,
            self.adco,
            self.adco.parse::<i64>().unwrap(),
            self.optarif,
            self.optarif,
            self.isousc,
            self.isousc.parse::<i32>().unwrap(),
            self.base,
            self.base.parse::<i64>().unwrap(),
            self.ptec,
            &self.ptec[0..2],
            self.iinst,
            self.iinst.parse::<i32>().unwrap(),
            self.imax,
            self.imax.parse::<i32>().unwrap(),
            self.papp,
            self.papp.parse::<i32>().unwrap(),
            self.hhphc,
            self.hhphc
        )
    }
}

pub fn parse_teleinfo(teleinfo: &str) -> Result<TeleinfoFrame, Box<dyn Error>> {
    let mut teleinfo_map = HashMap::new();
    for line in teleinfo.lines() {
        let mut split = line.split_whitespace();

        let key = split.next().ok_or("Missing key")?;
        if [0x02, 0x03].contains(&key.as_bytes()[0]) {
            // Skip start and end of frame characters
            continue;
        }
        let value = split.next().ok_or("Missing value")?;
        teleinfo_map.insert(key, value);
    }
    Ok(TeleinfoFrame {
        adco: teleinfo_map.get("ADCO").ok_or("Missing ADCO")?.to_string(),
        optarif: teleinfo_map
            .get("OPTARIF")
            .ok_or("Missing OPTARIF")?
            .to_string(),
        isousc: teleinfo_map
            .get("ISOUSC")
            .ok_or("Missing ISOUSC")?
            .to_string(),
        base: teleinfo_map.get("BASE").ok_or("Missing BASE")?.to_string(),
        ptec: teleinfo_map.get("PTEC").ok_or("Missing PTEC")?.to_string(),
        iinst: teleinfo_map
            .get("IINST")
            .ok_or("Missing IINST")?
            .to_string(),
        imax: teleinfo_map.get("IMAX").ok_or("Missing IMAX")?.to_string(),
        papp: teleinfo_map.get("PAPP").ok_or("Missing PAPP")?.to_string(),
        hhphc: teleinfo_map
            .get("HHPHC")
            .ok_or("Missing HHPHC")?
            .to_string(),
        motdetat: teleinfo_map
            .get("MOTDETAT")
            .ok_or("Missing MOTDETAT")?
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_teleinfo() {
        let teleinfo = "ADCO 012345678901 B\nOPTARIF BASE 0\nISOUSC 30 9\nBASE 002809718 .\nPTEC TH.. $\nIINST 002 Y\nIMAX 090 H\nPAPP 00390 -\nHHPHC A ,\nMOTDETAT 000000 B";
        let parse_teleinfo = parse_teleinfo(teleinfo);
        assert!(parse_teleinfo.is_ok());
        let parse_teleinfo = parse_teleinfo.unwrap();
        assert_eq!(parse_teleinfo.adco, "012345678901");
        assert_eq!(parse_teleinfo.optarif, "BASE");
        assert_eq!(parse_teleinfo.isousc, "30");
        assert_eq!(parse_teleinfo.base, "002809718");
        assert_eq!(parse_teleinfo.ptec, "TH..");
        assert_eq!(parse_teleinfo.iinst, "002");
        assert_eq!(parse_teleinfo.imax, "090");
        assert_eq!(parse_teleinfo.papp, "00390");
        assert_eq!(parse_teleinfo.hhphc, "A");
        assert_eq!(parse_teleinfo.motdetat, "000000");
    }
}
