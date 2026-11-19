use std::collections::BTreeMap;

use crate::siphash::hash24_palindromic;

pub fn class_id(key: (u64, u64), name: &str) -> i64 {
    hash24_palindromic(key.0, key.1, name.as_bytes()) as i64
}

pub fn labels_id(key: (u64, u64), labels: &BTreeMap<String, String>) -> i64 {
    let mut pairs: Vec<(u64, u64)> = labels
        .iter()
        .map(|(k, v)| {
            (
                hash24_palindromic(key.0, key.1, k.as_bytes()),
                hash24_palindromic(key.0, key.1, v.as_bytes()),
            )
        })
        .collect();
    pairs.sort();
    let mut buf = Vec::with_capacity(pairs.len() * 16);
    for (nh, vh) in pairs {
        buf.extend_from_slice(&nh.to_be_bytes());
        buf.extend_from_slice(&vh.to_be_bytes());
    }
    hash24_palindromic(key.0, key.1, &buf) as i64
}

pub fn gts_ids_key(class_id: i64, labels_id: i64) -> [u8; 16] {
    let mut out = [0u8; 16];
    out[0..8].copy_from_slice(&class_id.to_be_bytes());
    out[8..16].copy_from_slice(&labels_id.to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_id_is_order_independent() {
        let key = (0x0706050403020100, 0x0f0e0d0c0b0a0908);
        let mut a = BTreeMap::new();
        a.insert("b".to_string(), "2".to_string());
        a.insert("a".to_string(), "1".to_string());
        let mut b = BTreeMap::new();
        b.insert("a".to_string(), "1".to_string());
        b.insert("b".to_string(), "2".to_string());
        assert_eq!(labels_id(key, &a), labels_id(key, &b));
    }

    #[test]
    fn labels_id_changes_with_labels() {
        let key = (1, 2);
        let mut a = BTreeMap::new();
        a.insert("a".to_string(), "1".to_string());
        let mut b = BTreeMap::new();
        b.insert("a".to_string(), "2".to_string());
        assert_ne!(labels_id(key, &a), labels_id(key, &b));
    }

    #[test]
    fn gts_ids_key_is_big_endian() {
        let k = gts_ids_key(0x0102030405060708, 0x1112131415161718);
        assert_eq!(&k[0..8], &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(&k[8..16], &[0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18]);
    }
}
