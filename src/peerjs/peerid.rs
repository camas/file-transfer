use rand::{thread_rng, Rng};

const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

#[derive(Clone)]
pub struct PeerID {
    base_id: String,
    full_id: String,
}

impl PeerID {
    pub fn new(base_id: String) -> Option<PeerID> {
        if !Self::valid_base(&base_id) {
            return None;
        }

        let full_id = format!("camas-file-transfer-{base_id}");

        Some(PeerID { base_id, full_id })
    }

    pub fn new_random_short_id() -> PeerID {
        let base_id = random_alphabet_string(4);
        PeerID::new(base_id).unwrap()
    }

    pub fn new_random_long_id() -> PeerID {
        let base_id = random_alphabet_string(10);
        PeerID::new(base_id).unwrap()
    }

    pub fn new_short_id(base_id: String) -> Option<PeerID> {
        if base_id.len() != 4 || !Self::valid_base(&base_id) {
            return None;
        }

        PeerID::new(base_id)
    }

    pub fn valid_base(base_id: &str) -> bool {
        base_id.as_bytes().iter().all(|c| ALPHABET.contains(c))
    }

    pub fn base(&self) -> &str {
        &self.base_id
    }

    pub fn full(&self) -> &str {
        &self.full_id
    }
}

fn random_alphabet_string(len: usize) -> String {
    let mut rng = thread_rng();
    (0..len)
        .map(|_| {
            let index = rng.gen_range(0..ALPHABET.len());
            ALPHABET[index] as char
        })
        .collect::<String>()
}
