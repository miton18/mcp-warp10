use std::collections::BTreeMap;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Long(i64),
    Double(f64),
    Bool(bool),
    Str(String),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct Datum {
    pub ts: i64,
    pub location: Option<i64>,
    pub elevation: Option<i64>,
    pub class: String,
    pub labels: BTreeMap<String, String>,
    pub value: Value,
}

#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("empty line")]
    Empty,
    #[error("invalid timestamp")]
    BadTimestamp,
    #[error("invalid location")]
    BadLocation,
    #[error("invalid elevation")]
    BadElevation,
    #[error("missing class")]
    MissingClass,
    #[error("invalid labels")]
    BadLabels,
    #[error("missing value")]
    MissingValue,
    #[error("invalid value: {0}")]
    BadValue(String),
}

pub fn to_geoxppoint(lat: f64, lon: f64) -> i64 {
    let lat = lat.clamp(-90.0, 90.0);
    let lon = ((lon + 180.0) % 360.0 + 360.0) % 360.0 - 180.0;
    let lat_e7 = (lat * 1e7) as i64;
    let lon_e7 = (lon * 1e7) as i64;
    (lat_e7 << 32) | (lon_e7 & 0xffff_ffff)
}

fn parse_value(raw: &str) -> Result<Value, ParseError> {
    if raw == "true" {
        return Ok(Value::Bool(true));
    }
    if raw == "false" {
        return Ok(Value::Bool(false));
    }
    if raw.len() >= 2 && ((raw.starts_with('\'') && raw.ends_with('\'')) || (raw.starts_with('"') && raw.ends_with('"'))) {
        return Ok(Value::Str(raw[1..raw.len() - 1].to_string()));
    }
    if let Some(hexstr) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        return i64::from_str_radix(hexstr, 16)
            .map(Value::Long)
            .map_err(|_| ParseError::BadValue(raw.to_string()));
    }
    if let Ok(l) = raw.parse::<i64>() {
        return Ok(Value::Long(l));
    }
    if let Ok(d) = raw.parse::<f64>() {
        return Ok(Value::Double(d));
    }
    Err(ParseError::BadValue(raw.to_string()))
}

fn parse_ts_token(tok: &str) -> Result<(i64, Option<i64>, Option<i64>), ParseError> {
    let parts: Vec<&str> = tok.split('/').filter(|p| !p.is_empty()).collect();
    let ts = parts[0].parse::<i64>().map_err(|_| ParseError::BadTimestamp)?;
    let location = match parts.get(1) {
        None => None,
        Some(l) => {
            let (lat, lon) = l.split_once(':').ok_or(ParseError::BadLocation)?;
            let lat = lat.parse::<f64>().map_err(|_| ParseError::BadLocation)?;
            let lon = lon.parse::<f64>().map_err(|_| ParseError::BadLocation)?;
            Some(to_geoxppoint(lat, lon))
        }
    };
    let elevation = match parts.get(2) {
        None => None,
        Some(e) => Some(e.parse::<i64>().map_err(|_| ParseError::BadElevation)?),
    };
    Ok((ts, location, elevation))
}

fn parse_class_labels(tok: &str) -> Result<(String, BTreeMap<String, String>), ParseError> {
    let open = tok.find('{').ok_or(ParseError::MissingClass)?;
    let class = tok[..open].to_string();
    if class.is_empty() {
        return Err(ParseError::MissingClass);
    }
    if !tok.ends_with('}') {
        return Err(ParseError::BadLabels);
    }
    let inner = &tok[open + 1..tok.len() - 1];
    let mut labels = BTreeMap::new();
    if !inner.is_empty() {
        for pair in inner.split(',') {
            let (k, v) = pair.split_once('=').ok_or(ParseError::BadLabels)?;
            labels.insert(k.to_string(), v.to_string());
        }
    }
    Ok((class, labels))
}

pub fn parse_line(line: &str) -> Result<Datum, ParseError> {
    let line = line.trim();
    if line.is_empty() {
        return Err(ParseError::Empty);
    }
    let mut it = line.splitn(3, char::is_whitespace);
    let ts_tok = it.next().ok_or(ParseError::Empty)?;
    let class_tok = it.next().ok_or(ParseError::MissingClass)?;
    let value_raw = it.next().ok_or(ParseError::MissingValue)?.trim();
    let (ts, location, elevation) = parse_ts_token(ts_tok)?;
    let (class, labels) = parse_class_labels(class_tok)?;
    let value = parse_value(value_raw)?;
    Ok(Datum {
        ts,
        location,
        elevation,
        class,
        labels,
        value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_long() {
        let d = parse_line("1380475081000000// foo{a=b,c=d} 42").unwrap();
        assert_eq!(d.ts, 1380475081000000);
        assert_eq!(d.location, None);
        assert_eq!(d.class, "foo");
        assert_eq!(d.labels.get("a").unwrap(), "b");
        assert_eq!(d.value, Value::Long(42));
    }

    #[test]
    fn parses_geo_and_elev_and_double() {
        let d = parse_line("1380475081123456/48.0:-4.5/10000000 bar{} 3.14").unwrap();
        assert_eq!(d.location, Some(to_geoxppoint(48.0, -4.5)));
        assert_eq!(d.elevation, Some(10000000));
        assert_eq!(d.value, Value::Double(3.14));
    }

    #[test]
    fn parses_bool_and_string() {
        assert_eq!(parse_line("1// c{} true").unwrap().value, Value::Bool(true));
        assert_eq!(
            parse_line("1// c{} 'hello world'").unwrap().value,
            Value::Str("hello world".to_string())
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_line("").is_err());
        assert!(parse_line("abc// c{} 1").is_err());
        assert!(parse_line("1// c{}").is_err());
    }
}
