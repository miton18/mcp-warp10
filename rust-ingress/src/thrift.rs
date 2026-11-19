use std::collections::BTreeMap;

const T_STOP: u8 = 0;
const T_I32: u8 = 8;
const T_I64: u8 = 10;
const T_STRING: u8 = 11;
const T_STRUCT: u8 = 12;
const T_MAP: u8 = 13;

pub const KAFKA_MSG_STORE: i32 = 1;
pub const KAFKA_MSG_ARCHIVE: i32 = 2;
pub const KAFKA_MSG_DELETE: i32 = 3;

pub struct Writer {
    pub buf: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }
    pub fn field(&mut self, ttype: u8, id: i16) {
        self.buf.push(ttype);
        self.buf.extend_from_slice(&id.to_be_bytes());
    }
    pub fn i64_field(&mut self, id: i16, v: i64) {
        self.field(T_I64, id);
        self.buf.extend_from_slice(&v.to_be_bytes());
    }
    pub fn i32_field(&mut self, id: i16, v: i32) {
        self.field(T_I32, id);
        self.buf.extend_from_slice(&v.to_be_bytes());
    }
    pub fn bytes_field(&mut self, id: i16, v: &[u8]) {
        self.field(T_STRING, id);
        self.buf.extend_from_slice(&(v.len() as i32).to_be_bytes());
        self.buf.extend_from_slice(v);
    }
    pub fn str_map_field(&mut self, id: i16, v: &BTreeMap<String, String>) {
        self.field(T_MAP, id);
        self.buf.push(T_STRING);
        self.buf.push(T_STRING);
        self.buf.extend_from_slice(&(v.len() as i32).to_be_bytes());
        for (k, val) in v {
            self.bytes_field_raw(k.as_bytes());
            self.bytes_field_raw(val.as_bytes());
        }
    }
    fn bytes_field_raw(&mut self, v: &[u8]) {
        self.buf.extend_from_slice(&(v.len() as i32).to_be_bytes());
        self.buf.extend_from_slice(v);
    }
    pub fn struct_begin(&mut self, id: i16) {
        self.field(T_STRUCT, id);
    }
    pub fn stop(&mut self) {
        self.buf.push(T_STOP);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    pub class_id: i64,
    pub labels_id: i64,
    pub attributes: Option<BTreeMap<String, String>>,
}

impl Metadata {
    pub fn serialize(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.bytes_field(1, self.name.as_bytes());
        w.str_map_field(2, &self.labels);
        w.i64_field(3, self.class_id);
        w.i64_field(4, self.labels_id);
        if let Some(attrs) = &self.attributes {
            w.str_map_field(5, attrs);
        }
        w.stop();
        w.buf
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KafkaDataMessage {
    pub class_id: i64,
    pub labels_id: i64,
    pub data: Option<Vec<u8>>,
    pub msg_type: i32,
    pub metadata: Option<Metadata>,
    pub attributes: Option<BTreeMap<String, String>>,
}

impl KafkaDataMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.i64_field(1, self.class_id);
        w.i64_field(2, self.labels_id);
        if let Some(data) = &self.data {
            w.bytes_field(3, data);
        }
        w.i32_field(7, self.msg_type);
        if let Some(m) = &self.metadata {
            w.struct_begin(9);
            w.buf.extend_from_slice(&m.serialize());
        }
        if let Some(attrs) = &self.attributes {
            w.str_map_field(10, attrs);
        }
        w.stop();
        w.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_layout() {
        let m = Metadata {
            name: "c".to_string(),
            labels: BTreeMap::from([("a".to_string(), "b".to_string())]),
            class_id: 1,
            labels_id: 2,
            attributes: None,
        };
        let b = m.serialize();
        let mut expected = vec![11, 0, 1, 0, 0, 0, 1, b'c'];
        expected.extend([13, 0, 2, 11, 11, 0, 0, 0, 1]);
        expected.extend([0, 0, 0, 1, b'a']);
        expected.extend([0, 0, 0, 1, b'b']);
        expected.extend([10, 0, 3]);
        expected.extend(1i64.to_be_bytes());
        expected.extend([10, 0, 4]);
        expected.extend(2i64.to_be_bytes());
        expected.push(0);
        assert_eq!(b, expected);
    }

    #[test]
    fn kafka_data_message_layout() {
        let msg = KafkaDataMessage {
            class_id: 0x0102,
            labels_id: 0x0304,
            data: Some(vec![0xaa]),
            msg_type: KAFKA_MSG_STORE,
            metadata: None,
            attributes: None,
        };
        let b = msg.serialize();
        let mut expected = Vec::new();
        expected.extend([10, 0, 1]);
        expected.extend(0x0102i64.to_be_bytes());
        expected.extend([10, 0, 2]);
        expected.extend(0x0304i64.to_be_bytes());
        expected.extend([11, 0, 3, 0, 0, 0, 1, 0xaa]);
        expected.extend([8, 0, 7, 0, 0, 0, 1]);
        expected.push(0);
        assert_eq!(b, expected);
    }
}
