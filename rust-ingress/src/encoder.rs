use crate::gts::Value;

const FLAGS_CONTINUATION: u8 = 0x80;
const FLAGS_TIMESTAMP_EQUALS_BASE: u8 = 0x20;
const FLAGS_TIMESTAMP_ZIGZAG_DELTA_BASE: u8 = 0x40;
const FLAGS_TIMESTAMP_RAW_ABSOLUTE: u8 = 0x60;
const FLAGS_MASK_TIMESTAMP: u8 = 0x60;
const FLAGS_TYPE_BOOLEAN: u8 = 0x00;
const FLAGS_TYPE_LONG: u8 = 0x08;
const FLAGS_TYPE_DOUBLE: u8 = 0x10;
const FLAGS_TYPE_STRING: u8 = 0x18;
const FLAGS_MASK_TYPE: u8 = 0x18;
const FLAGS_STRING_BINARY: u8 = 0x02;
const FLAGS_BOOLEAN_VALUE_TRUE: u8 = 0x04;
const FLAGS_BOOLEAN_VALUE_FALSE: u8 = 0x02;
const FLAGS_DELETE_MARKER: u8 = 0x07;
const FLAGS_LONG_ZIGZAG: u8 = 0x04;
const FLAGS_LONG_DELTA_PREVIOUS: u8 = 0x02;
const FLAGS_DOUBLE_IEEE754: u8 = 0x04;
const FLAGS_VALUE_IDENTICAL: u8 = 0x01;
const FLAGS_LOCATION: u8 = 0x40;
const FLAGS_LOCATION_GEOXPPOINT_ZIGZAG_DELTA: u8 = 0x20;
const FLAGS_LOCATION_IDENTICAL: u8 = 0x10;
const FLAGS_ELEVATION: u8 = 0x08;
const FLAGS_ELEVATION_ZIGZAG: u8 = 0x04;
const FLAGS_ELEVATION_DELTA_PREVIOUS: u8 = 0x02;
const FLAGS_ELEVATION_IDENTICAL: u8 = 0x01;

pub fn encode_uvarint(mut v: u64, out: &mut Vec<u8>) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            break;
        }
        out.push(b | 0x80);
    }
}

pub fn encode_zigzag(v: i64, out: &mut Vec<u8>) {
    let z = ((v << 1) ^ (v >> 63)) as u64;
    encode_uvarint(z, out);
}

pub struct Encoder {
    base: i64,
    out: Vec<u8>,
    count: u64,
    last_ts: i64,
    valid_last_ts: bool,
    last_loc: i64,
    valid_last_loc: bool,
    last_elev: i64,
    valid_last_elev: bool,
    last_long: i64,
    valid_last_long: bool,
    last_double: f64,
    valid_last_double: bool,
    last_str: Option<String>,
}

