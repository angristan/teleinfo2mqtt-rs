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

/// Validates the checksum of a TeleInfo data set line.
/// Format: <Label> <Value> <Checksum> (space-separated for historical mode)
/// Checksum = (S1 & 0x3F) + 0x20, where S1 is the sum of ASCII values
/// from label (included) to the separator before checksum (excluded).
fn validate_checksum(line: &str) -> bool {
    // The line format is: LABEL<sep>VALUE<sep>CHECKSUM
    // where <sep> is either tab (0x09) or space (0x20)
    // The checksum is calculated over "LABEL<sep>VALUE" (excluding final separator)

    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    // Find the last separator (space or tab) before checksum
    let last_sep_pos = bytes.iter().rposition(|&b| b == b' ' || b == b'\t');
    let Some(last_sep_pos) = last_sep_pos else {
        return false;
    };

    // Checksum is the last character
    if last_sep_pos + 1 >= bytes.len() {
        return false;
    }
    let expected_checksum = bytes[last_sep_pos + 1];

    // Calculate checksum over everything up to (but not including) the last separator
    let sum: u32 = bytes[..last_sep_pos].iter().map(|&b| b as u32).sum();
    let calculated_checksum = ((sum & 0x3F) + 0x20) as u8;

    expected_checksum == calculated_checksum
}

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
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Skip lines starting with STX or ETX
        let first_byte = trimmed.as_bytes()[0];
        if first_byte == 0x02 || first_byte == 0x03 {
            continue;
        }

        // Validate checksum before processing
        if !validate_checksum(trimmed) {
            return Err(format!("Invalid checksum for line: {}", trimmed).into());
        }

        let mut split = trimmed.split_whitespace();
        let key = split.next().ok_or("Missing key")?;
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
    fn test_validate_checksum_valid() {
        // Checksum = (sum(LABEL + SEP + VALUE) & 0x3F) + 0x20
        assert!(validate_checksum("ADCO 012345678901 E"));
        assert!(validate_checksum("OPTARIF BASE 0"));
        assert!(validate_checksum("ISOUSC 30 9"));
        assert!(validate_checksum("BASE 002809718 ."));
        assert!(validate_checksum("PTEC TH.. $"));
        assert!(validate_checksum("IINST 002 Y"));
        assert!(validate_checksum("IMAX 090 H"));
        assert!(validate_checksum("PAPP 00390 -"));
        assert!(validate_checksum("HHPHC A ,"));
        assert!(validate_checksum("MOTDETAT 000000 B"));
    }

    #[test]
    fn test_validate_checksum_invalid() {
        // Wrong checksum character
        assert!(!validate_checksum("ADCO 012345678901 X"));
        assert!(!validate_checksum("ISOUSC 30 Z"));
        // Corrupted value
        assert!(!validate_checksum("ADCO 999999999999 E"));
    }

    #[test]
    fn test_validate_checksum_edge_cases() {
        assert!(!validate_checksum(""));
        assert!(!validate_checksum("NOSPACE"));
    }

    #[test]
    fn test_parse_teleinfo_valid() {
        let teleinfo = "ADCO 012345678901 E\nOPTARIF BASE 0\nISOUSC 30 9\nBASE 002809718 .\nPTEC TH.. $\nIINST 002 Y\nIMAX 090 H\nPAPP 00390 -\nHHPHC A ,\nMOTDETAT 000000 B";
        let result = parse_teleinfo(teleinfo);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());
        let frame = result.unwrap();
        assert_eq!(frame.adco, "012345678901");
        assert_eq!(frame.optarif, "BASE");
        assert_eq!(frame.isousc, "30");
        assert_eq!(frame.base, "002809718");
        assert_eq!(frame.ptec, "TH..");
        assert_eq!(frame.iinst, "002");
        assert_eq!(frame.imax, "090");
        assert_eq!(frame.papp, "00390");
        assert_eq!(frame.hhphc, "A");
        assert_eq!(frame.motdetat, "000000");
    }

    #[test]
    fn test_parse_teleinfo_invalid_checksum() {
        // Same as valid but with wrong checksum on ADCO line
        let teleinfo = "ADCO 012345678901 X\nOPTARIF BASE 0\nISOUSC 30 9\nBASE 002809718 .\nPTEC TH.. $\nIINST 002 Y\nIMAX 090 H\nPAPP 00390 -\nHHPHC A ,\nMOTDETAT 000000 B";
        let result = parse_teleinfo(teleinfo);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid checksum"));
    }
}
