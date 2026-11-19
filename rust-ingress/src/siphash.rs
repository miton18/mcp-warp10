const C0: u64 = 0x736f6d6570736575;
const C1: u64 = 0x646f72616e646f6d;
const C2: u64 = 0x6c7967656e657261;
const C3: u64 = 0x7465646279746573;

#[inline(always)]
fn sipround(v: &mut [u64; 4]) {
    v[0] = v[0].wrapping_add(v[1]);
    v[2] = v[2].wrapping_add(v[3]);
    v[1] = v[1].rotate_left(13);
    v[3] = v[3].rotate_left(16);
    v[1] ^= v[0];
    v[3] ^= v[2];
    v[0] = v[0].rotate_left(32);
    v[2] = v[2].wrapping_add(v[1]);
    v[0] = v[0].wrapping_add(v[3]);
    v[1] = v[1].rotate_left(17);
    v[3] = v[3].rotate_left(21);
    v[1] ^= v[2];
    v[3] ^= v[0];
    v[2] = v[2].rotate_left(32);
}

#[inline(always)]
fn le64(data: &[u8], offset: usize) -> u64 {
    let mut l = 0u64;
    for i in 0..8 {
        l |= (data[offset + i] as u64) << (8 * i);
    }
    l
}

pub fn get_key(key: &[u8]) -> (u64, u64) {
    let k0 = u64::from_be_bytes(key[0..8].try_into().unwrap());
    let k1 = u64::from_be_bytes(key[8..16].try_into().unwrap());
    (k0, k1)
}

pub fn hash24(k0: u64, k1: u64, data: &[u8]) -> u64 {
    let mut v = [C0 ^ k0, C1 ^ k1, C2 ^ k0, C3 ^ k1];
    let len = data.len();
    let len8 = len - len % 8;
    let mut idx = 0usize;
    while idx < len8 {
        let m = le64(data, idx);
        v[3] ^= m;
        sipround(&mut v);
        sipround(&mut v);
        v[0] ^= m;
        idx += 8;
    }
    let mut m = (len as u64) << 56;
    for (i, b) in data[len8..].iter().enumerate() {
        m |= (*b as u64) << (8 * i);
    }
    v[3] ^= m;
    sipround(&mut v);
    sipround(&mut v);
    v[0] ^= m;
    v[2] ^= 0xff;
    sipround(&mut v);
    sipround(&mut v);
    sipround(&mut v);
    sipround(&mut v);
    v[0] ^ v[1] ^ v[2] ^ v[3]
}

pub fn hash24_palindromic(k0: u64, k1: u64, data: &[u8]) -> u64 {
    if data.is_empty() {
        return hash24(k0, k1, data);
    }
    let mut v = [C0 ^ k0, C1 ^ k1, C2 ^ k0, C3 ^ k1];
    let len = data.len();
    let last = (2 * len) / 8 * 8;
    let len8 = len - len % 8;
    let mut idx = 0usize;
    let offset2 = 2 * len - 1;
    while idx < len8 {
        let m = le64(data, idx);
        v[3] ^= m;
        sipround(&mut v);
        sipround(&mut v);
        v[0] ^= m;
        idx += 8;
    }
    let mut m = 0u64;
    for i in 0..8u32 {
        let pos = if idx >= len { len - 1 - (idx % len) } else { idx };
        m |= (data[pos] as u64) << (8 * i);
        idx += 1;
    }
    v[3] ^= m;
    sipround(&mut v);
    sipround(&mut v);
    v[0] ^= m;
    while idx < last {
        let m = le64(data, offset2 - (idx + 7)).swap_bytes();
        idx += 8;
        v[3] ^= m;
        sipround(&mut v);
        sipround(&mut v);
        v[0] ^= m;
    }
    let mut m = 0u64;
    if idx != 0 {
        let mut i = 0usize;
        let rem = (2 * len).saturating_sub(idx);
        while i < rem {
            m = (m << 8) | data[i] as u64;
            i += 1;
        }
    } else {
        for i in 0..2 * len {
            let pos = if i >= len { len - 1 - (i % len) } else { i };
            m = (m << 8) | data[pos] as u64;
        }
    }
    m |= (2 * len as u64) << 56;
    v[3] ^= m;
    sipround(&mut v);
    sipround(&mut v);
    v[0] ^= m;
    v[2] ^= 0xff;
    sipround(&mut v);
    sipround(&mut v);
    sipround(&mut v);
    sipround(&mut v);
    v[0] ^ v[1] ^ v[2] ^ v[3]
}

#[cfg(test)]
mod tests {
    use super::*;

    const K0: u64 = 0x0706050403020100;
    const K1: u64 = 0x0f0e0d0c0b0a0908;

    const EXPECTED: [u64; 10] = [
        0x726fdb47dd0e0e31,
        0x74f839c593dc67fd,
        0x0d6c8009d9a94f5a,
        0x85676696d7fb7e2d,
        0xcf2794e0277187b7,
        0x18765564cd99a68d,
        0xcbc9466e58fee3ce,
        0xab0200f58b01d137,
        0x93f5f5799a932462,
        0x9e0082df0ba9e4b0,
    ];

    #[test]
    fn hash24_official_vectors() {
        for (i, e) in EXPECTED.iter().enumerate() {
            let msg: Vec<u8> = (0..i as u8).collect();
            assert_eq!(*e, hash24(K0, K1, &msg), "vector {}", i);
        }
    }

    #[test]
    fn palindromic_is_deterministic_and_sweeps_lengths() {
        let msg = b"Too many secrets, Marty!";
        assert_eq!(hash24_palindromic(K0, K1, msg), hash24_palindromic(K0, K1, msg));
        for len in 0..40usize {
            let data: Vec<u8> = (0..len as u8).collect();
            let _ = hash24_palindromic(K0, K1, &data);
        }
        assert_ne!(
            hash24_palindromic(K0, K1, b"warp10"),
            hash24_palindromic(K0, K1, b"warp11")
        );
    }

    #[test]
    fn palindromic_differs_from_plain() {
        let msg = b"warp10";
        assert_ne!(hash24(K0, K1, msg), hash24_palindromic(K0, K1, msg));
    }
}