impl Encoder {
    pub fn new(base_timestamp: i64) -> Self {
        Self {
            base: base_timestamp,
            out: Vec::new(),
            count: 0,
            last_ts: 0,
            valid_last_ts: false,
            last_loc: 0,
            valid_last_loc: false,
            last_elev: 0,
            valid_last_elev: false,
            last_long: 0,
            valid_last_long: false,
            last_double: 0.0,
            valid_last_double: false,
            last_str: None,
        }
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn add(&mut self, ts: i64, location: Option<i64>, elevation: Option<i64>, value: &Value) {
        let mut flag: u8 = 0;
        if ts == self.base {
            flag |= FLAGS_TIMESTAMP_EQUALS_BASE;
        } else if self.valid_last_ts {
            let delta_base = (ts - self.base).abs();
            let delta_last = (ts - self.last_ts).abs();
            if delta_base < delta_last {
                if delta_base < (1i64 << 48) {
                    flag |= FLAGS_TIMESTAMP_ZIGZAG_DELTA_BASE;
                } else {
                    flag |= FLAGS_TIMESTAMP_RAW_ABSOLUTE;
                }
            } else if delta_last < (1i64 << 48) {
                flag |= 0x00;
            } else {
                flag |= FLAGS_TIMESTAMP_RAW_ABSOLUTE;
            }
        } else if (ts - self.base).abs() < (1i64 << 48) {
            flag |= FLAGS_TIMESTAMP_ZIGZAG_DELTA_BASE;
        } else {
            flag |= FLAGS_TIMESTAMP_RAW_ABSOLUTE;
        }

        match value {
            Value::Long(lv) => {
                flag |= FLAGS_TYPE_LONG;
                if self.valid_last_long && *lv == self.last_long {
                    flag |= FLAGS_VALUE_IDENTICAL;
                } else {
                    let offset = lv.wrapping_sub(self.last_long);
                    if self.valid_last_long
                        && offset.abs() < lv.abs()
                        && offset.abs() < (1i64 << 48)
                    {
                        flag |= FLAGS_LONG_DELTA_PREVIOUS | FLAGS_LONG_ZIGZAG;
                    } else if lv.abs() < (1i64 << 48) {
                        flag |= FLAGS_LONG_ZIGZAG;
                    }
                }
            }
            Value::Bool(b) => {
                flag |= FLAGS_TYPE_BOOLEAN;
                flag |= if *b {
                    FLAGS_BOOLEAN_VALUE_TRUE
                } else {
                    FLAGS_BOOLEAN_VALUE_FALSE
                };
            }
            Value::Str(s) => {
                flag |= FLAGS_TYPE_STRING;
                if self.last_str.as_deref() == Some(s.as_str()) {
                    flag |= FLAGS_VALUE_IDENTICAL;
                }
            }
            Value::Bytes(b) => {
                flag |= FLAGS_TYPE_STRING | FLAGS_STRING_BINARY;
                let s = String::from_utf8_lossy(b).into_owned();
                if self.last_str.as_deref() == Some(s.as_str()) {
                    flag |= FLAGS_VALUE_IDENTICAL;
                }
            }
            Value::Double(d) => {
                flag |= FLAGS_TYPE_DOUBLE;
                if self.valid_last_double && (*d == self.last_double || (d.is_nan() && self.last_double.is_nan())) {
                    flag |= FLAGS_VALUE_IDENTICAL;
                } else {
                    flag |= FLAGS_DOUBLE_IEEE754;
                }
            }
        }

        let mut loc_elev: u8 = 0;
        if let Some(loc) = location {
            loc_elev |= FLAGS_LOCATION;
            if self.valid_last_loc && loc == self.last_loc {
                loc_elev |= FLAGS_LOCATION_IDENTICAL;
            } else if self.valid_last_loc && (loc - self.last_loc).abs() < (1i64 << 48) {
                loc_elev |= FLAGS_LOCATION_GEOXPPOINT_ZIGZAG_DELTA;
            }
        }
        if let Some(elev) = elevation {
            loc_elev |= FLAGS_ELEVATION;
            if self.valid_last_elev && elev == self.last_elev {
                loc_elev |= FLAGS_ELEVATION_IDENTICAL;
            } else if self.valid_last_elev && (elev - self.last_elev).abs() < (1i64 << 48) {
                loc_elev |= FLAGS_ELEVATION_ZIGZAG | FLAGS_ELEVATION_DELTA_PREVIOUS;
            } else if elev.abs() < (1i64 << 48) {
                loc_elev |= FLAGS_ELEVATION_ZIGZAG;
            }
        }
        if loc_elev != 0 {
            flag |= FLAGS_CONTINUATION;
        }

        self.out.push(flag);
        if flag & FLAGS_CONTINUATION != 0 {
            self.out.push(loc_elev);
        }

        match flag & FLAGS_MASK_TIMESTAMP {
            FLAGS_TIMESTAMP_RAW_ABSOLUTE => self.out.extend_from_slice(&ts.to_be_bytes()),
            FLAGS_TIMESTAMP_EQUALS_BASE => {}
            FLAGS_TIMESTAMP_ZIGZAG_DELTA_BASE => encode_zigzag(ts - self.base, &mut self.out),
            _ => encode_zigzag(ts - self.last_ts, &mut self.out),
        }
        self.last_ts = ts;
        self.valid_last_ts = true;

        if loc_elev & FLAGS_LOCATION != 0 {
            if loc_elev & FLAGS_LOCATION_IDENTICAL == 0 {
                let loc = location.unwrap();
                if loc_elev & FLAGS_LOCATION_GEOXPPOINT_ZIGZAG_DELTA != 0 {
                    encode_zigzag(loc - self.last_loc, &mut self.out);
                } else {
                    self.out.extend_from_slice(&loc.to_be_bytes());
                }
                self.last_loc = loc;
                self.valid_last_loc = true;
            }
        }
        if loc_elev & FLAGS_ELEVATION != 0 {
            if loc_elev & FLAGS_ELEVATION_IDENTICAL == 0 {
                let elev = elevation.unwrap();
                let toencode = if loc_elev & FLAGS_ELEVATION_DELTA_PREVIOUS != 0 {
                    elev - self.last_elev
                } else {
                    elev
                };
                if loc_elev & FLAGS_ELEVATION_ZIGZAG != 0 {
                    encode_zigzag(toencode, &mut self.out);
                } else {
                    self.out.extend_from_slice(&toencode.to_be_bytes());
                }
                self.last_elev = elev;
                self.valid_last_elev = true;
            }
        }

        match flag & FLAGS_MASK_TYPE {
            FLAGS_TYPE_STRING => {
                if flag & FLAGS_VALUE_IDENTICAL == 0 {
                    match value {
                        Value::Str(s) => {
                            let bytes = s.as_bytes();
                            encode_uvarint(bytes.len() as u64, &mut self.out);
                            self.out.extend_from_slice(bytes);
                            self.last_str = Some(s.clone());
                        }
                        Value::Bytes(b) => {
                            encode_uvarint(b.len() as u64, &mut self.out);
                            self.out.extend_from_slice(b);
                            self.last_str = Some(String::from_utf8_lossy(b).into_owned());
                        }
                        _ => unreachable!(),
                    }
                }
            }
            FLAGS_TYPE_LONG => {
                if flag & FLAGS_VALUE_IDENTICAL == 0 {
                    let lv = match value {
                        Value::Long(l) => *l,
                        _ => unreachable!(),
                    };
                    let toencode = if flag & FLAGS_LONG_DELTA_PREVIOUS != 0 {
                        lv.wrapping_sub(self.last_long)
                    } else {
                        lv
                    };
                    if flag & FLAGS_LONG_ZIGZAG != 0 {
                        encode_zigzag(toencode, &mut self.out);
                    } else {
                        self.out.extend_from_slice(&toencode.to_be_bytes());
                    }
                    self.last_long = lv;
                    self.valid_last_long = true;
                }
            }
            FLAGS_TYPE_DOUBLE => {
                if flag & FLAGS_VALUE_IDENTICAL == 0 {
                    let d = match value {
                        Value::Double(d) => *d,
                        _ => unreachable!(),
                    };
                    self.out.extend_from_slice(&d.to_be_bytes());
                    self.last_double = d;
                    self.valid_last_double = true;
                }
            }
            _ => {}
        }
        self.count += 1;
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_single_long_datum() {
        let mut e = Encoder::new(1000);
        e.add(1000, None, None, &Value::Long(42));
        let bytes = e.into_bytes();
        assert_eq!(bytes[0] & FLAGS_MASK_TIMESTAMP, FLAGS_TIMESTAMP_EQUALS_BASE);
        assert_eq!(bytes[0] & FLAGS_MASK_TYPE, FLAGS_TYPE_LONG);
        assert_eq!(bytes[0] & 0x07, FLAGS_LONG_ZIGZAG);
        assert_eq!(bytes.len(), 1 + 1);
        assert_eq!(bytes[1], 84);
    }

    #[test]
    fn encodes_raw_timestamp_when_far_from_base() {
        let mut e = Encoder::new(0);
        e.add(1i64 << 50, None, None, &Value::Bool(true));
        let bytes = e.into_bytes();
        assert_eq!(bytes[0] & FLAGS_MASK_TIMESTAMP, FLAGS_TIMESTAMP_RAW_ABSOLUTE);
        assert_eq!(bytes[0] & FLAGS_MASK_TYPE, FLAGS_TYPE_BOOLEAN);
        assert_eq!(bytes[0] & 0x07, FLAGS_BOOLEAN_VALUE_TRUE);
        assert_eq!(bytes.len(), 1 + 8);
    }

    #[test]
    fn encodes_string_value() {
        let mut e = Encoder::new(0);
        e.add(1, None, None, &Value::Str("abc".to_string()));
        let bytes = e.into_bytes();
        assert_eq!(bytes[0] & FLAGS_MASK_TYPE, FLAGS_TYPE_STRING);
        assert_eq!(bytes[0] & FLAGS_MASK_TIMESTAMP, FLAGS_TIMESTAMP_ZIGZAG_DELTA_BASE);
        assert_eq!(bytes[1], 2);
        assert_eq!(bytes[2], 3);
        assert_eq!(&bytes[3..], b"abc");
    }

    #[test]
    fn uvarint_and_zigzag() {
        let mut out = Vec::new();
        encode_uvarint(300, &mut out);
        assert_eq!(out, vec![0xac, 0x02]);
        let mut out = Vec::new();
        encode_zigzag(-1, &mut out);
        assert_eq!(out, vec![1]);
        let mut out = Vec::new();
        encode_zigzag(42, &mut out);
        assert_eq!(out, vec![84]);
    }
}
