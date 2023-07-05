use sha2::{Sha256, Digest};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

pub struct ChallengeResponse {
    secret: String,
    challenge: String
}

impl ChallengeResponse {
    pub fn new(secret: String) -> Self {
        let challenge = thread_rng().sample_iter(&Alphanumeric)
                    .take(64)
                    .map(char::from)
                    .collect();
        Self {
            secret,
            challenge,
        }
    }

    pub fn with_challenge(secret: String, challenge: &str) -> Self {
        Self {
            secret,
            challenge: challenge.to_string(),
        }
    }

    pub fn hash(&self, input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.secret.clone());
        hasher.update(input);
        hex::encode(hasher.finalize())
    }

    pub fn challenge(&self) -> String {
        self.challenge.clone()
    }

    pub fn verify(&self, response: &str) -> bool {
        self.hash(&self.challenge) == response
    }

    pub fn response(&self) -> String {
        self.hash(&self.challenge)
    }
}